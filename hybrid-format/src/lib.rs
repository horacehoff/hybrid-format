//! [`hformat`] macro that formats a string like [`format!`](https://doc.rust-lang.org/std/macro.format.html), but faster and better.
//! Supports `no_std`, though runtime formatting needs an allocator.
//!
//! Constant arguments (literals, `const` blocks, `SCREAMING_SNAKE_CASE` names) are formatted at compile time.
//! If every argument is a constant, the macro outputs a `&'static str`, otherwise it outputs a `String` built with a single allocation.
//!
//! The following types can be formatted both at compile-time and runtime:
//! - `&str`
//! - `bool`
//! - `char`
//! - `f32`/`f64`
//! - `i8`,`i16`,`i32`,`i64`,`i128`,`isize`,`u8`,`u16`,`u32`,`u64`,`u128`,`usize`
//!
//! To format your own types at compile-time, look at [`ConstFormattedString`].
//! To format your own types at runtime, look at the two traits [`HybridFormat`] and [`Formatted`].
//! # Examples
//! ```
//! use hybrid_format::hformat;
//!
//! assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
//! assert_eq!(const { hformat!("{42}") }, "42");
//!
//! assert_eq!(
//!     const { hformat!("Float: {}", 4.2) },
//!     format!("Float: {}", 4.2)
//! );
//!
//! assert_eq!(
//!     const { hformat!("Bool: {}", true) },
//!     format!("Bool: {}", true)
//! );
//!
//! assert_eq!(
//!     const { hformat!("Char: {}", 'a') },
//!     format!("Char: {}", 'a')
//! );
//!
//! const MY_FLOAT: f64 = 4.2;
//! assert_eq!(
//!     const { hformat!("Float: {}", MY_FLOAT) },
//!     format!("Float: {}", 4.2)
//! );
//!
//! const MY_INT: i32 = 42;
//! const MY_BOOL: bool = true;
//! const MY_STR: &str = "Hello, world!";
//! let f = 4.2 + 6.7;
//! let c = 'a';
//! assert_eq!(
//!     hformat!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"),
//!     format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}")
//! );
//! ```

#![cfg_attr(not(test), no_std)]
extern crate alloc;
extern crate self as hybrid_format;

/// A string that is at most N bytes big and is built at compile time.
/// Use this when you want to format your own types at compile-time.
///
/// # Example
///
/// ```
/// use hybrid_format::hformat;
/// use hybrid_format::ConstFormattedString;
///
/// struct Person {
///     first_name: &'static str,
///     last_name: &'static str,
///     age: u8,
///     balance: f64,
/// }
/// impl Person {
///     const fn format(&self) -> ConstFormattedString<64> {
///         ConstFormattedString::new()
///             .push_str(self.last_name)
///             .push_str(", ")
///             .push_str(self.first_name)
///             .push_str(" | Age: ")
///             .push_u8(self.age)
///             .push_str(" | $")
///             .push_f64(self.balance)
///     }
/// }
/// fn main() {
///     const RANDOM_GUY: Person = Person {
///         first_name: "John",
///         last_name: "Doe",
///         age: 40,
///         balance: 32.0,
///     };
///     // You can format it once, at compile time, then use it by name like any other constant
///     const RANDOM_GUY_STR: ConstFormattedString<64> = RANDOM_GUY.format();
///     const GREETING: &str = hformat!("Hello, {RANDOM_GUY_STR}!");
///
///     // You can also compute it at compile-time with a const block
///     const SAME_GREETING: &str = hformat!("Hello, {}!", const { RANDOM_GUY.format() });
///
///     assert_eq!(GREETING, "Hello, Doe, John | Age: 40 | $32.0!");
///     assert_eq!(GREETING, SAME_GREETING);
///
///     // And you can also mix it with runtime values
///     let id = std::hint::black_box(0);
///     assert_eq!(
///         hformat!("#{id} - {RANDOM_GUY_STR}"),
///         "#0 - Doe, John | Age: 40 | $32.0"
///     );
/// }
/// ```
pub use hybrid_format_impls::const_args::ConstFormattedString;

pub use hybrid_format_impls::{Formatted, HybridFormat};

/// Macro that formats a string like [`format!`](https://doc.rust-lang.org/std/macro.format.html), but faster and better.
/// Constant arguments (literals, `const` blocks, `SCREAMING_SNAKE_CASE` names) are formatted at compile time.
/// If every argument is a constant, the macro outputs a `&'static str`, otherwise it outputs a `String` built with a single allocation.
///
/// # Examples
/// ```
/// use hybrid_format::hformat;
///
/// assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));            // => "42"
///
/// const MY_INT: i32 = 42;
/// assert_eq!(const { hformat!("{}", MY_INT) }, format!("{}", MY_INT));    // => "42"
///
/// let i = 2 + 2;
/// assert_eq!(hformat!("{}", i), format!("{}", i));                        // => "4"
///
/// assert_eq!(
///     const { hformat!("Float: {}", 4.2) },                               // => "Float: 4.2"
///     format!("Float: {}", 4.2)
/// );
///
/// const MY_FLOAT: f64 = 4.2;
/// assert_eq!(
///     const { hformat!("Float: {}", MY_FLOAT) },                          // => "Float: 4.2"
///     format!("Float: {}", MY_FLOAT)
/// );
///
/// const MY_BOOL: bool = true;
/// const MY_STR: &str = "Hello, world!";
/// let f = 4.2 + 6.7;
/// let c = 'a';
/// assert_eq!(
///     hformat!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"),
///     format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}")
/// );
/// ```
pub use hybrid_format_macros::hformat;

#[doc(hidden)]
pub mod __private {
    pub use alloc::string::String;

    pub use const_format;
    pub use hybrid_format_impls::const_args;
    pub use hybrid_format_impls::push_str_unchecked;
}

#[macro_export]
#[doc(hidden)]
macro_rules! __hformat_internal {
    // a dynamic (runtime) item
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr} $(, $($remaining:tt)*)?) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let _dynamic_elem = &$dynamic_elem;
        let _dynamic_formatted = $crate::HybridFormat::format(_dynamic_elem);
        $crate::__hformat_internal!(
            [$buffer]
            []
            [
                $($capacity_expr)*
                + _temp_formatted.len()
                + $crate::Formatted::size(&_dynamic_formatted)
            ]
            [$(
                $add_to_str_statements)*
                unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
                unsafe {$crate::Formatted::append(&_dynamic_formatted, $buffer)};
            ]
            $($($remaining)*)?
        )
    }};
    // static item
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr $(, $($remaining:tt)*)?) => {{
        $crate::__hformat_internal!(
            [$buffer]
            [$($pending_static_elems)* $static_elem,]
            [$($capacity_expr)*]
            [$($add_to_str_statements)*]
            $($($remaining)*)?
        )
    }};
    // pure const
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] []) => {
        $crate::__private::const_format::concatcp!($($pending_static_elems)*)
    };
    // runtime/const hybrid
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*]) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let mut $buffer = $crate::__private::String::with_capacity($($capacity_expr)* + _temp_formatted.len());
        {
            // shadowed just to give the statements a &mut String
            let $buffer = &mut $buffer;
            $($add_to_str_statements)*
            unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
        }
        $buffer
    }};
    // the last one, it's the one that's actually called in the code
    ($($elems: tt)*) => {
        $crate::__hformat_internal!(
            [buf]
            []
            [0usize]
            []
            $($elems)*
        )
    };
}

#[cfg(test)]
mod tests {
    use hybrid_format_impls::const_args::ConstFormattedString;
    use hybrid_format_macros::hformat;

    #[test]
    fn int_const_literal() {
        assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
    }
    #[test]
    fn int_const_variable() {
        const MY_INT: i32 = 42;
        assert_eq!(const { hformat!("{}", MY_INT) }, format!("{MY_INT}"));
    }
    #[test]
    fn int_dynamic() {
        let i = 2 + 2;
        assert_eq!(hformat!("{}", i), format!("{i}"));
    }
    #[test]
    fn float_const_literal() {
        assert_eq!(
            const { hformat!("Float: {}", 4.2) },
            format!("Float: {}", 4.2)
        );
    }
    #[test]
    fn float_const_variable() {
        const MY_FLOAT: f64 = 4.2;
        assert_eq!(
            const { hformat!("Float: {}", MY_FLOAT) },
            format!("Float: {MY_FLOAT}")
        );
    }
    #[test]
    fn float_dynamic() {
        let f = 4.2 + 6.7;
        assert_eq!(hformat!("Float: {}", f), format!("Float: {f}"));
    }
    #[test]
    fn bool_const_literal() {
        assert_eq!(
            const { hformat!("Bool: {}", true) },
            format!("Bool: {}", true)
        );
    }
    #[test]
    fn bool_const_variable() {
        const MY_BOOL: bool = true;
        assert_eq!(
            const { hformat!("Bool: {}", MY_BOOL) },
            format!("Bool: {MY_BOOL}")
        );
    }
    #[test]
    fn bool_dynamic() {
        let b = true;
        assert_eq!(hformat!("Bool: {}", b), format!("Bool: {b}"));
    }
    #[test]
    fn char_const_literal() {
        assert_eq!(
            const { hformat!("Char: {}", 'a') },
            format!("Char: {}", 'a')
        );
    }
    #[test]
    fn char_const_variable() {
        const MY_CHAR: char = 'a';
        assert_eq!(
            const { hformat!("Char: {}", MY_CHAR) },
            format!("Char: {MY_CHAR}")
        );
    }
    #[test]
    fn char_dynamic() {
        let c = 'a';
        assert_eq!(hformat!("Char: {}", c), format!("Char: {c}"));
    }
    #[test]
    fn str_const_literal() {
        assert_eq!(
            const { hformat!("String: {}", "Hello, world!") },
            format!("String: {}", "Hello, world!")
        );
    }
    #[test]
    fn str_const_variable() {
        const MY_STR: &str = "Hello, world!";
        assert_eq!(
            const { hformat!("String: {}", MY_STR) },
            format!("String: {MY_STR}")
        );
    }
    #[test]
    fn str_dynamic() {
        let s = "Hello, world!";
        assert_eq!(hformat!("String: {}", s), format!("String: {s}"));
    }
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
    fn custom_compiletime_type() {
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
        const RANDOM_GUY: Person = Person {
            first_name: "John",
            last_name: "Doe",
            age: 40,
            balance: 32.0,
        };
        const RANDOM_GUY_STR: ConstFormattedString<64> = RANDOM_GUY.format();
        const GREETING: &str = hformat!("Hello, {RANDOM_GUY_STR}!");
        const SAME_GREETING: &str = hformat!("Hello, {}!", const { RANDOM_GUY.format() });
        assert_eq!(GREETING, "Hello, Doe, John | Age: 40 | $32.0!");
        assert_eq!(GREETING, SAME_GREETING);
        let id = core::hint::black_box(0);
        assert_eq!(
            hformat!("#{id} - {RANDOM_GUY_STR}"),
            "#0 - Doe, John | Age: 40 | $32.0"
        );
    }
}
