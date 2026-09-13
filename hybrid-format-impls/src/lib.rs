pub trait HybridFormat {
    fn formatted_size(&self) -> usize;
    fn append(&self, buf: &mut String);
}

#[doc(hidden)]
pub mod __private {
    use crate::HybridFormat;
    use lexical_core::FormattedSize;

    impl HybridFormat for str {
        #[inline]
        fn formatted_size(&self) -> usize {
            self.len()
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(self);
        }
    }
    impl HybridFormat for String {
        #[inline]
        fn formatted_size(&self) -> usize {
            self.len()
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(self);
        }
    }
    impl HybridFormat for bool {
        #[inline]
        fn formatted_size(&self) -> usize {
            5
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(if *self { "true" } else { "false" });
        }
    }
    impl HybridFormat for f32 {
        #[inline]
        fn formatted_size(&self) -> usize {
            24
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(zmij::Buffer::new().format(*self));
        }
    }
    impl HybridFormat for f64 {
        #[inline]
        fn formatted_size(&self) -> usize {
            24
        }
        #[inline]
        fn append(&self, buf: &mut String) {
            buf.push_str(zmij::Buffer::new().format(*self));
        }
    }
    macro_rules! HybridFormatInt {
        ($($t: ty )*) => {$(
            impl HybridFormat for $t {
                #[inline]
                fn formatted_size(&self) -> usize {
                    Self::FORMATTED_SIZE_DECIMAL
                }
                #[inline]
                fn append(&self, buf: &mut String) {
                    let mut buffer = [0u8; Self::FORMATTED_SIZE_DECIMAL];
                    let digits = lexical_core::write(*self, &mut buffer);
                    buf.push_str(unsafe { str::from_utf8_unchecked(digits) });
                }
            })*
        };
    }
    HybridFormatInt!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);
}
