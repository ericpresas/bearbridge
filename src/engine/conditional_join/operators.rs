use crate::errors::BearBridgeError;
use std::str::FromStr;
use polars::prelude::*;
use crate::engine::conditional_join::types::{Condition, JoinOptions};

#[derive(Debug, Clone, Copy)]
pub enum Operator {
    // Primitive Operators
    Gt,
    Lt,
    GtEq,
    LtEq,
    Eq,
    NotEq,
    // Custom operators
    Weekday,
    Fuzzy,
}
impl FromStr for Operator {
    type Err = BearBridgeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" => Ok(Operator::Gt),
            "<" => Ok(Operator::Lt),
            ">=" => Ok(Operator::GtEq),
            "<=" => Ok(Operator::LtEq),
            "==" => Ok(Operator::Eq),
            "!=" => Ok(Operator::NotEq),
            "weekday" => Ok(Operator::Weekday),
            "fuzzy" => Ok(Operator::Fuzzy),
            _ => Err(BearBridgeError::InvalidOperator(s.to_string()))
        }
    }
}

impl Operator {
    fn pl_expr(&self, lhs: Expr, rhs: Expr) -> Expr {
        match self {
            Self::Gt => lhs.gt(rhs),
            Self::Lt => lhs.lt(rhs),
            Self::GtEq => lhs.gt_eq(rhs),
            Self::LtEq => lhs.lt_eq(rhs),
            Self::Eq => lhs.eq(rhs),
            Self::NotEq => lhs.neq(rhs),
            Self::Fuzzy => unimplemented!("Fuzzy operator not yet implemented"),
            Self::Weekday => unimplemented!("Weekday operator not yet implemented"),
        }
    }
}

pub fn get_predicate(condition: &Condition) -> Result<Expr, BearBridgeError> {
    let (left_col, right_col) = condition.cols();
    let options = condition.options()?;
    let operator = Operator::from_str(condition.operator())?;

    // Extract left hand side of condition expression
    let lhs = match options {
        JoinOptions::Standard => col(left_col),
        JoinOptions::Difference { .. } => {
            (col(left_col) - col(right_col)).abs()
        },
        JoinOptions::TimeDifference { .. } => {
            (col(right_col) - col(left_col)).abs().cast(DataType::Int64)
        }
    };

    // Extract right hand side of condition expression
    let rhs = match options {
        JoinOptions::Standard => col(right_col),
        JoinOptions::Difference { thr, .. } => lit(thr),
        JoinOptions::TimeDifference { thr, time_unit, .. } => {
            let nanos_val = (thr as i64) * time_unit.to_nanos();
            lit(nanos_val)
        }
    };

    // Build operator expression
    Ok(operator.pl_expr(lhs, rhs))

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_operator_from_str() {
        assert!(matches!(Operator::from_str(">").unwrap(), Operator::Gt));
        assert!(matches!(Operator::from_str("<").unwrap(), Operator::Lt));
        assert!(matches!(Operator::from_str(">=").unwrap(), Operator::GtEq));
        assert!(matches!(Operator::from_str("<=").unwrap(), Operator::LtEq));
        assert!(matches!(Operator::from_str("==").unwrap(), Operator::Eq));
        assert!(matches!(Operator::from_str("!=").unwrap(), Operator::NotEq));
        assert!(matches!(Operator::from_str("weekday").unwrap(), Operator::Weekday));
        assert!(matches!(Operator::from_str("fuzzy").unwrap(), Operator::Fuzzy));

        assert!(Operator::from_str("unknown").is_err());
    }

    #[test]
    fn test_pl_expr_generation() {
        let lhs = col("left");
        let rhs = col("right");

        // Test Greater Than
        let expr = Operator::Gt.pl_expr(lhs.clone(), rhs.clone());
        // We verify by checking the debug string representation
        assert_eq!(format!("{:?}", expr), "[(col(\"left\")) > (col(\"right\"))]");

        // Test Equality
        let expr_eq = Operator::Eq.pl_expr(lhs.clone(), rhs.clone());
        assert_eq!(format!("{:?}", expr_eq), "[(col(\"left\")) == (col(\"right\"))]");

        // Test Not Equal
        let expr_neq = Operator::NotEq.pl_expr(lhs.clone(), rhs.clone());
        assert_eq!(format!("{:?}", expr_neq), "[(col(\"left\")) != (col(\"right\"))]");
    }

    #[test]
    #[should_panic(expected = "Fuzzy operator not yet implemented")]
    fn test_fuzzy_operator_unimplemented() {
        let lhs = col("left");
        let rhs = col("right");
        Operator::Fuzzy.pl_expr(lhs, rhs);
    }

    #[test]
    #[should_panic(expected = "Weekday operator not yet implemented")]
    fn test_weekday_operator_unimplemented() {
        let lhs = col("left");
        let rhs = col("right");
        Operator::Weekday.pl_expr(lhs, rhs);
    }

    #[test]
    fn test_operator_execution() -> PolarsResult<()> {
        // Create a small dummy DF to verify the logic actually runs
        let df = df!(
            "left" => [10, 20, 30],
            "right" => [5, 25, 30]
        )?;

        let op = Operator::Gt;
        let expr = op.pl_expr(col("left"), col("right"));

        let res = df.lazy()
            .select([expr.alias("result")])
            .collect()?;

        let series = res.column("result")?.bool()?;
        assert_eq!(series.get(0), Some(true));  // 10 > 5
        assert_eq!(series.get(1), Some(false)); // 20 > 25
        assert_eq!(series.get(2), Some(false)); // 30 > 30

        Ok(())
    }
    #[test]
    fn test_strict_dsl_standard() {
        let cond = Condition::Standard {
            left_col: "id_l".into(),
            right_col: "id_r".into(),
            op: "==".into(),
        };
        let expr = get_predicate(&cond).unwrap();

        // Exact match for standard equality
        assert_eq!(format!("{:?}", expr), "[(col(\"id_l\")) == (col(\"id_r\"))]");
    }

    #[test]
    fn test_strict_dsl_difference() {
        let cond = Condition::Difference {
            left_col: "val_a".into(),
            right_col: "val_b".into(),
            op: ">=".into(),
            thr: 100.0,
        };
        let expr = get_predicate(&cond).unwrap();

        // Note the parentheses created by Polars for the math operations
        assert_eq!(
            format!("{:?}", expr),
            "[([(col(\"val_a\")) - (col(\"val_b\"))].abs()) >= (dyn float: 100)]"
        );
    }

    #[test]
    fn test_strict_dsl_time_difference() {
        let cond = Condition::TimeDifference {
            left_col: "ts_l".into(),
            right_col: "ts_r".into(),
            op: "<".into(),
            thr: 2.0,
            time_unit: "s".into(),
        };
        let expr = get_predicate(&cond).unwrap();

        // This is the most complex one.
        let expected = "[([(col(\"ts_r\")) - (col(\"ts_l\"))].abs().cast(Int64)) < (dyn int: 2000000000)]";

        assert_eq!(format!("{:?}", expr), expected);
    }

    #[test]
    fn test_strict_dsl_not_equal() {
        let cond = Condition::Standard {
            left_col: "a".into(),
            right_col: "b".into(),
            op: "!=".into(),
        };
        let expr = get_predicate(&cond).unwrap();

        // Polars renders NotEq as !=
        assert_eq!(format!("{:?}", expr), "[(col(\"a\")) != (col(\"b\"))]");
    }
}
