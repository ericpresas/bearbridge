from typing import List

import pandas as pd
import pyarrow as pa

from bearbridge.bear_bridge_engine import conditional_join
from .validate import JoinCondition, JoinConditionDict


def join(
    left: pd.DataFrame,
    right: pd.DataFrame,
    conditions: List[JoinConditionDict],
    how: str = "left"
):
    """

    :param left:
    :type left:
    :param right:
    :type right:
    :param conditions:
    :type conditions:
    :param how:
    :type how:
    :return:
    :rtype:
    """

    # Validate conditions
    conditions = [JoinCondition(**condition).to_dict() for condition in conditions]

    left_arr = pa.Table.from_pandas(df=left)
    right_arr = pa.Table.from_pandas(df=right)

    result_arr = conditional_join(left_arr.to_batches(), right_arr.to_batches(), conditions, how)
    df = pa.Table.from_batches(batches=result_arr).to_pandas()
    return df

