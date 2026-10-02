pub use hybrid_format_impls::{Formatted, HybridFormat};
pub use hybrid_format_macros::hformat;

#[doc(hidden)]
pub mod __private {
    pub use const_format;
    pub use hybrid_format_impls::const_args;
    pub use hybrid_format_impls::push_str_unchecked;
}

#[macro_export]
#[doc(hidden)]
macro_rules! __hformat_internal {
    // a dynamic (runtime) item
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr} $(, $($remaining:tt)*)?) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let _dynamic_elem = &$dynamic_elem;
        let _dynamic_formatted = $crate::HybridFormat::format(_dynamic_elem);
        $crate::__hformat_internal!(
            [$buffer]
            []
            [
                $($capacity_expr)*
                + _temp_formatted.len()
                + $crate::Formatted::size(&_dynamic_formatted)
            ]
            [$(
                $add_to_str_statements)*
                unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
                unsafe {$crate::Formatted::append(&_dynamic_formatted, $buffer)};
            ]
            $($($remaining)*)?
        )
    }};
    // static item
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr $(, $($remaining:tt)*)?) => {{
        $crate::__hformat_internal!(
            [$buffer]
            [$($pending_static_elems)* $static_elem,]
            [$($capacity_expr)*]
            [$($add_to_str_statements)*]
            $($($remaining)*)?
        )
    }};
    // pure const
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] []) => {
        $crate::__private::const_format::concatcp!($($pending_static_elems)*)
    };
    // runtime/const hybrid
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*]) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let mut $buffer = String::with_capacity($($capacity_expr)* + _temp_formatted.len());
        {
            // shadowed just to give the statements a &mut String
            let $buffer = &mut $buffer;
            $($add_to_str_statements)*
            unsafe {$crate::__private::push_str_unchecked($buffer, _temp_formatted)};
        }
        $buffer
    }};
    // the last one, it's the one that's actually called in the code
    ($($elems: tt)*) => {
        $crate::__hformat_internal!(
            [buf]
            []
            [0usize]
            []
            $($elems)*
        )
    };
}

#[cfg(test)]
mod tests {
    use hybrid_format_macros::hformat;

    #[test]
    fn int_const_literal() {
        assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
    }
    #[test]
    fn int_const_variable() {
        const MY_INT: i32 = 42;
        assert_eq!(const { hformat!("{}", MY_INT) }, format!("{}", MY_INT));
    }
    #[test]
    fn int_dynamic() {
        let i = 2 + 2;
        assert_eq!(hformat!("{}", i), format!("{}", i));
    }
    #[test]
    fn float_const_literal() {
        assert_eq!(
            const { hformat!("Float: {}", 4.2) },
            format!("Float: {}", 4.2)
        );
    }
    #[test]
    fn float_const_variable() {
        const MY_FLOAT: f64 = 4.2;
        assert_eq!(
            const { hformat!("Float: {}", MY_FLOAT) },
            format!("Float: {}", MY_FLOAT)
        );
    }
    #[test]
    fn float_dynamic() {
        let f = 4.2 + 6.7;
        assert_eq!(hformat!("Float: {}", f), format!("Float: {}", f));
    }
    #[test]
    fn bool_const_literal() {
        assert_eq!(
            const { hformat!("Bool: {}", true) },
            format!("Bool: {}", true)
        );
    }
    #[test]
    fn bool_const_variable() {
        const MY_BOOL: bool = true;
        assert_eq!(
            const { hformat!("Bool: {}", MY_BOOL) },
            format!("Bool: {}", MY_BOOL)
        );
    }
    #[test]
    fn bool_dynamic() {
        let b = true;
        assert_eq!(hformat!("Bool: {}", b), format!("Bool: {}", b));
    }
    #[test]
    fn char_const_literal() {
        assert_eq!(
            const { hformat!("Char: {}", 'a') },
            format!("Char: {}", 'a')
        );
    }
    #[test]
    fn char_const_variable() {
        const MY_CHAR: char = 'a';
        assert_eq!(
            const { hformat!("Char: {}", MY_CHAR) },
            format!("Char: {}", MY_CHAR)
        );
    }
    #[test]
    fn char_dynamic() {
        let c = 'a';
        assert_eq!(hformat!("Char: {}", c), format!("Char: {}", c));
    }
    #[test]
    fn str_const_literal() {
        assert_eq!(
            const { hformat!("String: {}", "Hello, world!") },
            format!("String: {}", "Hello, world!")
        );
    }
    #[test]
    fn str_const_variable() {
        const MY_STR: &str = "Hello, world!";
        assert_eq!(
            const { hformat!("String: {}", MY_STR) },
            format!("String: {}", MY_STR)
        );
    }
    #[test]
    fn str_dynamic() {
        let s = "Hello, world!";
        assert_eq!(hformat!("String: {}", s), format!("String: {}", s));
    }
    #[test]
    fn hybrid() {
        const MY_INT: i32 = 42;
        const MY_BOOL: bool = true;
        const MY_STR: &str = "Hello, world!";
        let f = 4.2 + 6.7;
        let c = 'a';
        assert_eq!(
            hformat!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"),
            format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}")
        );
    }
}
