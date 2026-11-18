#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", env!("CARGO_PKG_README")))]
#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod const_args;
mod impls;

pub use const_args::ConstFormattedString;

#[derive(Clone, Copy)]
/// Text alignment. Borrowed from [`core::fmt::Alignment`].
pub enum Alignment {
    /// Indication that contents should be left-aligned.
    Left,
    /// Indication that contents should be right-aligned.
    Center,
    /// Indication that contents should be center-aligned.
    Right,
}

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
///     unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
///         unsafe {
///             let buf = '('.append(buf);
///             let buf = self.x.append(buf);
///             let buf = ", ".append(buf);
///             let buf = self.y.append(buf);
///             ')'.append(buf)
///         }
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
    /// The default alignment. This is overwritten by integers and floats.
    const DEFAULT_ALIGNMENT: Alignment = Alignment::Left;
    /// The size of the object when formatted in bytes. Needs to be exact or at least an upper bound.
    fn size(&self) -> usize;
    /// Writes the formatted object to `buf` and returns the pointer past the last byte written.
    /// # Safety
    /// `buf` must be valid for [`Formatted::size()`] bytes of writes.
    unsafe fn append(&self, buf: *mut u8) -> *mut u8;
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

/// `hformat!` is a [`format!`](https://doc.rust-lang.org/std/macro.format.html) alternative that formats constants at compile time. It's **3x faster** with runtime arguments, **8x faster** with mixed arguments, and there's **no runtime formatting** for constants ([benchmarks](crate#benchmarks)).
/// ```rust
/// use hybrid_format::{hformat, hwrite};
///
/// const NUMBER: f64 = 42.0;
/// const fn something() -> &'static str { "42" }
///
/// // Constants are always formatted at compile time.
/// // They are detected automatically, but if detection misses,
/// // the placeholder prefix `@` forces compile-time formatting.
///
/// // Constants only -> a `&'static str`.
/// const ANSWER: &str = hformat!("{NUMBER} == the answer == {@something()}");
///
/// // Mixed -> a String built in a single allocation.
/// // Here, only `my_name` is formatted at runtime.
/// let my_name = "Horace";
/// let message = hformat!("Hello {my_name}. {NUMBER}!");
///
/// // `hwrite!` appends to an existing Stirng.
/// let mut answers = String::new();
/// hwrite!(answers, "0: {ANSWER}");
/// ```
/// Adjacent constants and literal text are merged at compile time, so each "constant run" costs a single string copy. Please [report any constant detection false positives](https://github.com/horacehoff/hybrid-format/issues).
///
/// The [format spec](https://doc.rust-lang.org/std/fmt/index.html) (fill, align, precision, ...) is in the process of being implemented.
/// Currently, only padding (width, fill, and alignment) is available.
///
/// The following types can be formatted both at compile time and runtime (the implementations are very optimized) and you can [add your own](#custom-types):
/// - `&str`
/// - `bool`
/// - `char`
/// - `f32`/`f64` (whole-number floats keep a trailing `.0` and extreme values are formatted like `4.2e+67`)
/// - `i8`,`i16`,`i32`,`i64`,`i128`,`isize`,`u8`,`u16`,`u32`,`u64`,`u128`,`usize`
#[macro_export]
macro_rules! hformat {
    ($($args: tt)*) => {{
        use $crate as __hitchhikers_guide_to_hybrid_format;
        $crate::__private::hformat!($($args)*)
    }};
}

/// [`hformat!`] equivalent of the [`write!`] macro. Appends to an existing `String` and reserves the exact capacity once.
#[macro_export]
macro_rules! hwrite {
    ($buf:expr, $($args: tt)*) => {{
        use $crate as __hitchhikers_guide_to_hybrid_format;
        $crate::__private::hformat!(($buf) $($args)*)
    }};
}

#[doc(hidden)]
pub mod __private {
    pub use crate::const_args::{HybridFormatConstArg, const_format_padding, default_alignment};
    pub use crate::impls::{Padded, push_str_unchecked};
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
                $buffer = unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
                $buffer = unsafe {$crate::Formatted::append(&_dynamic_formatted, $buffer)};
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
        let mut string = $crate::__private::String::with_capacity($($capacity_expr)* + _temp_formatted.len());
        let mut $buffer = unsafe { string.as_mut_vec() }.as_mut_ptr();
        $($add_to_str_statements)*
        $buffer = unsafe { $crate::__private::push_str_unchecked($buffer, _temp_formatted) };
        let len = unsafe { $buffer.offset_from_unsigned(string.as_ptr()) };
        unsafe { string.as_mut_vec().set_len(len) }
        string
    }};
    // hwrite! => appends to existing String
    ([$buffer:ident ($buf_tgt:expr)] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*]) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        use ::core::borrow::BorrowMut as _;
        match $buf_tgt.borrow_mut() {
            string => {
                let string: &mut $crate::__private::String = string;
                string.reserve($($capacity_expr)* + _temp_formatted.len());
                let mut $buffer = unsafe { string.as_mut_vec().as_mut_ptr().add(string.len()) };
                $($add_to_str_statements)*
                $buffer = unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
                let len = unsafe { $buffer.offset_from_unsigned(string.as_ptr()) };
                unsafe { string.as_mut_vec().set_len(len) }
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
