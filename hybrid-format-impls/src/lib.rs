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

#[doc(hidden)]
#[inline]
/// Pushes `string` into `src` without checking capacity.
/// # Safety
/// Just make sure `src` has at least `string.len()` bytes of remaining capacity.
pub unsafe fn push_str_unchecked(src: &mut String, string: &str) {
    let len = src.len();
    let string_len = string.len();
    debug_assert!(string_len <= src.capacity() - len);
    unsafe {
        std::ptr::copy_nonoverlapping(string.as_ptr(), src.as_mut_ptr().add(len), string_len);
        src.as_mut_vec().set_len(len + string_len);
    }
}

#[doc(hidden)]
pub mod __private {
    use crate::Formatted;
    use crate::HybridFormat;
    use crate::push_str_unchecked;
    use lexical_core::FormattedSize;

    unsafe impl Formatted for &str {
        #[inline(always)]
        fn size(&self) -> usize {
            self.len()
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            unsafe { push_str_unchecked(buf, self) }
        }
    }
    impl HybridFormat for str {
        type Formatted<'a>
            = &'a str
        where
            Self: 'a;

        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            self
        }
    }
    impl HybridFormat for &str {
        type Formatted<'a>
            = &'a str
        where
            Self: 'a;

        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            self
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
            // this can only overallocate by one byte so it's worth it and avoids extra work
            5
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            let buf_len = buf.len();
            debug_assert!(5 <= buf.capacity() - buf_len);
            unsafe {
                // smol micro-optimization trick
                std::ptr::copy_nonoverlapping(
                    if *self {
                        b"true".as_ptr()
                    } else {
                        b"fals".as_ptr()
                    },
                    buf.as_mut_ptr().add(buf_len),
                    4,
                );
                std::ptr::copy_nonoverlapping("e".as_ptr(), buf.as_mut_ptr().add(buf_len + 4), 1);
                buf.as_mut_vec().set_len(buf_len + 5 - (*self as usize));
            }
        }
    }
    impl HybridFormat for bool {
        type Formatted<'a>
            = bool
        where
            Self: 'a;

        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            *self
        }
    }
    unsafe impl Formatted for char {
        #[inline(always)]
        fn size(&self) -> usize {
            // this can overallocate, but really not by a lot, and it avoids cpu work
            4
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            let mut temp_char_buf = [0u8; 4];
            let char_len = self.encode_utf8(&mut temp_char_buf).len();
            let buf_len = buf.len();
            debug_assert!(4 <= buf.capacity() - buf_len);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    temp_char_buf.as_ptr(),
                    buf.as_mut_ptr().add(buf_len),
                    4,
                );
                buf.as_mut_vec().set_len(buf_len + char_len);
            }
        }
    }
    impl HybridFormat for char {
        type Formatted<'a>
            = char
        where
            Self: 'a;
        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            *self
        }
    }

    unsafe impl Formatted for f64 {
        #[inline(always)]
        fn size(&self) -> usize {
            24
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            unsafe { push_str_unchecked(buf, zmij::Buffer::new().format(*self)) }
        }
    }
    unsafe impl Formatted for f32 {
        #[inline(always)]
        fn size(&self) -> usize {
            24
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            unsafe { push_str_unchecked(buf, zmij::Buffer::new().format(*self)) }
        }
    }
    impl HybridFormat for f64 {
        type Formatted<'a>
            = f64
        where
            Self: 'a;
        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            *self
        }
    }
    impl HybridFormat for f32 {
        type Formatted<'a>
            = f32
        where
            Self: 'a;
        #[inline(always)]
        fn format(&self) -> Self::Formatted<'_> {
            *self
        }
    }

    #[inline(never)]
    fn write_int<N: lexical_core::ToLexical>(n: N, buf: &mut [u8]) -> usize {
        lexical_core::write(n, buf).len()
    }

    macro_rules! HybridFormatIntUnsigned {
        ($($t: ty )*) => {$(
            unsafe impl Formatted for $t {
                #[inline(always)]
                fn size(&self) -> usize {
                    <$t>::FORMATTED_SIZE_DECIMAL
                }
                #[inline]
                unsafe fn append(&self, buf: &mut String) {
                    let buf_len = buf.len();
                    let max_size = <$t>::FORMATTED_SIZE_DECIMAL;
                    debug_assert!(max_size <= buf.capacity() - buf_len);
                    unsafe {
                        let written_bytes = write_int(*self, std::slice::from_raw_parts_mut(buf.as_mut_ptr().add(buf_len), max_size));
                        buf.as_mut_vec().set_len(buf_len + written_bytes);
                    }
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
                unsafe fn append(&self, buf: &mut String) {
                    let buf_len = buf.len();
                    let max_size = <$u>::FORMATTED_SIZE_DECIMAL;
                    debug_assert!(max_size+1 <= buf.capacity() - buf_len);
                    unsafe {
                        // asm (on arm64) output shows that this is the smallest and fastest option
                        let buf_ptr = buf.as_mut_ptr().add(buf_len);
                        buf_ptr.write(b'-');
                        let is_int_neg = (*self < 0) as usize;
                        let written_bytes = write_int(self.unsigned_abs(), std::slice::from_raw_parts_mut(buf_ptr.add(is_int_neg), max_size));
                        buf.as_mut_vec().set_len(buf_len + written_bytes + is_int_neg);
                    }
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
}
