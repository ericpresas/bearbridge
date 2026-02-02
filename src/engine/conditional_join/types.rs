use std::str::FromStr;
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3::exceptions::PyKeyError;
use crate::errors::BearBridgeError;

#[derive(Debug, Clone)]
pub enum Condition {
    Standard {
        left_col: String,
        right_col: String,
        op: String
    },
    Difference {
        left_col: String,
        right_col: String,
        op: String,
        thr: f64
    },
    TimeDifference {
        left_col: String,
        right_col: String,
        op: String,
        thr: f64,
        time_unit: String
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimeUnit {
    Days,
    Hours,
    Minutes,
    Seconds,
    Milliseconds,
}

pub enum JoinOptions {
    Standard,
    Difference {
        thr: f64
    },
    TimeDifference {
        thr: f64,
        time_unit: TimeUnit
    }
}

impl Condition {
    pub fn options(&self) -> Result<JoinOptions, BearBridgeError> {
        match self {
            Self::Difference {thr, ..} => {
                let options = JoinOptions::Difference {thr: *thr, };
                Ok(options)
            },
            Self::TimeDifference {thr, time_unit, ..} => {
                let options = JoinOptions::TimeDifference {
                    thr: *thr,
                    time_unit: TimeUnit::from_str(time_unit.as_str())?
                };
                Ok(options)
            },
            _ => {
                Ok(JoinOptions::Standard)
            }
        }
    }

    pub fn cols(&self) -> (&str, &str) {
        match self {
            Self::Standard { left_col, right_col, .. } => (left_col, right_col),
            Self::Difference { left_col, right_col, .. } => (left_col, right_col),
            Self::TimeDifference { left_col, right_col, .. } => (left_col, right_col)
        }
    }

    pub fn operator(&self) -> &str {
        match self {
            Self::Standard { op, ..} => op,
            Self::Difference { op, ..} => op,
            Self::TimeDifference { op, ..} => op,
        }
    }
}

impl<'a> FromPyObject<'a> for Condition {
    fn extract_bound(ob: &Bound<'a, PyAny>) -> PyResult<Self> {
        let dict = ob.downcast::<pyo3::types::PyDict>()?;
        // Required columns
        let left_col: String = dict.get_item("left_col")?.ok_or_else(|| {
            PyErr::new::<PyKeyError, _>("missing 'left_col'")
        })?.extract()?;
        let right_col: String = dict.get_item("right_col")?.ok_or_else(|| {
            PyErr::new::<PyKeyError, _>("missing 'right_col'")
        })?.extract()?;

        let op: String = dict.get_item("op")?.ok_or_else(|| {
            PyErr::new::<PyKeyError, _>("missing 'op'")
        })?.extract()?;

        // Optional Columns
        let thr: Option<f64> = dict.get_item("thr")?.map(|i| i.extract()).transpose()?;
        let time_unit: Option<String> = dict.get_item("time_unit")?.map(|i| i.extract()).transpose()?;

        match (thr, time_unit) {
            (Some(t), Some(u)) => Ok(Condition::TimeDifference { left_col, right_col, op, thr: t, time_unit: u }),
            (Some(t), None) => Ok(Condition::Difference { left_col, right_col, op, thr: t }),
            _ => Ok(Condition::Standard { left_col, right_col, op }),
        }
    }
}

impl FromStr for TimeUnit {
    type Err = BearBridgeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "days" | "d" => Ok(Self::Days),
            "hours" | "h" => Ok(Self::Hours),
            "minutes" | "m" => Ok(Self::Minutes),
            "seconds" | "s" => Ok(Self::Seconds),
            "milliseconds" | "ms" => Ok(Self::Milliseconds),
            _ => Err(Self::Err::InvalidTimeUnit(s.to_string()))
        }
    }
}

impl TimeUnit {
    pub fn to_nanos(&self) -> i64 {
        match self {
            Self::Milliseconds => 1_000_000,
            Self::Seconds      => 1_000_000_000,
            Self::Minutes      => 60 * 1_000_000_000,
            Self::Hours        => 60 * 60 * 1_000_000_000,
            Self::Days         => 24 * 60 * 60 * 1_000_000_000,
        }
    }
}


pub fn str_to_join_type(s: &str) -> Result<JoinType, BearBridgeError> {
    let jt = match s.to_lowercase().as_str() {
        "inner" => JoinType::Inner,
        "left"  => JoinType::Left,
        "full" | "outer" => JoinType::Full,
        _ => return Err(BearBridgeError::InvalidJoinType(s.to_string())),
    };
    Ok(jt)
}