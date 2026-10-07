//! Use the `hybrid-format` crate.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{BinOp, Expr, Ident, Lit, LitStr, UnOp, parse_macro_input};

mod parser;

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
        Expr::Lit(_) | Expr::Const(_) => true, // obviously
        Expr::Path(path) => path.path.segments.last().map_or(
            false,
            |name| {
                name.ident
                    .to_string()
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            }, // SCREAMING_SNAKE_CASE (Rust convention for consts)
        ),
        Expr::Macro(m) => {
            let macro_path = &m.mac.path.segments;
            (macro_path.len() == 1
                || (macro_path.len() == 2
                    && (macro_path[0].ident == "std" || macro_path[0].ident == "core")))
                && macro_path.last().is_some_and(|macro_name| {
                    matches!(
                        macro_name.ident.to_string().as_str(),
                        "env"
                            | "cfg"
                            | "concat"
                            | "stringify"
                            | "line"
                            | "column"
                            | "file"
                            | "module_path"
                            | "include_str"
                    )
                })
        }
        Expr::Paren(e) => is_probably_const(&e.expr),
        Expr::Group(e) => is_probably_const(&e.expr),
        Expr::Unary(unary_op) => {
            matches!(unary_op.op, UnOp::Neg(_) | UnOp::Not(_)) && is_probably_const(&unary_op.expr)
        }
        Expr::Binary(binary_op) => {
            matches!(
                binary_op.op,
                BinOp::Add(_)
                    | BinOp::Sub(_)
                    | BinOp::Mul(_)
                    | BinOp::Div(_)
                    | BinOp::Rem(_)
                    | BinOp::And(_)
                    | BinOp::Or(_)
                    | BinOp::BitXor(_)
                    | BinOp::BitAnd(_)
                    | BinOp::BitOr(_)
                    | BinOp::Shl(_)
                    | BinOp::Shr(_)
            ) && is_probably_const(&binary_op.left)
                && is_probably_const(&binary_op.right)
        }
        Expr::Cast(e) => is_probably_const(&e.expr),
        Expr::Field(f) => is_probably_const(&f.base),
        Expr::Index(e) => is_probably_const(&e.expr) && is_probably_const(&e.index),
        _ => false,
    }
}

fn const_arg(
    hformat: &proc_macro2::TokenStream,
    expr: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote!(const {#hformat::__private::const_args::HybridFormatConstArg(#expr).format_const()}.as_const_arg())
}

fn get_literal_type(expr: &Expr) -> Option<proc_macro2::TokenStream> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(int) if int.suffix().is_empty() => Some(quote!(i32)),
            Lit::Float(float) if float.suffix().is_empty() => Some(quote!(f64)),
            _ => None,
        },
        Expr::Paren(e) => get_literal_type(&e.expr),
        Expr::Group(e) => get_literal_type(&e.expr),
        Expr::Unary(unary_op) => get_literal_type(&unary_op.expr),
        Expr::Binary(binary_op) => match binary_op.op {
            BinOp::Or(_) | BinOp::And(_) => None,
            BinOp::Shl(_) | BinOp::Shr(_) => get_literal_type(&binary_op.left),
            _ => get_literal_type(&binary_op.left).and_then(|_| get_literal_type(&binary_op.right)),
        },
        _ => None,
    }
}

fn format_expr(hformat: &proc_macro2::TokenStream, expr: &Expr) -> proc_macro2::TokenStream {
    if is_probably_const(expr) {
        const_arg(
            hformat,
            &get_literal_type(expr).map_or_else(
                || quote!(#expr),
                |literal_type| quote!((#expr) as #literal_type),
            ),
        )
    } else {
        quote! ({#expr})
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

#[cold]
fn compile_error(span: Span, message: String) -> TokenStream {
    syn::Error::new(span, message).into_compile_error().into()
}

#[proc_macro]
#[inline]
pub fn hformat(input: TokenStream) -> TokenStream {
    let HFormatInput { format_str, args } = parse_macro_input!(input as HFormatInput);
    let hformat = import_hformat();
    let span = format_str.span();

    let format_parts = match parser::parse_format_string(&format_str.value()) {
        Ok(parts) => parts,
        Err(msg) => return compile_error(span, msg),
    };

    let mut format_tokens = Vec::new();
    let mut positional_parameters = args.iter();
    for part in format_parts {
        match part {
            parser::FormatPart::Text(text) => format_tokens.push(quote!(#text)),
            parser::FormatPart::Placeholder { arg, spec } => {
                if spec != parser::FormatSpec::default() {
                    return compile_error(span, "format specs are not supported yet".into());
                }
                let expr = match arg {
                    parser::Argument::NextPositional => match positional_parameters.next() {
                        Some(expr) => expr.clone(),
                        None => {
                            return compile_error(
                                span,
                                "there are more positional parameters than arguments".into(),
                            );
                        }
                    },
                    parser::Argument::Expression(e) => match syn::parse_str::<Expr>(&e) {
                        Ok(expr) => expr,
                        Err(err) => {
                            return compile_error(span, format!("invalid expression `{e}`: {err}"));
                        }
                    },
                    parser::Argument::Index(_) => unreachable!("Please report this bug!"),
                };
                format_tokens.push(format_expr(&hformat, &expr));
            }
        }
    }
    if positional_parameters.next().is_some() {
        return compile_error(
            span,
            "there are more arguments than positional parameters".into(),
        );
    }

    quote::quote! {
        {
            #hformat::__hformat_internal!(#(#format_tokens),*)
        }
    }
    .into()
}
