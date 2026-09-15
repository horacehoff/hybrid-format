# Hybrid-Format
A very experimental Rust library that provides an `hformat` macro that's faster than Rust's `format` macro, but is more limited too.
The whole idea of `hformat` is to do as much of the work at compilation time as possible. It's called "hybrid" because it supports both constants and dynamic values.

Literals and const blocks are automatically inlined. Variables with `SCREAMING_SNAKE_CASE` names are treated as constants and are automatically inlined (NOTE: if they are not constants, the macro will produce an error).
Anything else needs to be wrapped in a `const {...}` block to be treated as a constant.
You can essentially type any expression inside `{}`, it doesn't require an inner block.

If the macro only contains constants, it returns a constant `&str`, otherwise it returns a `String`.

Two traits are used:
```rust
pub trait Formatted {
    /// The size of the object when formatted in bytes. Needs to be exact or an upper bound.
    fn size(&self) -> usize;
    /// Appends the formatted object to `buf`. `buf` is guaranteed to have enough capacity.
    fn append(&self, buf: &mut String);
}

pub trait HybridFormat {
    type Formatted<'a>: Formatted
    where
        Self: 'a;
    /// Formats the object into an intermediate representation that can be borrowed.
    fn format(&self) -> Self::Formatted<'_>;
}
```
They can be extended, and you can choose the fastest/most efficient intermediate representation for your type.
Built-in implementations are:
- `str`/`&str`/`String`
- `bool`
- `char`
- `f32`/`f64`
- `i8`,`i16`,`i32`,`i64`,`128`,`isize`,`u8`,`u16`,`u32`,`u64`,`u128`,`usize`

The built-in implementations try to be as fast as possible.

## Limitations / Quirks
- To type the character `{`, type `{{` (like the `format!()` macro)
- To type the character `}`, type `}` (unlike the `format!()` macro)
- There are no format modifiers yet (such as `{:?}`, `{:.2}`, ...)
- `f32`/`f64` consts need to be annotated with `as f32`/`as f64`
- Floats with zero decimal places are formatted with a trailing zero

The goal is to eventually fully support [https://doc.rust-lang.org/std/fmt/index.html](https://doc.rust-lang.org/std/fmt/index.html)

## Usage
```rust
assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));

assert_eq!(const { hformat!("{42}") }, "42");

assert_eq!(
    const { hformat!("Hello, world!") },
    format!("Hello, world!")
);

assert_eq!(
    const { hformat!("Float: {}", 4.2) },
    format!("Float: {}", 4.2)
);

let x = 4.2e5;
assert_eq!(hformat!("Float: {x}"), format!("Float: {x}.0"));

const B: bool = true;
assert_eq!(const { hformat!("Bool: {B}") }, format!("Bool: {B}"));
```

## Benchmarks
| Benchmark    | `std::format!` | `hformat!` | Speedup compared from `std::format!` |
| -------- | ------- | ------- | ------- |
| constant  | 13400ps | 311.2ps | 43x |
| all_dynamic | 117ns | 36ns | 3.25x |
| ten_const_ten_dynamic | 446.8ns | 63.8ns | 7x |

Ad astra per aspera!