import pyarrow as pa
import bearbridge # Your compiled module
import pandas as pd
import numpy as np
import time

def test_join():
    # 1. Create Arrow Tables
    left_table = pa.table({
        "id": [1, 2, 3],
        "name": ["apple", "banana", "cherry"],
        "val": [100, 200, 300]
    })

    right_table = pa.table({
        "id_a": [2, 3, 4],
        "tag": ["B", "C", "D"],
        "score": [0.5, 0.8, 1.2]
    })

    # 2. Define Join Conditions (matching your Condition enum)
    # Using 'type' as the Serde tag we defined earlier
    conditions = [
        {"op": "==", "left_col": "id", "right_col": "id_a"}
    ]

    print("--- Executing Join ---")
    try:
        # Call the renamed function
        result = bearbridge.conditional_join(
            left_table.to_batches(),
            right_table.to_batches(),
            conditions,
            how="inner"
        )

        # 3. Inspect Results
        df = pa.Table.from_batches(batches=result).to_pandas()
        print(df)

        # Verify schema/data
        assert "name" in df.columns
        assert "score" in df.columns
        assert len(df) == 2 # Only IDs 2 and 3 should match
        print("\n✅ Python test passed! FFI Bridge is stable.")

    except Exception as e:
        print(f"\n❌ Python test failed: {e}")

def mock_large_dfs(N: int):

    # 2. Create Left Table: Transactions
    left_data = {
        "user_id": np.random.randint(0, 100_000, N),
        "amount": np.random.uniform(10, 500, N),
        "timestamp": pd.date_range("2026-01-01", periods=N, freq="s")
    }

    right_data = {
        "id": np.random.randint(0, 100_000, N),
        "status": np.random.choice(["success", "fail"], N),
        "timestamp": left_data["timestamp"] + pd.to_timedelta(np.random.randint(1, 6, N), unit='s')
    }

    return pd.DataFrame(left_data), pd.DataFrame(right_data)

def test_large(left_df, right_df):
    # 1. Configuration
    left_table = pa.Table.from_pandas(left_df)

    right_table = pa.Table.from_pandas(right_df)

    print(f"Left Unit: {left_table.schema.field('timestamp').type.unit}")
    print(f"Right Unit: {right_table.schema.field('timestamp').type.unit}")

    # Define the conditions using your new dictionary format
    conditions = [
        {
            "left_col": "user_id",
            "right_col": "id",
            "op": "=="
        },
        {
            "left_col": "timestamp",
            "right_col": "timestamp_right",
            "op": "<",
            "thr": 10,
            "time_unit": "s"
        }
    ]

    start_time = time.perf_counter()

    # Perform the join using the batches approach
    # We use .to_batches() to satisfy the PyArrowType<Vec<RecordBatch>> requirement
    results_raw = bearbridge.conditional_join(
        left_table.to_batches(),
        right_table.to_batches(),
        conditions,
        "inner"
    )

    # Reconstruct and convert to Pandas
    result_df = pa.Table.from_batches(results_raw).to_pandas(types_mapper=None)


    end_time = time.perf_counter()
    print(f"Join completed in {end_time - start_time:.4f} seconds.")
    print(f"Resulting shape: {result_df.shape}")
    return result_df

def test_pure_pandas(left_df, right_df):
    print(f"Starting Pure Pandas join...")
    start_time = time.perf_counter()

    # 1. Standard Hash Join on IDs
    # This creates a massive intermediate table in memory
    merged = pd.merge(left_df, right_df, left_on="user_id", right_on="id")

    # 2. Vectorized filter for the time condition
    # timestamp_x and timestamp_y are created by the merge
    mask = abs((merged["timestamp_y"] - merged["timestamp_x"]).dt.total_seconds()) < 10
    result_df = merged.loc[mask]

    end_time = time.perf_counter()
    print(f"Pandas Join completed in {end_time - start_time:.4f} seconds.")
    print(f"Resulting shape: {result_df.shape}")
    return result_df

if __name__ == "__main__":
    left_df, right_df = mock_large_dfs(1_000_000)
    df1 = test_large(left_df, right_df)
    df2 = test_pure_pandas(left_df, right_df)

    result = pd.merge(
        df2,
        df1,
        how="left",
        on=["id","user_id"],
        suffixes=("", "_pl")
    )
    print()