/// The compile-time counterpart of the `HybridFormat` trait, used by `hformat!` for constant arguments.
///
/// It's a struct because const traits aren't stable yet.
/// Two functions are used instead of a single one because a const function can't return a 'static reference.
pub struct HybridFormatConstArg<T>(pub T);

/// A compile-time formatted float.
pub struct FormattedFloat {
    bytes: [u8; 24],
    size: usize,
}

impl FormattedFloat {
    #[must_use]
    #[inline]
    pub const fn as_const_arg(&self) -> &str {
        // SAFETY: `self.bytes[..self.size]` was copied directly from the string returned by const_zmij, so it's known to be valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.bytes.split_at(self.size).0) }
    }
}

macro_rules! HybridFormatConstArgFloat {
    ($($t: ty )*) => {$(
        impl HybridFormatConstArg<$t> {
            #[inline]
            pub const fn format_const(self) -> FormattedFloat {
                let mut buffer = const_zmij::Buffer::new();
                let s = const_zmij::Format(&mut buffer, self.0).call_once();
                let s_len = s.len();
                let mut bytes = [0u8; 24];
                bytes.split_at_mut(s_len).0.copy_from_slice(s.as_bytes());
                FormattedFloat {
                    bytes,
                    size: s_len,
                }
            }
        }
    )*};
}

HybridFormatConstArgFloat!(f32 f64);

impl<T: Copy> HybridFormatConstArg<T> {
    #[must_use]
    #[inline(always)]
    pub const fn as_const_arg(&self) -> T {
        self.0
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
