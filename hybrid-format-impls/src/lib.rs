//! Use the `hybrid-format` crate.

#![cfg_attr(not(test), no_std)]
extern crate alloc;
use alloc::string::String;

#[doc(hidden)]
pub mod const_args;

#[expect(clippy::missing_safety_doc)]
pub unsafe trait Formatted {
    /// The size of the object when formatted in bytes. Needs to be exact or at least an upper bound.
    fn size(&self) -> usize;
    /// Appends the formatted object to `buf`.
    /// # Safety
    /// This assumes `buf` still has at least [`Formatted::size()`] bytes of remaining capacity.
    unsafe fn append(&self, buf: &mut String);
}

pub trait HybridFormat {
    /// The intermediate representation, that actually gets written into the final formatted string.
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
        core::ptr::copy_nonoverlapping(
            string.as_ptr(),
            src.as_mut_vec().as_mut_ptr().add(len),
            string_len,
        );
        src.as_mut_vec().set_len(len + string_len);
    }
}

#[doc(hidden)]
pub mod __private {
    use crate::Formatted;
    use crate::HybridFormat;
    use crate::push_str_unchecked;
    use alloc::string::String;
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
        unsafe fn append(&self, buf: &mut String) {
            unsafe {
                let buf_vec = buf.as_mut_vec();
                let buf_len = buf_vec.len();
                debug_assert!(5 <= buf_vec.capacity() - buf_len);
                buf_vec
                    .as_mut_ptr()
                    .add(buf_len)
                    .cast::<[u8; 8]>()
                    .write(if *self {
                        *b"true\0\0\0\0"
                    } else {
                        *b"false\0\0\0"
                    });
                buf_vec.set_len(buf_len + 5 - usize::from(*self));
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
    unsafe impl Formatted for char {
        #[inline(always)]
        fn size(&self) -> usize {
            // this can overallocate, but really not by a lot, and it avoids cpu work
            4
        }
        #[inline]
        unsafe fn append(&self, buf: &mut String) {
            let buf_len = buf.len();
            debug_assert!(4 <= buf.capacity() - buf_len);
            unsafe {
                let char_len = self
                    .encode_utf8(core::slice::from_raw_parts_mut(
                        buf.as_mut_vec().as_mut_ptr().add(buf_len),
                        4,
                    ))
                    .len();
                buf.as_mut_vec().set_len(buf_len + char_len);
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
                    const MAX_SIZE: usize = <$t>::FORMATTED_SIZE_DECIMAL;
                    let buf_len = buf.len();
                    debug_assert!(MAX_SIZE <= buf.capacity() - buf_len);
                    unsafe {
                        let written_bytes = write_int(*self, core::slice::from_raw_parts_mut(buf.as_mut_vec().as_mut_ptr().add(buf_len), MAX_SIZE));
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
                    const MAX_SIZE: usize = <$u>::FORMATTED_SIZE_DECIMAL;
                    let buf_len = buf.len();
                    debug_assert!(MAX_SIZE < buf.capacity() - buf_len);
                    unsafe {
                        // asm (on arm64) output shows that this is the smallest and fastest option
                        let buf_ptr = buf.as_mut_vec().as_mut_ptr().add(buf_len);
                        buf_ptr.write(b'-');
                        let is_int_neg = usize::from(*self < 0);
                        let written_bytes = write_int(self.unsigned_abs(), core::slice::from_raw_parts_mut(buf_ptr.add(is_int_neg), MAX_SIZE));
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
