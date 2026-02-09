from typing import List, Dict
from .validate import JoinConditionDict, JoinCondition

def prepare_join_metadata(
        l_cols: List[str],
        r_cols: List[str],
        conditions: List[JoinConditionDict],
        suffixes: tuple[str, str] = ("_left", "_right")
) -> tuple[List[JoinConditionDict], Dict[str, str], Dict[str, str]]:
    """
    Unified pipeline to:
    1. Identify overlapping columns and generate rename maps.
    2. Validate and transform join conditions to match renamed columns.
    Args:
        l_cols (List[str]): List of column names.
        r_cols (List[str]): List of column names.
        conditions (List[JoinConditionDict]): List of join conditions.
        suffixes (tuple, optional): Suffixes for rename columns when equal in both sides. Defaults to ("_left", "_right").

    Returns:

    """
    l_suffix, r_suffix = suffixes

    # Generate Rename Dicts
    common_cols = set(l_cols).intersection(set(r_cols))
    left_rename = {col: f"{col}{l_suffix}" for col in l_cols if col in common_cols}
    right_rename = {col: f"{col}{r_suffix}" for col in r_cols if col in common_cols}

    # Process Conditions
    final_conditions = []
    for cond_dict in conditions:
        # Instantiate (triggers __post_init__ validation)
        cond = JoinCondition(**cond_dict)

        # Validate existence in original columns
        cond.validate_cols(l_cols, r_cols)

        # Apply renaming to the condition so it matches the future state
        # We use .get(key, key) to use the renamed version or keep the original
        new_left = left_rename.get(cond.left_col, cond.left_col)
        new_right = right_rename.get(cond.right_col, cond.right_col)

        # Replace col values
        cond.left_col = new_left
        cond.right_col = new_right


        # Convert to dict for Rust bridge
        final_conditions.append(cond.to_typed_dict())

    return final_conditions, left_rename, right_rename
