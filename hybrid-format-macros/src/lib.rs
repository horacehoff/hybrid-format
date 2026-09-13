use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Expr, Ident, LitStr, parse_macro_input};

fn import_hformat() -> proc_macro2::TokenStream {
    let found_crate =
        crate_name("hybrid-format").expect("hybrid-format is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(crate::__private::*),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( ::#ident::__private::* )
        }
    }
}

struct HFormatInput {
    format_str: LitStr,
    args: syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
}

fn format_expr(expr: &Expr) -> proc_macro2::TokenStream {
    match expr {
        Expr::Lit(_) | Expr::Const(_) => quote! { #expr },
        _ => quote! { { #expr } },
    }
}

impl syn::parse::Parse for HFormatInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let format_str = input.parse()?;

        let args = if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            syn::punctuated::Punctuated::parse_terminated(input)?
        } else {
            syn::punctuated::Punctuated::new()
        };

        Ok(HFormatInput { format_str, args })
    }
}

#[proc_macro]
pub fn hformat(input: TokenStream) -> TokenStream {
    let HFormatInput { format_str, args } = parse_macro_input!(input as HFormatInput);
    let input_format_str = format_str.value();

    let mut format_tokens = Vec::new();
    let mut temp_str = String::new();

    let mut input_chars = input_format_str.chars().peekable();
    let mut arg_idx = 0;
    while let Some(c) = input_chars.next() {
        match c {
            '{' => {
                if !temp_str.is_empty() {
                    format_tokens.push(quote! { #temp_str });
                    temp_str.clear();
                }
                if input_chars.peek() == Some(&'}') {
                    input_chars.next();
                    let arg = &args[arg_idx];
                    arg_idx += 1;
                    format_tokens.push(format_expr(arg));
                    continue;
                }
                let mut expr = String::new();
                let mut expr_depth = 1;
                for c in input_chars.by_ref() {
                    match c {
                        '{' => {
                            expr_depth += 1;
                            expr.push(c);
                        }
                        '}' => {
                            expr_depth -= 1;
                            if expr_depth == 0 {
                                break;
                            }
                            expr.push(c);
                        }
                        _ => expr.push(c),
                    }
                }
                let expr: syn::Expr = match syn::parse_str(&expr) {
                    Ok(expr) => expr,
                    Err(e) => return e.into_compile_error().into(),
                };
                format_tokens.push(format_expr(&expr));
            }
            _ => temp_str.push(c),
        }
    }
    if !temp_str.is_empty() {
        format_tokens.push(quote! {
            #temp_str
        })
    }
    if arg_idx != args.len() {
        panic!("Invalid number of arguments")
    }

    let hformat = import_hformat();
    quote::quote! {
        {
            use #hformat;
            macro_rules! __hformat_internal {
                // a dynamic (runtime) item with some elements after
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr}, $($remaining:tt)*) => {{
                    let _temp_formatted: &str = const_format::concatcp!($($pending_static_elems)*);
                    __hformat_internal!(
                        [$buffer]
                        []
                        [$($capacity_expr)* + _temp_formatted.len() + $dynamic_elem.size()]
                        [$(
                            $add_to_str_statements)*
                            $buffer.push_str_unchecked(_temp_formatted);
                            ($dynamic_elem).append($buffer);
                        ]
                        $($remaining)*
                    )
                }};
                // a dynamic (runtime) item with no elements after (the last one), allows a trailing comma
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] {$dynamic_elem: expr} $(,)?) => {{
                    let _temp_formatted: &str = const_format::concatcp!($($pending_static_elems)*);
                    __hformat_internal!(
                        [$buffer]
                        []
                        [$($capacity_expr)* + _temp_formatted.len() + $dynamic_elem.size()]
                        [$(
                            $add_to_str_statements)*
                            $buffer.push_str(_temp_formatted);
                            ($dynamic_elem).append($buffer);
                        ]
                    )
                }};
                // static item with some elements after
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr, $($remaining:tt)*) => {{
                    __hformat_internal!(
                        [$buffer]
                        [$($pending_static_elems)* $static_elem,]
                        [$($capacity_expr)*]
                        [$($add_to_str_statements)*]
                        $($remaining)*
                    )
                }};
                // static item with no elements after
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*] $static_elem: expr $(,)?) => {{
                    __hformat_internal!(
                        [$buffer]
                        [$($pending_static_elems)* $static_elem,]
                        [$($capacity_expr)*]
                        [$($add_to_str_statements)*]
                    )
                }};
                // pure const
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] []) => {
                    const_format::concatcp!($($pending_static_elems)*)
                };
                // runtime/const hybrid
                ([$buffer:ident] [$($pending_static_elems: tt)*] [$($capacity_expr: tt)*] [$($add_to_str_statements: tt)*]) => {{
                    let _temp_formatted: &str = const_format::concatcp!($($pending_static_elems)*);
                    let mut $buffer = String::with_capacity($($capacity_expr)* + _temp_formatted.len());
                    {
                        // shadowed just to give the statements a &mut String
                        let $buffer = &mut $buffer;
                        $($add_to_str_statements)*
                    }
                    $buffer.push_str(_temp_formatted);
                    $buffer
                }};
                // the last one, it's the one that's actually called in the code
                ($($elems: tt)*) => {
                    __hformat_internal!(
                        [buf]
                        []
                        [0usize]
                        []
                        $($elems)*
                    )
                };
            }
            __hformat_internal!(#(#format_tokens),*)
        }
    }
    .into()
}
