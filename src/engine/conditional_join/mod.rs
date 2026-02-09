mod types;
mod operators;
#[cfg(test)]
mod tests;

use polars::prelude::*;

pub use types::{Condition, str_to_join_type};
use operators::{get_predicate};

use crate::errors::BearBridgeError;


pub fn execute_join(
    left: &DataFrame,
    right: &DataFrame,
    conditions: &[Condition],
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


pub struct JoinIterator {
    pub left: DataFrame,
    pub right: DataFrame,
    pub conditions: Vec<Condition>,
    pub how: String,
    pub current_chunk: usize,
    pub chunk_size: usize,
}

impl Iterator for JoinIterator {
    type Item = Result<DataFrame, BearBridgeError>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.current_chunk * self.chunk_size;
        // Break loop when no more left rows to watch.
        if start >= self.left.height() {
            return None;
        }
        // Make a slice from left dataframe
        let end = (start + self.chunk_size).min(self.left.height());
        let left_slice = self.left.slice(start as i64, end - start);

        self.current_chunk += 1;

        let joined_slice = execute_join(
            &left_slice,
            &self.right,
            &self.conditions,
            self.how.as_str(),
        );

        match joined_slice {
            Ok(df) if df.height() > 0 => Some(Ok(df)),
            Err(e) => Some(Err(e)),
            _ => self.next(),
        }
    }
}
