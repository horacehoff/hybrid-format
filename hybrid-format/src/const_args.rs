/// The compile-time counterpart of the `HybridFormat` trait, used by `hformat!` for constant arguments.
///
/// It's a struct because const traits aren't stable yet.
/// Two functions are used instead of a single one because a const function can't return a 'static reference.
#[doc(hidden)]
pub struct HybridFormatConstArg<T>(pub T);

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
pub struct ConstFormattedString<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<T: Copy> HybridFormatConstArg<T> {
    #[must_use]
    #[inline(always)]
    pub const fn as_const_arg(self) -> T {
        self.0
    }
}

impl<const N: usize> Default for ConstFormattedString<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! push_int_impl {
    ($($name: ident($t: ty) => $base: ident),* $(,)?) => {$(
        #[must_use]
        #[inline]
        pub const fn $name(self, n: $t) -> Self {
            self.$base(n as _)
        }
    )*};
}

impl<const N: usize> ConstFormattedString<N> {
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }
    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[must_use]
    #[inline]
    /// Returns the string built so far.
    pub const fn as_str(&self) -> &str {
        // SAFETY: This is always valid UTF-8, because `buf[..len]` is only ever written by `push_str` and `write_u128`.
        unsafe { str::from_utf8_unchecked(self.buf.split_at(self.len).0) }
    }
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn as_const_arg(&'static self) -> &'static str {
        self.as_str()
    }
    #[must_use]
    #[inline]
    /// # Panics
    /// This will panic if `self.len() + string.len() > N`.
    pub const fn push_str(mut self, string: &str) -> Self {
        let bytes = string.as_bytes();
        let bytes_len = bytes.len();
        assert!(
            bytes_len <= N - self.len,
            "Capacity exceeded when pushing &str"
        );
        self.buf
            .split_at_mut(self.len)
            .1
            .split_at_mut(bytes_len)
            .0
            .copy_from_slice(bytes);
        self.len += bytes_len;
        self
    }
    #[must_use]
    #[inline]
    pub const fn push_char(self, c: char) -> Self {
        self.push_str(c.encode_utf8(&mut [0u8; 4]))
    }
    #[must_use]
    #[inline]
    pub const fn push_bool(self, b: bool) -> Self {
        self.push_str(if b { "true" } else { "false" })
    }
    #[must_use]
    #[inline]
    pub const fn push_f64(self, f: f64) -> Self {
        let mut buffer = const_zmij::Buffer::new();
        self.push_str(const_zmij::Format(&mut buffer, f).call_once())
    }
    #[must_use]
    #[inline]
    pub const fn push_f32(self, f: f32) -> Self {
        let mut buffer = const_zmij::Buffer::new();
        self.push_str(const_zmij::Format(&mut buffer, f).call_once())
    }
    #[inline]
    const fn write_u128(&mut self, n: u128) {
        if n >= 10 {
            self.write_u128(n / 10);
        }
        assert!(self.len < N, "Capacity exceeded when pushing int");
        self.buf[self.len] = b'0' + (n % 10) as u8;
        self.len += 1;
    }
    #[must_use]
    #[inline]
    pub const fn push_u128(mut self, n: u128) -> Self {
        self.write_u128(n);
        self
    }
    #[must_use]
    #[inline]
    pub const fn push_i128(self, n: i128) -> Self {
        if n < 0 { self.push_char('-') } else { self }.push_u128(n.unsigned_abs())
    }
    push_int_impl! {
        push_u8(u8) => push_u128,
        push_u16(u16) => push_u128,
        push_u32(u32) => push_u128,
        push_u64(u64) => push_u128,
        push_usize(usize) => push_u128,
        push_i8(i8) => push_i128,
        push_i16(i16) => push_i128,
        push_i32(i32) => push_i128,
        push_i64(i64) => push_i128,
        push_isize(isize) => push_i128,
    }
}

impl<const N: usize> HybridFormatConstArg<ConstFormattedString<N>> {
    #[must_use]
    #[inline]
    pub const fn format_const(self) -> ConstFormattedString<N> {
        self.0
    }
}

impl HybridFormatConstArg<f32> {
    #[must_use]
    #[inline]
    pub const fn format_const(self) -> ConstFormattedString<24> {
        ConstFormattedString::new().push_f32(self.0)
    }
}
impl HybridFormatConstArg<f64> {
    #[must_use]
    #[inline]
    pub const fn format_const(self) -> ConstFormattedString<24> {
        ConstFormattedString::new().push_f64(self.0)
    }
}

macro_rules! HybridFormatConstArgPassthrough {
    ($($t: ty )*) => {$(
        impl HybridFormatConstArg<$t> {
            #[must_use]
            #[inline(always)]
            pub const fn format_const(self) -> Self {
                self
            }
        }
    )*};
}

HybridFormatConstArgPassthrough!(&'static str bool char u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
