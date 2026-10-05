# Hybrid-Format
> Rust 1.87+, `no_std` (runtime formatting needs an allocator)

`hformat!` macro that formats a string like [`format!`](https://doc.rust-lang.org/std/macro.format.html), but faster.

Constant arguments (literals, `const` blocks, `SCREAMING_SNAKE_CASE` names) are formatted at compile time. If every argument is a constant, the macro outputs a `&'static str`, otherwise it outputs a `String` built with a single allocation.

Format specs (fill, align, precision, ...) aren't supported yet.

You can essentially type any expression inside `{}`, it doesn't require an inner block.

The following types can be formatted both at compile-time and runtime (the implementations are very optimized):
- `&str`
- `bool`
- `char`
- `f32`/`f64`
- `i8`,`i16`,`i32`,`i64`,`i128`,`isize`,`u8`,`u16`,`u32`,`u64`,`u128`,`usize`

## Compile-time formatting
To format your own types at compile-time, you need to pass a `ConstFormattedString` literal/constant or a function that returns a `ConstFormattedString`.
```rust
impl<const N: usize> ConstFormattedString<N> {
    pub const fn new() -> Self;
    pub const fn len(&self) -> usize;
    pub const fn is_empty(&self) -> bool;
    pub const fn as_str(&self) -> &str;
    pub const fn push_str(mut self, string: &str) -> Self;
    pub const fn push_char(self, c: char) -> Self;
    pub const fn push_bool(self, b: bool) -> Self;
    pub const fn push_f64(self, f: f64) -> Self;
    pub const fn push_f32(self, f: f32) -> Self;
    pub const fn push_u8(self, n: u8) -> Self;
    pub const fn push_u16(self, n: u16) -> Self;
    pub const fn push_u32(self, n: u32) -> Self;
    pub const fn push_u64(self, n: u64) -> Self;
    pub const fn push_u128(self, n: u128) -> Self;
    pub const fn push_usize(self, n: usize) -> Self;
    pub const fn push_i8(self, n: i8) -> Self;
    pub const fn push_i16(self, n: i16) -> Self;
    pub const fn push_i32(self, n: i32) -> Self;
    pub const fn push_i64(self, n: i64) -> Self;
    pub const fn push_i128(self, n: i128) -> Self;
    pub const fn push_isize(self, n: isize) -> Self;
}
```

### Example
```rust
use hybrid_format::{hformat, ConstFormattedString};
struct Person {
    first_name: &'static str,
    last_name: &'static str,
    age: u8,
    balance: f64,
}
impl Person {
    const fn format(&self) -> ConstFormattedString<64> {
        ConstFormattedString::new()
            .push_str(self.last_name)
            .push_str(", ")
            .push_str(self.first_name)
            .push_str(" | Age: ")
            .push_u8(self.age)
            .push_str(" | $")
            .push_f64(self.balance)
    }
}
fn main() {
    const RANDOM_GUY: Person = Person {
        first_name: "John",
        last_name: "Doe",
        age: 40,
        balance: 32.0,
    };
    // You can format it once, at compile time, then use it by name like any other constant
    const RANDOM_GUY_STR: ConstFormattedString<64> = RANDOM_GUY.format();
    const GREETING: &str = hformat!("Hello, {RANDOM_GUY_STR}!");
    
    // You can also compute it at compile-time with a const block
    const SAME_GREETING: &str = hformat!("Hello, {}!", const { RANDOM_GUY.format() });
    
    assert_eq!(GREETING, "Hello, Doe, John | Age: 40 | $32.0!");
    assert_eq!(GREETING, SAME_GREETING);
    
    // And you can also mix it with runtime values
    let id = std::hint::black_box(0);
    assert_eq!(
        hformat!("#{id} - {RANDOM_GUY_STR}"),
        "#0 - Doe, John | Age: 40 | $32.0"
    );
}
```

## Runtime formatting
The built-in implementations aim to be as fast as possible.

To extend the implementations, and format your own types at runtime, use those two traits:
```rust
/// A not-yet-formatted value, that knows its formatted size, and can format(write) itself into a buffer without any reallocations.
/// # Safety
/// It is up to you to make sure that `size()` returns the exact or maximum size of your object when formatted (in bytes)!
/// `append()` uses this assumption to skip checks and avoid reallocation.
pub unsafe trait Formatted {
    /// The size of the object when formatted in bytes. Needs to be exact or at least an upper bound.
    fn size(&self) -> usize;
    /// Appends the formatted object to `buf`.
    /// # Safety
    /// This assumes `buf` still has at least `size()` bytes of remaining capacity.
    unsafe fn append(&self, buf: &mut String);
}

/// Converts an argument into an intermediate representation (that can be borrowed) that implements `Formatted`, that can then be formatted.
pub trait HybridFormat {
    type Formatted<'a>: Formatted
    where
        Self: 'a;
    /// Formats the object into an intermediate representation, that can be borrowed.
    fn format(&self) -> Self::Formatted<'_>;
}
```

## Limitations / Quirks
- To type the character `{`, type `{{` (like the `format!()` macro)
- To type the character `}`, type `}}` (like the `format!()` macro)
- Format specs aren't supported yet (they can only be parsed right now)
- Floats with zero decimal places are formatted with a trailing zero

The goal is to eventually fully support [https://doc.rust-lang.org/std/fmt/index.html](https://doc.rust-lang.org/std/fmt/index.html):
```
format_string := text [ maybe_format text ] *
maybe_format := '{' '{' | '}' '}' | format
format := '{' [ argument ] [ ':' format_spec ] [ ws ] * '}'
argument := integer | identifier

format_spec := [[fill]align][sign]['#']['0'][width]['.' precision][type]
fill := character
align := '<' | '^' | '>'
sign := '+' | '-'
width := count
precision := count | '*'
type := '?' | 'x?' | 'X?' | 'o' | 'x' | 'X' | 'p' | 'b' | 'e' | 'E'
count := parameter | integer
parameter := argument '$'
```

## Usage
```rust
use hybrid_format::hformat;
#[test]
fn hybrid() {
    const MY_INT: i32 = 42;
    const MY_BOOL: bool = true;
    const MY_STR: &str = "Hello, world!";
    let f = 4.2 + 6.7;
    let c = 'a';
    assert_eq!(
        hformat!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"),
        format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}")
    );
}
#[test]
fn const_int_literal() {
    assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
    assert_eq!(const { hformat!("{42}") }, "42");
}
#[test]
fn const_float_literal() {
    assert_eq!(
        const { hformat!("Float: {}", 4.2) },
        format!("Float: {}", 4.2)
    );
}
#[test]
fn const_bool_literal() {
    assert_eq!(
        const { hformat!("Bool: {}", true) },
        format!("Bool: {}", true)
    );
}
#[test]
fn const_char_literal() {
    assert_eq!(
        const { hformat!("Char: {}", 'a') },
        format!("Char: {}", 'a')
    );
}
#[test]
fn const_float_variable() {
    const MY_FLOAT: f64 = 4.2;
    assert_eq!(
        const { hformat!("Float: {}", MY_FLOAT) },
        format!("Float: {}", 4.2)
    );
}
```

## Benchmarks
| Benchmark    | `std::format!` | `hformat!` | Speedup compared to `std::format!` |
| -------- | ------- | ------- | ------- |
| constant  | 13.4ns | 311.2ps (just a &'static str) | 43x |
| all_dynamic | 117ns | 36ns | 3.25x |
| ten_const_ten_dynamic | 446.8ns | 63.8ns | 7x |

Ad astra per aspera!