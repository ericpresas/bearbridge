# 🐻 BearBridge

**High-performance Non-Equi Joins for Pandas, powered by Rust.**

BearBridge is a lightweight Pandas extension that bridges the gap between Python's ease of use and Rust's computational speed. It specializes in **Non-Equi Joins**—joins based on inequalities, ranges, or time-deltas—where standard Pandas often struggles with memory "explosions" and `SIGKILL` errors.

---

## 🚀 Performance Comparison

In a join involving **5,000,000 rows** with an equality check plus a 5-second time-delta condition:

| Metric | Pandas (Standard) | BearBridge (Streaming) | Improvement |
| :--- | :--- | :--- | :--- |
| **Execution Time** | 56.15s | **4.95s** | **11.3x Faster** |
| **Peak RAM** | 1,974.11 MB | **320.02 MB** | **83.8% Less Memory** |

---

## ✨ Features

- **Non-Equi Joins**: Join on `>`, `<`, `>=`, `<=`, and `abs(a - b) < threshold`.
- **Memory Streaming**: Process datasets larger than your RAM using the `join_stream` generator.
- **Zero-Copy Transfer**: Uses Apache Arrow to move data between Python and Rust without serialization overhead.
- **Pandas Native**: Access all functionality directly through the `.bear` accessor.
- **Type Safety**: Automatic schema inference and coercion to prevent runtime crashes.

---

## 🛠 Installation
*Not published yet*

## License

This project is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.