# Hybrid-Format
A very experimental Rust library that provides an `hformat` macro that's faster than Rust's `format` macro, but is more limited too.
The whole idea of `hformat` is to do as much of the work at compilation time as possible. It's called "hybrid" because it supports both constants and dynamic values.

Literals and const blocks are automatically inlined. Variables with `SCREAMING_SNAKE_CASE` names are treated as constants and are automatically inlined.
Anything else needs to be wrapped in a `const {...}` block to be treated as a constant.