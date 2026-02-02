mod types;
mod operators;


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


#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;
    use crate::engine::conditional_join::types::{Condition, TimeUnit};

    // Helper to create a basic DataFrame
    fn create_test_df() -> DataFrame {
        df!(
            "id" => [1, 2, 3],
            "val" => [10.0, 20.0, 30.0],
            "ts" => [1000i64, 2000i64, 3000i64] // Mock timestamps in nanos
        ).unwrap()
    }

    #[test]
    fn test_standard_inner_join() -> Result<(), Box<dyn std::error::Error>> {
        let df1 = create_test_df();
        let df2 = create_test_df();

        let conditions = vec![Condition::Standard {
            left_col: "id".to_string(),
            right_col: "id_right".to_string(),
            op: "==".to_string(),
        }];

        let result = execute_join(df1, df2, conditions, "inner")?;

        // Standard join on 1, 2, 3 should return 3 rows
        println!("{:?}", result);
        assert_eq!(result.height(), 3);
        Ok(())
    }

    #[test]
    fn test_difference_threshold_join() -> Result<(), Box<dyn std::error::Error>> {
        let df1 = df!("val" => [10.0]).unwrap();
        let df2 = df!("val" => [12.0, 15.0]).unwrap();

        // Join where abs(left - right) <= 3.0
        // Should match 10 with 12, but NOT 10 with 15.
        let conditions = vec![Condition::Difference {
            left_col: "val".to_string(),
            right_col: "val".to_string(),
            op: "<=".to_string(),
            thr: 3.0,
        }];

        let result = execute_join(df1, df2, conditions, "inner")?;

        assert_eq!(result.height(), 1);
        assert_eq!(result.column("val_right")?.f64()?.get(0), Some(12.0));
        Ok(())
    }

    #[test]
    fn test_time_difference_join() -> Result<(), Box<dyn std::error::Error>> {
        let df1 = df!("ts" => [1_000_000_000i64]).unwrap(); // 1 second
        let df2 = df!("ts" => [1_500_000_000i64, 3_000_000_000i64]).unwrap(); // 1.5s and 3s

        // Join where difference is <= 1 second
        let conditions = vec![Condition::TimeDifference {
            left_col: "ts".to_string(),
            right_col: "ts".to_string(),
            op: "<=".to_string(),
            thr: 1.0,
            time_unit: "seconds".to_string(),
        }];

        let result = execute_join(df1, df2, conditions, "inner")?;

        // Only the 1.5s record is within 1s of the 1s record.
        assert_eq!(result.height(), 1);
        Ok(())
    }
}