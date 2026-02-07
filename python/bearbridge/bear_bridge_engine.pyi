from typing import List
import pyarrow
from bearbridge.core.conditional_join.validate import JoinConditionDict

def conditional_join(
    left: List[pyarrow.RecordBatch],
    right: List[pyarrow.RecordBatch],
    conditions: List[JoinConditionDict],
    how: str = "inner"
) -> pyarrow.Table:
    """
    Executes a conditional join between two Arrow tables.
    """
    ...