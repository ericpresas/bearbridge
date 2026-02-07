import pandas as pd
import numpy as np
import time
from bearbridge.core import join

def generate_realistic_bench_data(n_rows=1_000_000):
    # Higher cardinality (50k unique IDs) prevents the 1-billion-row explosion
    unique_ids = 50_000

    # Create IDs with some overlap
    ids_l = np.random.randint(0, unique_ids, n_rows)
    ids_r = np.random.randint(0, unique_ids, n_rows)

    # Timestamps spread over 24 hours
    base_ts = 1600000000
    ts_l = base_ts + np.random.randint(0, 86400, n_rows)
    ts_r = base_ts + np.random.randint(0, 86400, n_rows)

    left_df = pd.DataFrame({
        "id": ids_l,
        "timestamp": pd.to_datetime(ts_l, unit='s'),
        "val_l": np.random.randn(n_rows),
        "category": np.random.choice(['A', 'B', 'C', 'D'], n_rows)
    })

    right_df = pd.DataFrame({
        "id": ids_r,
        "timestamp": pd.to_datetime(ts_r, unit='s'),
        "val_r": np.random.randn(n_rows),
        "category": np.random.choice(['A', 'B', 'C', 'D'], n_rows)
    })

    return left_df, right_df

def test_benchmark_join():
    L, R = generate_realistic_bench_data(1_000_000)

    # Condition: Same ID AND Timestamps within 5 seconds of each other
    conditions = [
        {"left_col": "id", "right_col": "id", "op": "=="},
        {"left_col": "timestamp", "right_col": "timestamp", "op": "<", "thr": 5, "time_unit": "s"}
    ]

    print(f"Starting Benchmark on {len(L)} rows...")

    # --- BEAR (RUST) ---
    start = time.perf_counter()
    # Using the accessor we built earlier
    res_bear = join(L, R, conditions=conditions, suffixes=("_l", "_r"))
    bear_time = time.perf_counter() - start
    print(f"Bear Join: {bear_time:.4f}s | Rows: {len(res_bear)}")

    # --- PANDAS (Standard Merge + Filter) ---
    # Pandas can't do the time delta inside the merge, so we must merge on ID first
    start = time.perf_counter()
    res_pd = L.merge(R, on="id", suffixes=("_l", "_r"))
    # Then filter by the time condition
    res_pd = res_pd[
        (res_pd["timestamp_l"] - res_pd["timestamp_r"]).abs() < pd.Timedelta(seconds=5)
        ]
    pd_time = time.perf_counter() - start
    print(f"Pandas Join: {pd_time:.4f}s | Rows: {len(res_pd)}")

    print(f"\nSpeedup: {pd_time / bear_time:.2f}x")

    assert pd_time > bear_time, "Lack of performance"