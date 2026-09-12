pub use hybrid_format_macros::hformat;

#[doc(hidden)]
pub mod __private {
    pub use hybrid_format_impls::*;
}

#[cfg(test)]
mod tests {
    use hybrid_format_macros::hformat;

    #[test]
    fn test() {
        let z = 9;
        let x = hformat!("{}", 7);
        assert_eq!(x, "9");
    }
}
