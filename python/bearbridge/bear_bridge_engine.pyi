from typing import List, Optional, Dict
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

class JoinStream:
    """A streaming iterator yielding pyarrow.Tables from the Bear engine."""
    def __iter__(self) -> 'JoinStream': ...
    def __next__(self) -> Optional[List[pyarrow.RecordBatch]]: ...

def conditional_join_stream(
    left: List[pyarrow.RecordBatch],
    right: List[pyarrow.RecordBatch],
    conditions: List[JoinConditionDict],
    batch_size: int = 100000,
    how: str = "inner"
) -> JoinStream:
    """Starts a memory-efficient streaming join."""
    ...