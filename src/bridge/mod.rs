mod conversions;

use pyo3::prelude::*;
use arrow::pyarrow::ToPyArrow;
use arrow::pyarrow::PyArrowType;
use arrow::record_batch::RecordBatch;
use crate::engine::conditional_join::{execute_join, Condition, JoinIterator};
use conversions::{bridge_to_polars, bridge_to_arrow_rs};



#[pyfunction (name = "conditional_join")]
#[pyo3(
    signature = (left, right, conditions, how="inner"),
    text_signature = "(left: pyarrow.Table, right: pyarrow.Table, conditions: list[dict], how: str = 'inner') -> pyarrow.Table"
)]
pub fn py_conditional_join<'py>(
    py: Python<'py>,
    left: PyArrowType<Vec<RecordBatch>>,
    right: PyArrowType<Vec<RecordBatch>>,
    conditions: Vec<Condition>, // PyO3 will use your Condition's FromPyObject impl
    how: &str,
) -> PyResult<Bound<'py, PyAny>> {

    let l_batches = left.0;
    let r_batches = right.0;

    // 2. Use your awesome bridge to get Polars DataFrames (Zero-Copy)
    let left_df = bridge_to_polars(l_batches)?;
    let right_df = bridge_to_polars(r_batches)?;

    // 3. Execute the Join using your internal engine
    let joined_df = execute_join(&left_df, &right_df, &conditions, how)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))?;

    // 4. Convert back to Arrow for the Python return
    let result_batches = bridge_to_arrow_rs(joined_df)?;

    // 5. Hand the results back to Python as a pyarrow.Table
    result_batches.to_pyarrow(py)
}

#[pyclass(name = "JoinStream")]
pub struct PyJoinStream {
    iterator: JoinIterator,
}

#[pymethods]
impl PyJoinStream {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        // This calls your while-loop iterator logic
        match slf.iterator.next() {
            Some(Ok(df)) => {
                // Convert Polars DF to Arrow RecordBatches
                let result_batches = bridge_to_arrow_rs(df)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))?;

                // Return as pyarrow.Table for this specific chunk
                Ok(Some(result_batches.to_pyarrow(py)?))
            },
            Some(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e))),
            None => Ok(None), // End of stream
        }
    }
}

#[pyfunction(name = "conditional_join_stream")]
#[pyo3(
    signature = (left, right, conditions, batch_size=100000, how="inner"),
    text_signature = "(left: pyarrow.Table, right: pyarrow.Table, conditions: list[dict], batch_size: int = 100000, how: str = 'inner') -> JoinStream"
)]
pub fn py_conditional_join_stream<'py>(
    _py: Python<'py>,
    left: PyArrowType<Vec<RecordBatch>>,
    right: PyArrowType<Vec<RecordBatch>>,
    conditions: Vec<Condition>,
    batch_size: usize,
    how: &str,
) -> PyResult<PyJoinStream> {
    // 1. Convert Arrow to Polars (Zero-Copy)
    let left_df = bridge_to_polars(left.0)?;
    let right_df = bridge_to_polars(right.0)?;

    // 2. Initialize the Iterator
    let iterator = JoinIterator {
        left: left_df,
        right: right_df,
        conditions,
        how: how.to_string(),
        current_chunk: 0,
        chunk_size: batch_size,
    };

    // 3. Wrap in our pyclass and return
    Ok(PyJoinStream { iterator })
}