from dataclasses import dataclass
from typing import Optional, Literal, Union, TypedDict

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

@dataclass(frozen=True)
class JoinCondition:
    left_col: str
    right_col: str
    op: Operator
    thr: Optional[Union[int, float]] = None
    time_unit: Optional[TimeUnit] = None

    def __post_init__(self):
        if self.op not in {">", "<", ">=", "<=", "==", "!=", "weekday", "fuzzy"}:
            raise ValueError(f"Unsupported operator '{self.op}'. Valid: {Operator}")

        # 2. Validate Threshold Requirements
        if self.time_unit and self.thr is None:
            raise ValueError("Threshold 'thr' must be provided when 'time_unit' is specified.")

        if self.thr is not None and not isinstance(self.thr, (int, float)):
            raise TypeError(f"Threshold 'thr' must be a number, got {type(self.thr).__name__}.")

    def to_dict(self) -> dict:
        """
        Converts the object back to a dict for the Rust bridge.
        """

        # Removes None values so the Rust enum mapper doesn't get confused
        return {k: v for k, v in self.__dict__.items() if v is not None}