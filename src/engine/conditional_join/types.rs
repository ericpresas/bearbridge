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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use polars::prelude::JoinType;
    #[test]
    fn test_condition_cols_and_operators() {
        // Test Standard variant
        let c1 = Condition::Standard {
            left_col: "user_id".to_string(),
            right_col: "id".to_string(),
            op: "==".to_string(),
        };
        assert_eq!(c1.cols(), ("user_id", "id"));
        assert_eq!(c1.operator(), "==");

        // Test Difference variant
        let c2 = Condition::Difference {
            left_col: "price".to_string(),
            right_col: "cost".to_string(),
            op: ">".to_string(),
            thr: 10.5,
        };
        assert_eq!(c2.cols(), ("price", "cost"));
        assert_eq!(c2.operator(), ">");
    }

    #[test]
    fn test_condition_options_conversion() {
        // 1. Test Standard -> Standard
        let c_std = Condition::Standard {
            left_col: "a".into(),
            right_col: "b".into(),
            op: "==".into(),
        };
        match c_std.options().unwrap() {
            JoinOptions::Standard => (),
            _ => panic!("Expected JoinOptions::Standard"),
        }

        // 2. Test TimeDifference -> TimeDifference with TimeUnit parsing
        let c_time = Condition::TimeDifference {
            left_col: "t1".into(),
            right_col: "t2".into(),
            op: "<".into(),
            thr: 5.0,
            time_unit: "s".into(), // Testing string "s" parses to Seconds
        };

        let opts = c_time.options().unwrap();
        if let JoinOptions::TimeDifference { thr, time_unit } = opts {
            assert_eq!(thr, 5.0);
            assert_eq!(time_unit, TimeUnit::Seconds);
        } else {
            panic!("Expected JoinOptions::TimeDifference");
        }
    }

    #[test]
    fn test_invalid_time_unit_fails() {
        let c_bad = Condition::TimeDifference {
            left_col: "t1".into(),
            right_col: "t2".into(),
            op: "<".into(),
            thr: 5.0,
            time_unit: "invalid_unit".into(),
        };

        // This should return a BearBridgeError because of TimeUnit::from_str
        assert!(c_bad.options().is_err());
    }

    #[test]
    fn test_time_unit_from_str() {
        // Test abbreviations
        assert_eq!(TimeUnit::from_str("d").unwrap(), TimeUnit::Days);
        assert_eq!(TimeUnit::from_str("h").unwrap(), TimeUnit::Hours);
        assert_eq!(TimeUnit::from_str("m").unwrap(), TimeUnit::Minutes);
        assert_eq!(TimeUnit::from_str("s").unwrap(), TimeUnit::Seconds);
        assert_eq!(TimeUnit::from_str("ms").unwrap(), TimeUnit::Milliseconds);

        // Test full names and case insensitivity
        assert_eq!(TimeUnit::from_str("DAYS").unwrap(), TimeUnit::Days);
        assert_eq!(TimeUnit::from_str("Minutes").unwrap(), TimeUnit::Minutes);
        assert_eq!(TimeUnit::from_str("milliseconds").unwrap(), TimeUnit::Milliseconds);

        // Test invalid unit
        let result = TimeUnit::from_str("weeks");
        assert!(result.is_err());
        if let Err(BearBridgeError::InvalidTimeUnit(msg)) = result {
            assert_eq!(msg, "weeks");
        } else {
            panic!("Expected InvalidTimeUnit error");
        }
    }

    #[test]
    fn test_time_unit_to_nanos() {
        // Base units
        assert_eq!(TimeUnit::Milliseconds.to_nanos(), 1_000_000);
        assert_eq!(TimeUnit::Seconds.to_nanos(), 1_000_000_000);

        // Calculated units
        assert_eq!(TimeUnit::Minutes.to_nanos(), 60_000_000_000);
        assert_eq!(TimeUnit::Hours.to_nanos(), 3_600_000_000_000);

        // Large unit (Days)
        // 24 * 60 * 60 * 10^9 = 86,400,000,000,000
        assert_eq!(TimeUnit::Days.to_nanos(), 86_400_000_000_000);
    }

    #[test]
    fn test_str_to_join_type_mappings() {
        // Test standard mappings
        assert_eq!(str_to_join_type("inner").unwrap(), JoinType::Inner);
        assert_eq!(str_to_join_type("left").unwrap(), JoinType::Left);

        // Test aliases for Full join
        assert_eq!(str_to_join_type("full").unwrap(), JoinType::Full);
        assert_eq!(str_to_join_type("outer").unwrap(), JoinType::Full);

        // Test case insensitivity
        assert_eq!(str_to_join_type("INNER").unwrap(), JoinType::Inner);
        assert_eq!(str_to_join_type("OuTeR").unwrap(), JoinType::Full);
    }

    #[test]
    fn test_str_to_join_type_error() {
        // Test invalid join type string
        let result = str_to_join_type("semi");
        assert!(result.is_err());

        if let Err(BearBridgeError::InvalidJoinType(msg)) = result {
            assert_eq!(msg, "semi");
        } else {
            panic!("Expected InvalidJoinType error");
        }
    }
}