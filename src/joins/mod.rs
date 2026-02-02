pub mod conditional;

pub mod python_bridge {
    use super::conditional::conditional_join;
    use pyo3::prelude::*;
    use pyo3_polars::PyDataFrame;
    use polars::prelude::*;

    #[pyfunction]
    #[pyo3(name = "conditional_join")]
    pub fn py_conditional_join(
        left: PyDataFrame,
        right: PyDataFrame,
        conditions: Vec<(String, String, String)>,
    ) -> PyResult<PyDataFrame> {
        // 1. Unwrap the Python DataFrames
        let l_df: DataFrame = left.into();
        let r_df: DataFrame = right.into();

        // 2. Call the logic function defined in conditional.rs
        // The '?' handles BearBridgeError -> PyErr conversion automatically
        let result_df = conditional_join(&l_df, &r_df, &conditions)?;

        // 3. Wrap back for Python
        Ok(PyDataFrame(result_df))
    }
}