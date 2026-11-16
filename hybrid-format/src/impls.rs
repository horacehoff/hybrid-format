use crate::Alignment;
use crate::Formatted;
use crate::HybridFormat;
use alloc::string::String;
use lexical_core::FormattedSize;

#[doc(hidden)]
pub struct Padded<T> {
    pub value: T,
    pub width: usize,
    pub fill: char,
    pub alignment: Option<Alignment>,
}

impl<T: HybridFormat> HybridFormat for Padded<T> {
    type Formatted<'a>
        = Padded<T::Formatted<'a>>
    where
        Self: 'a;
    #[inline(always)]
    fn format(&self) -> Self::Formatted<'_> {
        Padded {
            value: self.value.format(),
            width: self.width,
            fill: self.fill,
            alignment: self.alignment,
        }
    }
}

unsafe impl<T: Formatted> Formatted for Padded<T> {
    #[inline(always)]
    fn size(&self) -> usize {
        self.value.size() + (self.width * self.fill.len_utf8())
    }
    #[inline(always)]
    unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            format_padding(
                buf,
                self.value.append(buf),
                self.width,
                self.fill,
                self.alignment.unwrap_or(T::DEFAULT_ALIGNMENT),
            )
        }
    }
}

#[inline(never)]
unsafe fn format_padding(
    value_start: *mut u8,
    value_end: *mut u8,
    width: usize,
    fill_char: char,
    alignment: Alignment,
) -> *mut u8 {
    unsafe {
        let value_len = str::from_utf8_unchecked(core::slice::from_raw_parts(
            value_start,
            value_end.offset_from_unsigned(value_start),
        ))
        .chars()
        .count();
        let pad = width.saturating_sub(value_len);
        let fill_char_len = fill_char.len_utf8();
        let padded_end = if fill_char.is_ascii() {
            value_end.write_bytes(fill_char as u8, pad);
            value_end.add(pad)
        } else {
            let mut fill = [0u8; 4];
            fill_char.encode_utf8(&mut fill);
            (0..pad).fold(value_end, |dst, _| {
                dst.cast::<[u8; 4]>().write(fill);
                dst.add(fill_char_len)
            })
        };
        // Moves all(right)/half(center) of the padding chars to the start (left alignment doesn't do anything)
        core::slice::from_raw_parts_mut(value_start, padded_end.offset_from_unsigned(value_start))
            .rotate_right(
                fill_char_len
                    * match alignment {
                        Alignment::Left => 0,
                        Alignment::Center => pad / 2,
                        Alignment::Right => pad,
                    },
            );
        padded_end
    }
}

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
        let len = self.len();
        unsafe { core::hint::assert_unchecked(len <= isize::MAX.cast_unsigned()) }; // drops the zero-capacity and overflow branches (in only two cases)
        len
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
        static BOOL_STR: [[u8; 8]; 2] = [*b"false\0\0\0", *b"true\0\0\0\0"];
        unsafe {
            let bool_value = usize::from(*self);
            buf.cast::<[u8; 8]>().write(BOOL_STR[bool_value]);
            buf.add(5 - bool_value)
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
const unsafe extern "C" fn write_char(buf: *mut u8, c: u32) -> *mut u8 {
    unsafe {
        let len =
            char::from_u32_unchecked(c).encode_utf8(core::slice::from_raw_parts_mut(buf, 4)).len();
        buf.add(len)
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
            if self.is_ascii() {
                buf.write(*self as u8);
                buf.add(1)
            } else {
                write_char(buf, u32::from(*self))
            }
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
            const DEFAULT_ALIGNMENT: Alignment = Alignment::Right;
            #[inline(always)]
            fn size(&self) -> usize {
                24
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                #[inline(never)]
                // extern "C" removes the unwind cleanup code, it's the only way I found
                unsafe extern "C" fn write_float(f: $t, buf: *mut u8) -> *mut u8 {
                    if f.is_finite() {
                        unsafe { buf.add((*buf.cast::<zmij::Buffer>()).format_finite(f).len()) }
                    } else {
                        core::hint::cold_path();
                        let (bytes, len) = if f.is_nan() {
                            (*b"NaN\0",3)
                        } else if f.is_sign_positive() {
                            (*b"inf\0",3)
                        } else {
                            (*b"-inf",4)
                        };
                        unsafe {
                            buf.cast::<[u8;4]>().write(bytes);
                            buf.add(len)
                        }
                    }
                }
                unsafe { write_float(*self, buf) }
            }
        }
        impl HybridFormat for $t {
            type Formatted<'a>
                = Self
            where
                Self: 'a;
            #[inline(always)]
            fn format(&self) -> Self::Formatted<'_> {
                *self
            }
        }
    )*
    };
}

HybridFormatFloat!(f32 f64);

#[inline(never)]
unsafe extern "C" fn write_int<N: lexical_core::ToLexical>(buf: *mut u8, n: N) -> *mut u8 {
    unsafe {
        buf.add(
            lexical_core::write(n, core::slice::from_raw_parts_mut(buf, N::FORMATTED_SIZE_DECIMAL))
                .len(),
        )
    }
}

macro_rules! HybridFormatIntUnsigned {
    ($($t: ty )*) => {$(
        unsafe impl Formatted for $t {
            const DEFAULT_ALIGNMENT: Alignment = Alignment::Right;
            #[inline(always)]
            fn size(&self) -> usize {
                <$t>::FORMATTED_SIZE_DECIMAL
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                unsafe { write_int(buf, *self) }
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
            const DEFAULT_ALIGNMENT: Alignment = Alignment::Right;
            #[inline(always)]
            fn size(&self) -> usize {
                // fixes some sizes
                const {<$u>::FORMATTED_SIZE_DECIMAL + 1}
            }
            #[inline]
            unsafe fn append(&self, buf: *mut u8) -> *mut u8 {
                #[inline(never)]
                unsafe extern "C" fn write_signed(buf: *mut u8, n: $t) -> *mut u8 {
                    unsafe {
                        // asm (on arm64) output shows that this is the smallest and fastest option
                        buf.write(b'-');
                        let int_start = buf.add(usize::from(n < 0));
                        int_start.add(
                            lexical_core::write(n.unsigned_abs(), core::slice::from_raw_parts_mut(int_start, <$u>::FORMATTED_SIZE_DECIMAL)).len()
                        )
                    }
                }
                unsafe { write_signed(buf, *self) }
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
