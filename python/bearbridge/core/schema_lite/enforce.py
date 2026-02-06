import pandas as pd

class SchemaEnforcer:
    # Map of potentially messy types to high-performance Arrow types
    TYPE_MAP = {
        "string": "string[pyarrow]",
        "integer": "int64[pyarrow]",
        "floating": "float64[pyarrow]",
        "boolean": "boolean[pyarrow]",
        "datetime": "datetime64[ns]",
    }

    @classmethod
    def infer_and_coerce(cls, df: pd.DataFrame) -> pd.DataFrame:
        """
        Infers the intent of a column and forces it into a clean,
        non-object Arrow-backed type.
        """
        df = df.copy() # Avoid SettingWithCopy warnings

        for col in df.columns:
            series = df[col]
            inferred = pd.api.types.infer_dtype(series)

            # 1. Reject Mixed Types (Rust's worst enemy)
            if inferred == "mixed":
                raise TypeError(f"Column '{col}' contains mixed types. Rust requires a unified schema.")

            # 2. Map to Clean Type
            target_type = cls.TYPE_MAP.get(inferred)

            if target_type:
                try:
                    df[col] = series.astype(target_type)
                except (ValueError, TypeError) as e:
                    # Fallback for complex datetime strings
                    if inferred == "datetime":
                        df[col] = pd.to_datetime(series).dt.as_unit("ns")
                    else:
                        raise e
            elif inferred == "object":
                # If we can't infer it, but it's an object, it's a risk
                raise TypeError(f"Could not safely infer a high-performance type for object column '{col}'.")

        return df