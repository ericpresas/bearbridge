use std::sync::Arc;
use arrow::array::RecordBatch;
use polars::prelude::*;
use polars::datatypes::{CompatLevel, PlSmallStr};
use polars::frame::DataFrame;
use polars::prelude::{Column, Series};
use polars_arrow::ffi;
use pyo3::{PyErr, PyResult};
use arrow::ffi::{FFI_ArrowArray as ArrowArray, FFI_ArrowSchema as ArrowSchema};

pub fn bridge_to_polars(batches: Vec<RecordBatch>) -> PyResult<DataFrame> {
    if batches.is_empty() {
        return Ok(DataFrame::empty());
    }

    let schema = batches[0].schema();
    let mut columns = Vec::with_capacity(schema.fields().len());

    for (i, field) in schema.fields().iter().enumerate() {
        let mut chunks = Vec::with_capacity(batches.len());
        for batch in batches.iter() {
            let arrow_col = batch.column(i).clone();

            // 1. Export from arrow-rs using the new API
            let out_schema = ArrowSchema::try_from(arrow_col.data_type())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let out_array = ArrowArray::new(&arrow_col.to_data());

            unsafe {
                // 2. Step A: Convert Schema to DataType
                // We cast our local pointer to a polars_arrow pointer, then dereference it to get &ArrowSchema
                let polars_ffi_schema = &*(&out_schema as *const _ as *const ffi::ArrowSchema);

                let field = ffi::import_field_from_c(polars_ffi_schema)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                // 3. Step B: Import Array
                // Similarly, import_array_from_c likely wants &ArrowArray or a read value
                let polars_ffi_array = std::ptr::read(&out_array as *const _ as *const ffi::ArrowArray);

                let imported = ffi::import_array_from_c(
                    polars_ffi_array,
                    field.dtype,
                ).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                std::mem::forget(out_array);
                std::mem::forget(out_schema);
                chunks.push(imported);
            }
        }
        let name = PlSmallStr::from_str(field.name());
        let series = Series::from_arrow_chunks(name, chunks)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        columns.push(Column::from(series));
    }
    DataFrame::new(columns)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}





pub fn bridge_to_arrow_rs(df: DataFrame) -> PyResult<Vec<RecordBatch>> {
    if df.height() == 0 {
        return Ok(vec![]);
    }

    // 1. Ensure the DataFrame has a single chunk for simplicity in FFI
    let mut df = df;
    df.align_chunks();
    let columns = df.get_columns();
    let num_chunks = if !columns.is_empty() {
        df.max_n_chunks()
    } else {
        0
    };

    let polars_schema = df.schema().to_arrow(CompatLevel::newest());


    let mut batches = Vec::with_capacity(num_chunks);
    for chunk_idx in 0..num_chunks {
        let mut arrow_columns = Vec::with_capacity(df.width());

        for (i, col) in df.iter().enumerate() {
            // 2. Convert Polars Column to polars-arrow Array (Chunk 0)
            let p_array = col.to_arrow(chunk_idx, CompatLevel::newest());
            let p_field = &polars_schema.get_at_index(i).unwrap().1;

            // 3. Create uninitialized C structs for the local arrow-rs crate
            let mut p_ffi_array = ffi::ArrowArray::empty();
            let mut p_ffi_field = ffi::ArrowSchema::empty();

            unsafe {
                // 4. Export from polars-arrow into C pointers
                // We cast our local pointers to what polars-arrow FFI expects
                p_ffi_array = ffi::export_array_to_c(p_array);
                p_ffi_field = ffi::export_field_to_c(p_field);

                let out_array = std::ptr::read(&p_ffi_array as *const _ as *const arrow::ffi::FFI_ArrowArray);
                let out_schema = std::ptr::read(&p_ffi_field as *const _ as *const arrow::ffi::FFI_ArrowSchema);

                // 3. Prevent the original polars_arrow structs from calling their drop/release
                // Polars 0.52.0 FFI structs often handle release in their Drop impl.
                // We want the arrow-rs version to own the release now.
                std::mem::forget(p_ffi_array);
                std::mem::forget(p_ffi_field);

                // 4. Import into arrow-rs
                let imported_data = arrow::ffi::from_ffi(out_array, &out_schema)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                arrow_columns.push(arrow::array::make_array(imported_data));

            }
        }

        // 6. Finalize the arrow-rs RecordBatch
        let arrow_fields: Vec<arrow::datatypes::Field> = arrow_columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let name = df.get_column_names()[i].as_str();
                arrow::datatypes::Field::new(name, col.data_type().clone(), true)
            })
            .collect();

        let arrow_schema = Arc::new(arrow::datatypes::Schema::new(arrow_fields));

        let batch = RecordBatch::try_new(arrow_schema, arrow_columns)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        batches.push(batch);
    }

    Ok(batches)
}


#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, StringArray, Float64Array, BooleanArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn test_ffi_bridge_integrity() {
        // 1. Create arrow-rs data
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::Int32, true),
            Field::new("b", DataType::Utf8, true),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![Some(1), None, Some(3)])),
                Arc::new(StringArray::from(vec![Some("foo"), Some("bar"), None])),
            ],
        ).unwrap();

        // 2. Run the bridge
        let batches = vec![batch];
        let result = bridge_to_polars(batches);
        // 3. Assertions
        assert!(result.is_ok(), "Bridge failed: {:?}", result.err());
        let df = result.unwrap();

        assert_eq!(df.width(), 2);
        assert_eq!(df.height(), 3);

        // Verify types and names
        assert_eq!(df.get_column_names()[0].as_str(), "a");
        assert_eq!(df.get_column_names()[1].as_str(), "b");

        // Verify values
        let col_a = df.column("a").unwrap().as_series().unwrap().i32().unwrap();
        assert_eq!(col_a.get(0), Some(1));
        assert_eq!(col_a.get(1), None);
        assert_eq!(col_a.get(2), Some(3));

        println!("Successfully bridged DataFrame:\n{}", df);
    }

    #[test]
    fn test_bridge_to_arrow_rs_integrity() {
        // 1. Setup: Create a Polars DataFrame
        let df = df!(
            "ints" => &[1, 2, 3],
            "strs" => &["apple", "banana", "cherry"]
        ).unwrap();

        // 2. Execution: Run the bridge
        let result = bridge_to_arrow_rs(df);

        // 3. Validation
        assert!(result.is_ok(), "Bridge returned an error: {:?}", result.err());
        let batches = result.unwrap();

        assert_eq!(batches.len(), 1, "Should have returned exactly 1 batch");
        let batch = &batches[0];

        println!("✅ Bridge test passed! Resulting batch:\n{:?}", batch);


        // Check Schema
        assert_eq!(batch.num_columns(), 2);
        assert_eq!(batch.schema().field(0).name(), "ints");
        assert_eq!(batch.schema().field(1).name(), "strs");

        // Check Data Integrity
        // 1. Int32 is usually a PrimitiveArray<Int32Type>
        // In arrow-rs, Int32Array is a type alias for this, so this should still work:
        let int_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::Int32Array>()
            .expect("Column 0 should be Int32");

        assert_eq!(int_col.value(0), 1);
        assert_eq!(int_col.value(1), 2);

        // 2. StringView requires downcasting to StringViewArray
        // This matches the 'Utf8View' you see in your debug output
        let str_col = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::StringViewArray>()
            .expect("Column 1 should be StringView");

        assert_eq!(str_col.value(0), "apple");
        assert_eq!(str_col.value(1), "banana");

        println!("✅ Bridge test passed! Resulting batch:\n{:?}", batch);
    }

    #[test]
    fn test_empty_input() {
        // Test that an empty vector of batches returns an empty DataFrame
        let batches = vec![];
        let result = bridge_to_polars(batches);
        assert!(result.is_ok());
        let df = result.unwrap();
        assert!(df.is_empty());

        // Test that an empty DataFrame returns an empty vector of batches
        let result_back = bridge_to_arrow_rs(df);
        assert!(result_back.is_ok());
        let batches_back = result_back.unwrap();
        assert!(batches_back.is_empty());
    }

    #[test]
    fn test_multiple_chunks_concatenation() {
        // Create two batches with the same schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("val", DataType::Int32, true),
        ]));

        let batch1 = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![1, 2]))],
        ).unwrap();

        let batch2 = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![3, 4]))],
        ).unwrap();

        // Bridge to Polars
        let df = bridge_to_polars(vec![batch1, batch2]).unwrap();

        // Should have 4 rows total
        assert_eq!(df.height(), 4);

        // Check values
        let col = df.column("val").unwrap().i32().unwrap();
        assert_eq!(col.get(0), Some(1));
        assert_eq!(col.get(1), Some(2));
        assert_eq!(col.get(2), Some(3));
        assert_eq!(col.get(3), Some(4));
    }

    #[test]
    fn test_various_types_round_trip() {
        // Test Float64 and Boolean
        let schema = Arc::new(Schema::new(vec![
            Field::new("floats", DataType::Float64, true),
            Field::new("bools", DataType::Boolean, true),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Float64Array::from(vec![1.1, 2.2, 3.3])),
                Arc::new(BooleanArray::from(vec![true, false, true])),
            ],
        ).unwrap();

        // Arrow -> Polars
        let df = bridge_to_polars(vec![batch.clone()]).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.column("floats").unwrap().dtype(), &polars::prelude::DataType::Float64);
        assert_eq!(df.column("bools").unwrap().dtype(), &polars::prelude::DataType::Boolean);

        // Polars -> Arrow
        let batches_back = bridge_to_arrow_rs(df).unwrap();
        assert_eq!(batches_back.len(), 1);

        let batch_back = &batches_back[0];

        // Verify Floats
        let floats_back = batch_back.column(0).as_any().downcast_ref::<Float64Array>().unwrap();
        assert_eq!(floats_back.value(0), 1.1);
        assert_eq!(floats_back.value(1), 2.2);

        // Verify Bools
        let bools_back = batch_back.column(1).as_any().downcast_ref::<BooleanArray>().unwrap();
        assert_eq!(bools_back.value(0), true);
        assert_eq!(bools_back.value(1), false);
    }
}