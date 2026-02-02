mod errors;
mod engine;
mod bridge;

use pyo3::prelude::*;

#[pymodule]
fn bear_bridge_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bridge::py_conditional_join, m)?)?;
    Ok(())
}
