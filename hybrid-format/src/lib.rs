pub use hybrid_format_macros::hformat;

#[doc(hidden)]
pub mod __private {
    pub use const_format;
    pub use hybrid_format_impls::__private::*;
}

#[cfg(test)]
mod tests {
    use hybrid_format_macros::hformat;

    #[test]
    fn test() {
        const Z: i32 = 9;
        const IDK: &str = hformat!("{}", const { Z });
        assert_eq!(IDK, "9");
    }
}
