import tracemalloc
import pandas as pd
import numpy as np
import time
from bearbridge.core import join
from bearbridge.core.conditional_join import join_stream


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

def pandas_join_stream(left, right, on_col, time_col, thr_seconds, batch_size):
    """
    Simulates the streaming join behavior using pure Pandas.
    """
    chunks = []
    for i in range(0, len(left), batch_size):
        # 1. Slice the left chunk
        l_chunk = left.iloc[i : i + batch_size]

        # 2. Perform the equality join (ID)
        merged = l_chunk.merge(right, on=on_col, suffixes=("_l", "_r"))

        # 3. Apply the non-equi filter (Timestamp)
        mask = (merged[f"{time_col}_l"] - merged[f"{time_col}_r"]).abs() < pd.Timedelta(seconds=thr_seconds)
        filtered = merged[mask]

        chunks.append(filtered)

    return pd.concat(chunks, ignore_index=True) if chunks else pd.DataFrame()

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

def test_benchmark_join_stream():
    # Generate 1M rows
    L, R = generate_realistic_bench_data(1_000_000)

    batch_size = 100_000
    conditions = [
        {"left_col": "id", "right_col": "id", "op": "=="},
        {"left_col": "timestamp", "right_col": "timestamp", "op": "<", "thr": 5, "time_unit": "s"}
    ]

    print(f"📊 Memory Benchmark: {len(L):,} rows | Batch Size: {batch_size:,}")
    print("-" * 65)

    # --- TEST 1: BEAR STREAM (RUST) ---
    tracemalloc.start()
    start = time.perf_counter()

    bear_chunks = [chunk for chunk in join_stream(
        left=L, right=R, conditions=conditions,
        how="inner", suffixes=("_l", "_r"), batch_size=batch_size
    )]

    res_bear = pd.concat(bear_chunks) if bear_chunks else pd.DataFrame()
    bear_time = time.perf_counter() - start
    _, bear_peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    print(f"🌊 Bear Stream: {bear_time:.4f}s | Peak RAM: {bear_peak / 10**6:.2f} MB")

    # --- TEST 2: PANDAS STREAM (PURE PYTHON) ---
    tracemalloc.start()
    start = time.perf_counter()

    res_pd = pandas_join_stream(
        left=L, right=R, on_col="id",
        time_col="timestamp", thr_seconds=5, batch_size=batch_size
    )

    pd_time = time.perf_counter() - start
    _, pd_peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    print(f"🐼 Pandas Stream: {pd_time:.4f}s | Peak RAM: {pd_peak / 10**6:.2f} MB")

    # --- COMPARISON ---
    print("-" * 65)
    print(f"🚀 Speedup: {pd_time / bear_time:.2f}x")
    print(f"📉 Memory Saving: {((pd_peak - bear_peak) / pd_peak) * 100:.1f}% less RAM used by Bear")

if __name__ == "__main__":
    test_benchmark_join()
    test_benchmark_join_stream()