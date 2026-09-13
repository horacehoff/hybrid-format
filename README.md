# Hybrid-Format
A very experimental Rust library that provides an `hformat` macro that's faster than Rust's `format` macro, but is more limited too.
The whole idea of `hformat` is to do as much of the work at compilation time as possible. It's called "hybrid" because it supports both constants and dynamic values.

Literals and const blocks are automatically inlined. Const variables are not automatically inlined, you need to wrap them in a `const {...}` block.