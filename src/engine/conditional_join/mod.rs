mod types;
mod operators;
#[cfg(test)]
mod tests;

use polars::prelude::*;

pub use types::{Condition, str_to_join_type};
use operators::{get_predicate};

use crate::errors::BearBridgeError;


pub fn execute_join(
    left: DataFrame,
    right: DataFrame,
    conditions: Vec<Condition>,
    how: &str,
) -> Result<DataFrame, BearBridgeError> {
    // We treat this like an "internal" from_str
    let join_type = str_to_join_type(how)?;

    let predicates = conditions.iter()
        .map(get_predicate)
        .collect::<Result<Vec<_>, _>>()?;

    println!("{:?}", predicates);

    Ok(left.clone().lazy()
        .join_builder()
        .with(right.clone().lazy())
        .how(join_type)
        .join_where(predicates)
        .collect()?)
}
