use crate::errors::BearBridgeError;
use std::str::FromStr;
use polars::prelude::{DataFrame, Expr, IntoLazy};

fn get_predicates(
    left: &DataFrame,
    right: &DataFrame,
    conditions: &[(String, String, String)]
) -> Result<Vec<Expr>, BearBridgeError> {
    let mut errors: Vec<BearBridgeError> = Vec::new();
    let left_columns = left.get_column_names();
    let right_columns = right.get_column_names();
    let mut predicates: Vec<Expr> = Vec::new();

    for (left_col, right_col, op) in conditions {
        if !left_columns.iter().any(|x| x.as_str() == left_col) {
            errors.push(
                BearBridgeError::ColumnNotFound {
                    column: left_col.clone(),
                    side: "left".into()
                }
            );
        }
        if !right_columns.iter().any(|x| x.as_str() == right_col) {
            errors.push(
                BearBridgeError::ColumnNotFound {
                    column: right_col.clone(),
                    side: "right".into()
                }
            );
        }
        match Operator::from_str(op.as_str()) {
            Ok(op_enum) => predicates.push(op_enum.pl_expr(left_col, right_col)),
            Err(e) => errors.push(e)
        }

    }

    if !errors.is_empty() {
        let json_err = serde_json::to_string(&errors)
            .unwrap_or_else(|_| "[]".to_string());
        return Err(BearBridgeError::MultiError(errors.len(), json_err));
    }

    Ok(predicates)
}

pub fn conditional_join(
    left: &DataFrame,
    right: &DataFrame,
    conditions: &[(String, String, String)]
) -> Result<DataFrame, BearBridgeError> {

    // Validate and get predicates
    let predicates = get_predicates(left, right, conditions)?;

    // Get a single predicates expression using AND operator.
    // let final_predicate = predicates
    //     .into_iter()
    //     .reduce(|acc, p| acc.and(p))
    //     .ok_or_else(|| BearBridgeError::InvalidOperator("No conditions provided".into()))?;

    // Perform join
    let result = left.clone().lazy()
        .join_builder()
        .with(right.clone().lazy())
        .join_where(predicates)
        .collect()?;

    Ok(result)
}


#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    #[test]
    fn test_full_conditional_join() {
        // 1. Create Left DataFrame (Students)
        let df_students = df!(
            "name" => ["Alice", "Bob", "Charlie"],
            "score" => [85, 42, 95]
        ).expect("failed to create students df");

        // 2. Create Right DataFrame (Requirements)
        let df_reqs = df!(
            "min_score" => [50],
            "category" => ["passing"]
        ).expect("failed to create requirements df");

        // 3. Define conditions: score > min_score
        let conditions = vec![
            ("score".to_string(), "min_score".to_string(), ">".to_string())
        ];

        // 4. Run the join
        let result = conditional_join(&df_students, &df_reqs, &conditions)
            .expect("Join failed");

        // 5. Assertions
        // Alice (85) and Charlie (95) are > 50. Bob (42) is filtered out.
        assert_eq!(result.height(), 2);

        let names = result.column("name").unwrap().str().unwrap();
        assert!(names.into_iter().any(|opt_name| opt_name == Some("Alice")));
        assert!(names.into_iter().any(|opt_name| opt_name == Some("Charlie")));

        println!("Resulting Join:\n{}", result);
    }
}