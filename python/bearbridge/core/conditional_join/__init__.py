from typing import List

import pandas as pd
import pyarrow as pa

from bearbridge.bear_bridge_engine import conditional_join
from bearbridge.core.schema_lite.enforce import SchemaEnforcer
from .validate import JoinConditionDict
from .utilities import prepare_join_metadata


def join(
    left: pd.DataFrame,
    right: pd.DataFrame,
    conditions: List[JoinConditionDict],
    how: str = "left",
    suffixes: (str, str) = None,
) -> pd.DataFrame:
    """
    Conditional join between two dataframes.
    Args:
        left (pd.DataFrame): The left dataframe.
        right (pd.DataFrame): The right dataframe.
        conditions (List[JoinConditionDict]): List of condition dictionaries.
        how (str, optional): How to join columns. Defaults to "left".
        suffixes (tuple, optional): Suffixes to add to column names. Defaults to None.

    Returns:
        pd.DataFrame: The joined dataframe.
    """
    left_columns = left.columns.tolist()
    right_columns = right.columns.tolist()

    # Coerce them to strict types
    left = SchemaEnforcer.infer_and_coerce(left)
    right = SchemaEnforcer.infer_and_coerce(right)

    args_prepare = {
        "l_cols": left_columns,
        "r_cols": right_columns,
        "conditions": conditions,
    }
    if suffixes is not None:
        args_prepare["suffixes"] = suffixes

    # Validate conditions and rename dataframe columns based on suffixes
    final_conditions, left_rename, right_rename = prepare_join_metadata(**args_prepare)
    left = left.rename(columns=left_rename)
    right = right.rename(columns=right_rename)

    # Convert dataframes to pyarrow tables
    left_arr = pa.Table.from_pandas(df=left)
    right_arr = pa.Table.from_pandas(df=right)

    # Perform backend conditional_join
    result_arr = conditional_join(
        left=left_arr.to_batches(),
        right=right_arr.to_batches(),
        conditions=final_conditions,
        how=how
    )

    # Turn back to pandas Dataframe from pyarrow Table
    df = pa.Table.from_batches(batches=result_arr).to_pandas()
    return df

