//! Use the `hybrid-format` crate.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{BinOp, Expr, Lit, LitStr, UnOp, ext::IdentExt, parse_macro_input};

mod parser;

struct HFormatInput {
    buf_tgt: Option<proc_macro2::TokenTree>,
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
        let expr = get_literal_type(expr).map_or_else(
            || quote!(#expr),
            |literal_type| quote!((#expr) as #literal_type),
        );
        quote!(const {#hformat::__private::HybridFormatConstArg(#expr).format_const()}.as_const_arg())
    } else {
        quote! ({#expr})
    }
}

impl syn::parse::Parse for HFormatInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let buf_tgt = if input.peek(syn::token::Paren) {
            Some(input.parse()?)
        } else {
            None
        };
        let format_str = input.parse()?;

        let args = if input.parse::<Option<syn::Token![,]>>()?.is_some() {
            syn::punctuated::Punctuated::parse_terminated(input)?
        } else {
            syn::punctuated::Punctuated::new()
        };

        Ok(Self {
            buf_tgt,
            format_str,
            args,
        })
    }
}

#[cold]
fn compile_error(span: Span, message: String) -> TokenStream {
    syn::Error::new(span, message).into_compile_error().into()
}

#[doc(hidden)]
#[proc_macro]
#[inline]
pub fn hformat(input: TokenStream) -> TokenStream {
    let HFormatInput {
        buf_tgt,
        format_str,
        args,
    } = parse_macro_input!(input as HFormatInput);
    let hformat = quote!(__hitchhikers_guide_to_hybrid_format);
    let span = format_str.span();

    let format_parts = match parser::parse_format_string(&format_str.value()) {
        Ok(parts) => parts,
        Err(msg) => return compile_error(span, msg),
    };

    let mut named_arguments = Vec::new();
    let mut arg_values = Vec::new();
    let mut runtime_bindings = Vec::new();
    for (i, arg) in args.into_iter().enumerate() {
        let name = if let Expr::Assign(assign) = &arg
            && let Expr::Path(path) = assign.left.as_ref()
            && let Some(ident) = path.path.get_ident()
        {
            Some(ident.unraw())
        } else {
            None
        };
        let value = match arg {
            Expr::Assign(assign) if name.is_some() => *assign.right,
            _ if named_arguments.last().is_some_and(Option::is_some) => {
                return compile_error(
                    span,
                    "positional arguments cannot follow named arguments".into(),
                );
            }
            val => val,
        };
        arg_values.push(if is_probably_const(&value) {
            format_expr(&hformat, &value)
        } else {
            let arg_binding =
                quote::format_ident!("__hybrid_format_arg{}", i, span = Span::mixed_site());
            runtime_bindings.push(quote!(let #arg_binding = &(#value);));
            quote!({#arg_binding})
        });
        named_arguments.push(name);
    }

    let mut used = vec![false; arg_values.len()];
    let mut next_positional = 0;
    let mut format_tokens = Vec::new();
    for part in format_parts {
        match part {
            parser::FormatPart::Text(text) => format_tokens.push(quote!(#text)),
            parser::FormatPart::Placeholder { arg, spec } => {
                if spec != parser::FormatSpec::default() {
                    return compile_error(span, "format specs are not supported yet".into());
                }
                let index = match arg {
                    parser::Argument::NextPositional => {
                        let curr_idx = next_positional;
                        next_positional += 1;
                        curr_idx
                    }
                    parser::Argument::Index(idx) => idx,
                    parser::Argument::Expression(e) => {
                        if let Some(idx) = named_arguments
                            .iter()
                            .position(|name| name.as_ref().is_some_and(|name| *name == e))
                        {
                            idx
                        } else {
                            match LitStr::new(&e, span).parse::<Expr>() {
                                Ok(expr) => format_tokens.push(format_expr(&hformat, &expr)),
                                Err(err) => {
                                    return compile_error(
                                        span,
                                        format!("invalid expression `{e}`: {err}"),
                                    );
                                }
                            }
                            continue;
                        }
                    }
                };
                let Some(tokens) = arg_values.get(index) else {
                    return compile_error(span, format!("unknown argument {index}"));
                };
                used[index] = true;
                format_tokens.push(tokens.clone());
            }
        }
    }
    if used.iter().any(|used| !used) {
        return compile_error(span, "argument never used".into());
    }

    quote::quote! {{
        #(#runtime_bindings)*
        #hformat::__hformat_internal!(#buf_tgt #(#format_tokens),*)
    }}
    .into()
}
