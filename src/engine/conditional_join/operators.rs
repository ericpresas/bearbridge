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
            Self::Fuzzy => unimplemented!(),
            Self::Weekday => unimplemented!(),
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

