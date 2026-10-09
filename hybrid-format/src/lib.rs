#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", env!("CARGO_PKG_README")))]
#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod const_args;
mod impls;

pub use const_args::ConstFormattedString;

/// A not-yet-formatted value, that knows its formatted size, and can format(write) itself into a buffer without any reallocations.
///
/// # Safety
/// It is up to you to make sure that [`Formatted::size()`] returns the exact or maximum size of your object when formatted (in bytes)!
/// [`Formatted::append()`] uses this assumption to skip checks and avoid reallocation.
///
/// # Example
/// ```
/// use hybrid_format::{hformat, Formatted, HybridFormat};
///
/// #[derive(Clone, Copy)]
/// struct Point {
///     x: u8,
///     y: u8
/// }
/// unsafe impl Formatted for Point {
///     fn size(&self) -> usize {
///         10
///     }
///     unsafe fn append(&self, buf: &mut String) {
///         buf.push('(');
///         unsafe { self.x.append(buf) };
///         buf.push_str(", ");
///         unsafe { self.y.append(buf) };
///         buf.push(')');
///     }
/// }
/// impl HybridFormat for Point {
///     type Formatted<'a> = Self;
///     fn format(&self) -> Self {
///         *self
///     }
/// }
///
/// let p = Point { x: 42, y: 67 };
/// assert_eq!(hformat!("p = {p}"), "p = (42, 67)")
/// ```
pub unsafe trait Formatted {
    /// The size of the object when formatted in bytes. Needs to be exact or at least an upper bound.
    fn size(&self) -> usize;
    /// Appends the formatted object to `buf`.
    /// # Safety
    /// This assumes `buf` still has at least [`Formatted::size()`] bytes of remaining capacity.
    unsafe fn append(&self, buf: &mut alloc::string::String);
}

/// Converts an argument into an intermediate representation (that can be borrowed) that implements [`Formatted`], that can then be formatted.
/// # Example
/// ```
/// use hybrid_format::{hformat, Formatted, HybridFormat};
/// struct Line2D {
///     slope: f64,
///     intercept: f64,
/// }
/// impl HybridFormat for Line2D {
///     type Formatted<'a>
///         = f64
///     where
///         Self: 'a;
///     fn format(&self) -> Self::Formatted<'_> {
///         self.slope
///     }
/// }
/// let my_line = Line2D { slope: 3.14, intercept: 0.0 };
/// assert_eq!(hformat!("The line's slope is {my_line}."), "The line's slope is 3.14.");
/// ```
pub trait HybridFormat {
    /// The intermediate representation, that actually gets written into the final formatted string.
    type Formatted<'a>: Formatted
    where
        Self: 'a;
    /// Formats the object into an intermediate representation, that can be borrowed.
    fn format(&self) -> Self::Formatted<'_>;
}

/// Formats a string like [`format!`](https://doc.rust-lang.org/std/macro.format.html), but faster.
///
/// Constant arguments (literals, `const` blocks, `SCREAMING_SNAKE_CASE` names) are formatted at compile time.
/// If every argument is a constant, the macro outputs a `&'static str`, otherwise it outputs a `String` built with a single allocation.
///
/// Format specs (fill, align, precision, ...) aren't supported yet.
///
/// The following types can be formatted both at compile-time and runtime (the implementations are very optimized):
/// - `&str`
/// - `bool`
/// - `char`
/// - `f32`/`f64`
/// - `i8`,`i16`,`i32`,`i64`,`i128`,`isize`,`u8`,`u16`,`u32`,`u64`,`u128`,`usize`
///
/// # Examples
/// ```
/// use hybrid_format::hformat;
///
/// assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42)); // => "42"
///
/// const MY_INT: i32 = 42;
/// assert_eq!(const { hformat!("{}", MY_INT) }, format!("{}", MY_INT)); // => "42"
///
/// let i = 2 + 2;
/// assert_eq!(hformat!("{}", i), format!("{}", i)); // => "4"
///
/// assert_eq!(
///     const { hformat!("Float: {}", 4.2) }, // => "Float: 4.2"
///     format!("Float: {}", 4.2)
/// );
///
/// const MY_FLOAT: f64 = 4.2;
/// assert_eq!(
///     const { hformat!("Float: {f}", f = MY_FLOAT) }, // => "Float: 4.2"
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
#[macro_export]
macro_rules! hformat {
    ($($args: tt)*) => {{
        use $crate as __hitchhikers_guide_to_hybrid_format;
        $crate::__private::hformat!($($args)*)
    }};
}

/// [`hformat!`] equivalent of the [`write!`] macro.
#[macro_export]
macro_rules! hwrite {
    ($buf:expr, $($args: tt)*) => {{
        use $crate as __hitchhikers_guide_to_hybrid_format;
        $crate::__private::hformat!(($buf) $($args)*)
    }};
}

#[doc(hidden)]
pub mod __private {
    pub use crate::const_args::HybridFormatConstArg;
    pub use crate::impls::push_str_unchecked;
    pub use alloc::string::String;
    pub use const_format;
    pub use hybrid_format_macros::hformat;
}

#[macro_export]
#[doc(hidden)]
macro_rules! __hformat_internal {
    // a dynamic (runtime) item
    ([$buffer:ident $($buf_tgt:tt)*] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr} $(, $($remaining:tt)*)?) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let _dynamic_elem = &$dynamic_elem;
        let _dynamic_formatted = $crate::HybridFormat::format(_dynamic_elem);
        $crate::__hformat_internal!(
            [$buffer $($buf_tgt)*]
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
    ([$buffer:ident $($buf_tgt:tt)*] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr $(, $($remaining:tt)*)?) => {{
        $crate::__hformat_internal!(
            [$buffer $($buf_tgt)*]
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
    // hwrite! => appends to existing String
    ([$buffer:ident ($buf_tgt:expr)] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*]) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        use ::core::borrow::BorrowMut as _;
        match $buf_tgt.borrow_mut() {
            $buffer => {
                let $buffer: &mut $crate::__private::String = $buffer;
                $buffer.reserve($($capacity_expr)* + _temp_formatted.len());
                $($add_to_str_statements)*
                unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
            }
        }
    }};
    // hwrite!()
    (($buf_tgt: expr) $($elems: tt)*) => {
        $crate::__hformat_internal!(
            [buf ($buf_tgt)]
            []
            [0usize]
            []
            $($elems)*
        )
    };
    // hformat!()
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
mod tests;
