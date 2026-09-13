pub trait Formatted {
    /// Needs to be exact or an upper bound.
    fn size(&self) -> usize;
    fn append(&self, buf: &mut String);
}

pub trait HybridFormat {
    type Formatted<'a>: Formatted
    where
        Self: 'a;
    fn format(&self) -> Self::Formatted<'_>;
}

#[doc(hidden)]
#[inline]
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
    use lexical_core::FormattedSize;

    impl Formatted for &str {
        #[inline]
        fn size(&self) -> usize {
            self.len()
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(self);
        }
    }
    impl HybridFormat for str {
        type Formatted<'a>
            = &'a str
        where
            Self: 'a;

        #[inline]
        fn format(&self) -> Self::Formatted<'_> {
            self
        }
    }
    impl HybridFormat for &str {
        type Formatted<'a>
            = &'a str
        where
            Self: 'a;

        #[inline]
        fn format(&self) -> Self::Formatted<'_> {
            self
        }
    }
    impl HybridFormat for String {
        type Formatted<'a>
            = &'a str
        where
            Self: 'a;

        #[inline]
        fn format(&self) -> Self::Formatted<'_> {
            self.as_str()
        }
    }
    impl HybridFormat for bool {
        type Formatted<'a>
            = &'static str
        where
            Self: 'a;

        #[inline]
        fn format(&self) -> Self::Formatted<'_> {
            if *self { "true" } else { "false" }
        }
    }

    #[repr(transparent)]
    #[doc(hidden)]
    pub struct FormattedChar(char);

    impl Formatted for FormattedChar {
        #[inline]
        fn size(&self) -> usize {
            self.0.len_utf8()
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push(self.0);
        }
    }
    impl HybridFormat for char {
        type Formatted<'a>
            = FormattedChar
        where
            Self: 'a;
        #[inline]
        fn format(&self) -> Self::Formatted<'_> {
            FormattedChar(*self)
        }
    }

    #[repr(transparent)]
    #[doc(hidden)]
    pub struct FormattedFloat<T>(T);

    impl Formatted for FormattedFloat<f64> {
        #[inline]
        fn size(&self) -> usize {
            24
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(zmij::Buffer::new().format(self.0));
        }
    }
    impl Formatted for FormattedFloat<f32> {
        #[inline]
        fn size(&self) -> usize {
            24
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(zmij::Buffer::new().format(self.0));
        }
    }
    impl HybridFormat for f64 {
        type Formatted<'a>
            = FormattedFloat<f64>
        where
            Self: 'a;
        fn format(&self) -> Self::Formatted<'_> {
            FormattedFloat(*self)
        }
    }
    impl HybridFormat for f32 {
        type Formatted<'a>
            = FormattedFloat<f32>
        where
            Self: 'a;
        fn format(&self) -> Self::Formatted<'_> {
            FormattedFloat(*self)
        }
    }

    #[repr(transparent)]
    #[doc(hidden)]
    pub struct FormattedInt<T>(T);

    macro_rules! HybridFormatInt {
        ($($t: ty )*) => {$(
            impl Formatted for FormattedInt<$t> {
                #[inline]
                fn size(&self) -> usize {
                    <$t>::FORMATTED_SIZE_DECIMAL
                }
                #[inline]
                fn append(&self, buf: &mut String) {
                    let mut buffer = [0u8; <$t>::FORMATTED_SIZE_DECIMAL];
                    let digits = lexical_core::write(self.0, &mut buffer);
                    buf.push_str(unsafe { str::from_utf8_unchecked(digits) });
                }
            }
            impl HybridFormat for $t {
                type Formatted<'a> = FormattedInt<$t> where Self: 'a;
                #[inline]
                fn format(&self) -> Self::Formatted<'_> {
                    FormattedInt(*self)
                }
            }
        )*
        };
    }

    HybridFormatInt!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);
}
