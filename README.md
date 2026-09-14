# Hybrid-Format
A very experimental Rust library that provides an `hformat` macro that's faster than Rust's `format` macro, but is more limited too.
The whole idea of `hformat` is to do as much of the work at compilation time as possible. It's called "hybrid" because it supports both constants and dynamic values.

Literals and const blocks are automatically inlined. Variables with `SCREAMING_SNAKE_CASE` names are treated as constants and are automatically inlined.
Anything else needs to be wrapped in a `const {...}` block to be treated as a constant.

If the macro only contains constants, it returns a constant `&str`, otherwise it returns a `String`.

## Benchmarks
| Benchmark    | `std::format!` | `hformat!` | Speedup compared from `std::format!` |
| -------- | ------- | ------- | ------- |
| constant  | 13400ps | 311.2ps | 43x |
| all_dynamic | 117ns | 36ns | 3.25x |
| ten_const_ten_dynamic | 446.8ns | 63.8ns | 7x |

Ad astra per aspera!