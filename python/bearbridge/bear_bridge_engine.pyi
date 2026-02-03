import pyarrow

def conditional_join(
    left: pyarrow.Table,
    right: pyarrow.Table,
    conditions: list[dict],
    how: str = "inner"
) -> pyarrow.Table:
    """
    Executes a conditional join between two Arrow tables.
    """
    ...