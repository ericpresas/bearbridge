from dataclasses import dataclass
from typing import Optional, Literal, Union, TypedDict, List, Dict

# Define types for IDE support
Operator = Literal[">", "<", ">=", "<=", "==", "!=", "weekday", "fuzzy"]
TimeUnit = Literal["d", "h", "m", "s", "ms", "days", "hours", "minutes", "seconds", "milliseconds"]


# 1. Define the Schema for type checking (The "Shape")
class JoinConditionDict(TypedDict):
    left_col: str
    right_col: str
    op: str
    thr: Optional[float]
    time_unit: Optional[str]

@dataclass
class JoinCondition:
    left_col: str
    right_col: str
    op: Operator
    thr: Optional[Union[int, float]] = None
    time_unit: Optional[TimeUnit] = None

    def __post_init__(self):
        """
        Performs Early condition validations after instance class is created.

        """
        if self.op not in {">", "<", ">=", "<=", "==", "!=", "weekday", "fuzzy"}:
            raise ValueError(f"Unsupported operator '{self.op}'. Valid: {Operator}")

        # 2. Validate Threshold Requirements
        if self.time_unit and self.thr is None:
            raise ValueError("Threshold 'thr' must be provided when 'time_unit' is specified.")

        if self.thr is not None and not isinstance(self.thr, (int, float)):
            raise TypeError(f"Threshold 'thr' must be a number, got {type(self.thr).__name__}.")

    def validate_cols(self, left_cols: List[str], right_cols: List[str]) -> None:
        """
        Validate columns in conditions available in dataframe.
        Args:
            left_cols (List[str]): List of column names.
            right_cols (List[str]): List of column names.

        Returns:

        """
        if self.left_col not in left_cols:
            raise KeyError(f"Column '{self.left_col}' not in 'left'.")

        if self.right_col not in right_cols:
            raise KeyError(f"Column '{self.right_col}' not in 'right'.")

    def to_typed_dict(self) -> JoinConditionDict:
        pure_dict = {}
        for key, item in self.__dict__.items():
            if item is not None:
                pure_dict[key] = item
        return JoinConditionDict(**pure_dict)

