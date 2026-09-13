pub use hybrid_format_impls::{Formatted, HybridFormat};
pub use hybrid_format_macros::hformat;

#[doc(hidden)]
pub mod __private {
    pub use const_format;
    pub use hybrid_format_impls::push_str_unchecked;
}

#[macro_export]
#[doc(hidden)]
macro_rules! __hformat_internal {
    // a dynamic (runtime) item with some elements after
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr}, $($remaining:tt)*) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let _dynamic_elem = $dynamic_elem;
        let _dynamic_formatted = $crate::HybridFormat::format(&_dynamic_elem);
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
                $crate::Formatted::append(&_dynamic_formatted, $buffer);
            ]
            $($remaining)*
        )
    }};
    // a dynamic (runtime) item with no elements after (the last one), allows a trailing comma
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr} $(,)?) => {{
        let _temp_formatted: &str = $crate::__private::const_format::concatcp!($($pending_static_elems)*);
        let _dynamic_elem = $dynamic_elem;
        let _dynamic_formatted = $crate::HybridFormat::format(&_dynamic_elem);
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
                $crate::Formatted::append(&_dynamic_formatted, $buffer);
            ]
        )
    }};
    // static item with some elements after
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr, $($remaining:tt)*) => {{
        $crate::__hformat_internal!(
            [$buffer]
            [$($pending_static_elems)* $static_elem,]
            [$($capacity_expr)*]
            [$($add_to_str_statements)*]
            $($remaining)*
        )
    }};
    // static item with no elements after
    ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr $(,)?) => {{
        $crate::__hformat_internal!(
            [$buffer]
            [$($pending_static_elems)* $static_elem,]
            [$($capacity_expr)*]
            [$($add_to_str_statements)*]
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
    fn test() {
        assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
        assert_eq!(const { hformat!("{42}") }, "42");
        assert_eq!(
            const { hformat!("Hello, world!") },
            format!("Hello, world!")
        );
        assert_eq!(
            const { hformat!("Float: {}", 4.2) },
            format!("Float: {}", 4.2)
        );
        let x = 4.2e5;
        assert_eq!(hformat!("Float: {x}"), format!("Float: {x}.0"));
        const B: bool = true;
        assert_eq!(hformat!("Bool: {B}"), format!("Bool: {B}"));
    }
}
