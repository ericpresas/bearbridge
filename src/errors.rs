use serde::Serialize;
use polars::prelude::PolarsError;
use pyo3::exceptions::{PyValueError, PyRuntimeError};
use pyo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "error_type", content = "details")]
pub enum BearBridgeError {
    #[error("Invalid operator {0}. Allowed: >, <, >=, <=, ==, != ")]
    InvalidOperator(String),
    #[error("Column {column} not found in {side}.")]
    ColumnNotFound { column: String, side: String },
    #[error("TimeUnit {0} is invalid. Allowed: d|days, h|hours, m|minutes, s|seconds, ms|milliseconds")]
    InvalidTimeUnit(String),
    #[error("Join type {0} not available. Allowed inner, left, full|outer.")]
    InvalidJoinType(String),
    #[error("Polars engine error: {0}")]
    #[serde(skip_serializing)]
    Polars(#[from] PolarsError),
    #[error("Validation filed with {0} errors:\n{1}")]
    #[serde(skip_serializing)]
    MultiError(usize, String),
}


impl From<BearBridgeError> for PyErr {
    fn from(error: BearBridgeError) -> PyErr {
        match error {
            BearBridgeError::MultiError(_, json_str) => PyValueError::new_err(json_str),
            _ => PyRuntimeError::new_err(error.to_string()),
        }
    }
}