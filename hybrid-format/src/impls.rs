use crate::Formatted;
use crate::HybridFormat;
use alloc::string::String;
use lexical_core::FormattedSize;

#[inline]
/// Pushes `string` into `src` without checking capacity.
/// # Safety
/// Just make sure `src` has at least `string.len()` bytes of remaining capacity.
pub const unsafe fn push_str_unchecked(dst: *mut u8, string: &str) -> *mut u8 {
    let string_len = string.len();
    unsafe {
        core::ptr::copy_nonoverlapping(string.as_ptr(), dst, string_len);
        dst.add(string_len)
    }
}

unsafe impl Formatted for &str {
    #[inline(always)]
    fn size(&self) -> usize {
        self.len()
    }
    #[inline]
    unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
        unsafe { push_str_unchecked(buf, self) }
    }
}
impl HybridFormat for str {
    type Formatted<'a>
        = &'a Self
    where
        Self: 'a;

    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        self
    }
}
impl<T: HybridFormat + ?Sized> HybridFormat for &T {
    type Formatted<'a>
        = T::Formatted<'a>
    where
        Self: 'a;

    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        (**self).format()
    }
}
impl HybridFormat for String {
    type Formatted<'a>
        = &'a str
    where
        Self: 'a;

    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        self.as_str()
    }
}
unsafe impl Formatted for bool {
    #[inline(always)]
    fn size(&self) -> usize {
        // this can only overallocate by four bytes so it's worth it and avoids extra work
        8
    }
    #[inline]
    unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            buf.cast::<[u8; 8]>().write(if *self {
                *b"true\0\0\0\0"
            } else {
                *b"false\0\0\0"
            });
            buf.add(5 - usize::from(*self))
        }
    }
}
impl HybridFormat for bool {
    type Formatted<'a>
        = Self
    where
        Self: 'a;

    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        *self
    }
}
#[inline(never)]
// takes in a u32 because chars trigger a warning
const unsafe extern "C" fn write_char(buf: *mut u8, c: u32) -> usize {
    unsafe {
        char::from_u32_unchecked(c)
            .encode_utf8(core::slice::from_raw_parts_mut(buf, 4))
            .len()
    }
}
unsafe impl Formatted for char {
    #[inline(always)]
    fn size(&self) -> usize {
        // this can overallocate, but really not by a lot, and it avoids cpu work
        4
    }
    #[inline]
    unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            let char_len = if self.is_ascii() {
                buf.write(*self as u8);
                1
            } else {
                write_char(buf, u32::from(*self))
            };
            buf.add(char_len)
        }
    }
}
impl HybridFormat for char {
    type Formatted<'a>
        = Self
    where
        Self: 'a;
    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        *self
    }
}

macro_rules! HybridFormatFloat {
    ($($t: ty )*) => {$(
        unsafe impl Formatted for $t {
            #[inline(always)]
            fn size(&self) -> usize {
                24
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                #[inline(never)]
                // extern "C" removes the unwind cleanup code, it's the only way I found
                unsafe extern "C" fn write_float(f: $t, buf: *mut u8) -> usize {
                    if f.is_finite() {
                        unsafe {(*buf.cast::<zmij::Buffer>()).format_finite(f).len()}
                    } else {
                        core::hint::cold_path();
                        let (bytes, len) = if f.is_nan() {
                            (*b"NaN\0",3)
                        } else if f.is_sign_negative() {
                            (*b"-inf",4)
                        } else {
                            (*b"inf\0",3)
                        };
                        unsafe {buf.cast::<[u8;4]>().write(bytes)};
                        len
                    }
                }
                unsafe { buf.add(write_float(*self, buf)) }
            }
        }
    )*
    };
}

HybridFormatFloat!(f32 f64);

impl HybridFormat for f64 {
    type Formatted<'a>
        = Self
    where
        Self: 'a;
    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        *self
    }
}
impl HybridFormat for f32 {
    type Formatted<'a>
        = Self
    where
        Self: 'a;
    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        *self
    }
}

#[inline(never)]
unsafe fn write_int<N: lexical_core::ToLexical>(buf: *mut u8, n: N) -> usize {
    lexical_core::write(n, unsafe {
        core::slice::from_raw_parts_mut(buf, N::FORMATTED_SIZE_DECIMAL)
    })
    .len()
}

macro_rules! HybridFormatIntUnsigned {
    ($($t: ty )*) => {$(
        unsafe impl Formatted for $t {
            #[inline(always)]
            fn size(&self) -> usize {
                <$t>::FORMATTED_SIZE_DECIMAL
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                unsafe { buf.add(write_int(buf, *self)) }
            }
        }
        impl HybridFormat for $t {
            type Formatted<'a> = $t where Self: 'a;
            #[inline(always)]
            fn format(&self) -> Self::Formatted<'_> {
                *self
            }
        }
    )*
    };
}

macro_rules! HybridFormatIntSigned {
    ($({$t:ty, $u:ty})*) => {$(
        unsafe impl Formatted for $t {
            #[inline(always)]
            fn size(&self) -> usize {
                // fixes some sizes
                const {<$u>::FORMATTED_SIZE_DECIMAL + 1}
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                #[inline(never)]
                unsafe extern "C" fn write_signed(buf: *mut u8, n: $t) -> usize {
                    unsafe {
                        // asm (on arm64) output shows that this is the smallest and fastest option
                        buf.write(b'-');
                        let is_int_neg = usize::from(n < 0);
                        lexical_core::write(n.unsigned_abs(), core::slice::from_raw_parts_mut(buf.add(is_int_neg), <$u>::FORMATTED_SIZE_DECIMAL)).len() + is_int_neg
                    }
                }
                unsafe { buf.add(write_signed(buf, *self)) }
            }
        }
        impl HybridFormat for $t {
            type Formatted<'a> = $t where Self: 'a;
            #[inline(always)]
            fn format(&self) -> Self::Formatted<'_> {
                *self
            }
        }
    )*
    };
}

HybridFormatIntUnsigned!(u8 u16 u32 u64 u128 usize);
HybridFormatIntSigned!({i8,u8} {i16,u16} {i32,u32} {i64,u64} {i128,u128} {isize,usize});
