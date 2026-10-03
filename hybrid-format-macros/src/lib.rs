//! Use the `hybrid-format` crate instead.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Expr, Ident, Lit, LitStr, parse_macro_input};

fn import_hformat() -> proc_macro2::TokenStream {
    let found_crate =
        crate_name("hybrid-format").expect("hybrid-format is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(::hybrid_format),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    }
}

struct HFormatInput {
    format_str: LitStr,
    args: syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
}

fn is_probably_const(expr: &Expr) -> bool {
    match expr {
        Expr::Const(_) => true, // obviously
        Expr::Path(path) => path.path.segments.last().map_or(
            false,
            |name| {
                name.ident
                    .to_string()
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            }, // SCREAMING_SNAKE_CASE (Rust convention for consts)
        ),
        _ => false,
    }
}

fn const_arg(
    hformat: &proc_macro2::TokenStream,
    expr: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote!(const {#hformat::__private::const_args::HybridFormatConstArg(#expr).format_const()}.as_const_arg())
}

fn parse_literal(expr: &Expr) -> Option<&Lit> {
    match expr {
        Expr::Lit(lit) => Some(&lit.lit),
        Expr::Unary(unary_op) if matches!(unary_op.op, syn::UnOp::Neg(_) | syn::UnOp::Not(_)) => {
            parse_literal(&unary_op.expr)
        }
        _ => None,
    }
}

fn format_expr(hformat: &proc_macro2::TokenStream, expr: &Expr) -> proc_macro2::TokenStream {
    match parse_literal(expr) {
        Some(Lit::Int(int)) if int.suffix().is_empty() => {
            const_arg(hformat, &quote!((#expr) as i32))
        }
        Some(Lit::Float(float)) if float.suffix().is_empty() => {
            const_arg(hformat, &quote!((#expr) as f64))
        }
        Some(_) => const_arg(hformat, &quote!(#expr)),
        None if is_probably_const(expr) => const_arg(hformat, &quote!(#expr)),
        None => quote! { { #expr } },
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

        Ok(Self { format_str, args })
    }
}

#[proc_macro]
#[inline]
/// # Panics
/// It will panic if the number of arguments given doesn't match the number of positional parameters
pub fn hformat(input: TokenStream) -> TokenStream {
    let HFormatInput { format_str, args } = parse_macro_input!(input as HFormatInput);
    let input_format_str = format_str.value();
    let hformat = import_hformat();

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
                if input_chars.peek() == Some(&'{') {
                    input_chars.next();
                    temp_str.push(c);
                    continue;
                }
                if input_chars.peek() == Some(&'}') {
                    input_chars.next();
                    let arg = &args[arg_idx];
                    arg_idx += 1;
                    format_tokens.push(format_expr(&hformat, arg));
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
                format_tokens.push(format_expr(&hformat, &expr));
            }
            _ => temp_str.push(c),
        }
    }
    if !temp_str.is_empty() {
        format_tokens.push(quote! {
            #temp_str
        });
    }
    assert!(arg_idx == args.len(), "Invalid number of arguments");

    quote::quote! {
        {
            #hformat::__hformat_internal!(#(#format_tokens),*)
        }
    }
    .into()
}
