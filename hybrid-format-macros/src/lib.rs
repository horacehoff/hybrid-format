//! Use the `hybrid-format` crate.

use crate::parser::Alignment;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{BinOp, Expr, Lit, LitStr, UnOp, ext::IdentExt, parse_macro_input};

mod parser;

struct FmtArg {
    name: Option<proc_macro2::Ident>,
    value: Expr,
    is_const: bool,
    used: bool,
    runtime: bool,
}

enum Placeholder {
    Argument(usize),
    Inline(Expr),
}

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
        Expr::Paren(syn::ExprParen { expr, .. })
        | Expr::Group(syn::ExprGroup { expr, .. })
        | Expr::Cast(syn::ExprCast { expr, .. })
        | Expr::Field(syn::ExprField { base: expr, .. })
        | Expr::Repeat(syn::ExprRepeat { expr, .. }) => is_probably_const(expr),
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
        Expr::Index(e) => is_probably_const(&e.expr) && is_probably_const(&e.index),
        Expr::If(e) => {
            is_probably_const(&e.cond)
                && matches!(e.then_branch.stmts.as_slice(), [syn::Stmt::Expr(last, None)] if is_probably_const(last))
                && e.else_branch.as_ref().is_some_and(|(_, expr)| is_probably_const(expr))
        }
        Expr::Block(syn::ExprBlock { block, .. }) | Expr::Unsafe(syn::ExprUnsafe { block, .. }) => {
            matches!(block.stmts.as_slice(), [syn::Stmt::Expr(last, None)] if is_probably_const(last))
        }
        Expr::Array(syn::ExprArray { elems, .. }) | Expr::Tuple(syn::ExprTuple { elems, .. }) => {
            elems.iter().all(is_probably_const)
        }
        Expr::MethodCall(e) => {
            is_probably_const(&e.receiver)
                && e.args.iter().all(is_probably_const)
                && matches!(
                    e.method.to_string().as_str(),
                    // const methods from i32 (includes nightly ones just in case)
                    "count_ones"
                        | "count_zeros"
                        | "leading_zeros"
                        | "trailing_zeros"
                        | "leading_ones"
                        | "trailing_ones"
                        | "isolate_highest_one"
                        | "isolate_lowest_one"
                        | "highest_one"
                        | "lowest_one"
                        | "cast_unsigned"
                        | "saturating_cast_unsigned"
                        | "checked_cast_unsigned"
                        | "strict_cast_unsigned"
                        | "rotate_left"
                        | "rotate_right"
                        | "swap_bytes"
                        | "reverse_bits"
                        | "to_be"
                        | "to_le"
                        | "checked_add"
                        | "strict_add"
                        | "unchecked_add"
                        | "checked_add_unsigned"
                        | "strict_add_unsigned"
                        | "checked_sub"
                        | "strict_sub"
                        | "unchecked_sub"
                        | "checked_sub_unsigned"
                        | "strict_sub_unsigned"
                        | "checked_mul"
                        | "strict_mul"
                        | "unchecked_mul"
                        | "checked_div"
                        | "strict_div"
                        | "checked_div_euclid"
                        | "strict_div_euclid"
                        | "checked_div_exact"
                        | "div_exact"
                        | "unchecked_div_exact"
                        | "checked_rem"
                        | "strict_rem"
                        | "checked_rem_euclid"
                        | "strict_rem_euclid"
                        | "checked_neg"
                        | "unchecked_neg"
                        | "strict_neg"
                        | "checked_shl"
                        | "strict_shl"
                        | "unchecked_shl"
                        | "unbounded_shl"
                        | "shl_exact"
                        | "unchecked_shl_exact"
                        | "checked_shr"
                        | "strict_shr"
                        | "unchecked_shr"
                        | "unbounded_shr"
                        | "shr_exact"
                        | "unchecked_shr_exact"
                        | "checked_abs"
                        | "strict_abs"
                        | "checked_pow"
                        | "strict_pow"
                        | "checked_isqrt"
                        | "saturating_add"
                        | "saturating_add_unsigned"
                        | "saturating_sub"
                        | "saturating_sub_unsigned"
                        | "saturating_neg"
                        | "saturating_abs"
                        | "saturating_mul"
                        | "saturating_div"
                        | "saturating_pow"
                        | "wrapping_add"
                        | "wrapping_add_unsigned"
                        | "wrapping_sub"
                        | "wrapping_sub_unsigned"
                        | "wrapping_mul"
                        | "wrapping_div"
                        | "wrapping_div_euclid"
                        | "wrapping_rem"
                        | "wrapping_rem_euclid"
                        | "wrapping_neg"
                        | "wrapping_shl"
                        | "wrapping_shr"
                        | "wrapping_abs"
                        | "unsigned_abs"
                        | "wrapping_pow"
                        | "overflowing_add"
                        | "overflowing_add_unsigned"
                        | "overflowing_sub"
                        | "overflowing_sub_unsigned"
                        | "overflowing_mul"
                        | "widening_mul"
                        | "overflowing_div"
                        | "overflowing_div_euclid"
                        | "overflowing_rem"
                        | "overflowing_rem_euclid"
                        | "overflowing_neg"
                        | "overflowing_shl"
                        | "overflowing_shr"
                        | "overflowing_abs"
                        | "overflowing_pow"
                        | "pow"
                        | "isqrt"
                        | "div_floor"
                        | "div_ceil"
                        | "next_multiple_of"
                        | "checked_next_multiple_of"
                        | "ilog"
                        | "ilog2"
                        | "ilog10"
                        | "checked_ilog"
                        | "checked_ilog2"
                        | "checked_ilog10"
                        | "abs"
                        | "abs_diff"
                        | "signum"
                        | "is_positive"
                        | "is_negative"
                        | "to_be_bytes"
                        | "to_le_bytes"
                        | "to_ne_bytes"
                        | "truncate"
                        | "saturating_truncate"
                        | "checked_truncate"
                        | "widen"
                        | "saturating_cast"
                        | "wrapping_cast"
                        | "checked_cast"
                        | "strict_cast"
                        | "unchecked_cast"
                        | "midpoint"

                        // u32
                        | "bit_width"
                        | "cast_signed"
                        | "saturating_cast_signed"
                        | "checked_cast_signed"
                        | "strict_cast_signed"
                        | "funnel_shl"
                        | "funnel_shr"
                        | "unchecked_funnel_shl"
                        | "unchecked_funnel_shr"
                        | "carryless_mul"
                        | "extract_bits"
                        | "deposit_bits"
                        | "checked_add_signed"
                        | "strict_add_signed"
                        | "checked_sub_signed"
                        | "strict_sub_signed"
                        | "checked_signed_diff"
                        | "unchecked_disjoint_bitor"
                        | "saturating_add_signed"
                        | "saturating_sub_signed"
                        | "wrapping_add_signed"
                        | "wrapping_sub_signed"
                        | "overflowing_add_signed"
                        | "overflowing_sub_signed"
                        | "is_multiple_of"
                        | "is_power_of_two"
                        | "next_power_of_two"
                        | "checked_next_power_of_two"
                        | "wrapping_next_power_of_two"
                        | "widening_carryless_mul"
                        | "carrying_carryless_mul"

                        // other stuff
                        | "is_some"
                        | "is_none"
                )
        }
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
            BinOp::Or(_)
            | BinOp::And(_)
            | BinOp::Eq(_)
            | BinOp::Ne(_)
            | BinOp::Lt(_)
            | BinOp::Le(_)
            | BinOp::Gt(_)
            | BinOp::Ge(_) => None,
            BinOp::Shl(_) | BinOp::Shr(_) => get_literal_type(&binary_op.left),
            _ => get_literal_type(&binary_op.left).and_then(|_| get_literal_type(&binary_op.right)),
        },
        _ => None,
    }
}

fn format_expr(
    hformat: &proc_macro2::TokenStream,
    expr: &Expr,
    (width, fill, alignment): (usize, char, Option<Alignment>),
    is_const: bool,
) -> proc_macro2::TokenStream {
    let alignment = alignment.map(|alignment| match alignment {
        Alignment::Left => quote!(#hformat::Alignment::Left),
        Alignment::Center => quote!(#hformat::Alignment::Center),
        Alignment::Right => quote!(#hformat::Alignment::Right),
    });
    if is_const {
        let expr = get_literal_type(expr)
            .map_or_else(|| quote!(#expr), |literal_type| quote!((#expr) as #literal_type));
        let value = quote!(const {#hformat::__private::HybridFormatConstArg(#expr).format_const()}.as_const_arg());
        if width == 0 {
            return value;
        }
        let alignment = alignment
            .unwrap_or_else(|| quote!(#hformat::__private::default_alignment(&(const{#expr}))));
        let fill_chars = fill.to_string().repeat(width);
        let const_padding = quote!(#hformat::__private::const_format_padding(#fill_chars, #width, #hformat::__private::const_format::concatcp!((#value)), #alignment));
        quote!(const {#const_padding.0}, #value, const {#const_padding.1})
    } else if width == 0 {
        quote! ({#expr})
    } else {
        let alignment = alignment.map_or_else(
            || quote!(::core::option::Option::None),
            |alignment| quote!(::core::option::Option::Some(#alignment)),
        );
        quote!({#hformat::__private::Padded { value: &(#expr), width: #width, fill: #fill, alignment: #alignment }})
    }
}

impl syn::parse::Parse for HFormatInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let buf_tgt = if input.peek(syn::token::Paren) { Some(input.parse()?) } else { None };
        let format_str = input.parse()?;

        let args = if input.parse::<Option<syn::Token![,]>>()?.is_some() {
            syn::punctuated::Punctuated::parse_terminated(input)?
        } else {
            syn::punctuated::Punctuated::new()
        };

        Ok(Self { buf_tgt, format_str, args })
    }
}

#[cold]
fn compile_error(span: Span, message: String) -> TokenStream {
    syn::Error::new(span, message).into_compile_error().into()
}

#[inline(always)]
fn runtime_binding(idx: usize) -> proc_macro2::Ident {
    quote::format_ident!("__hybrid_format_arg{}", idx, span = Span::mixed_site())
}

#[doc(hidden)]
#[proc_macro]
#[inline]
pub fn hformat(input: TokenStream) -> TokenStream {
    let HFormatInput { buf_tgt, format_str, args } = parse_macro_input!(input as HFormatInput);
    let hformat = quote!(__hitchhikers_guide_to_hybrid_format);
    let span = format_str.span();

    let format_parts = match parser::parse_format_string(&format_str.value()) {
        Ok(parts) => parts,
        Err(msg) => return compile_error(span, msg),
    };

    let mut named_arguments = false;
    let mut arguments = Vec::with_capacity(args.len());
    for arg in args {
        let (name, value) = match arg {
            Expr::Assign(assign) => {
                if let Expr::Path(path) = assign.left.as_ref()
                    && let Some(ident) = path.path.get_ident()
                {
                    (Some(ident.unraw()), *assign.right)
                } else {
                    (None, Expr::Assign(assign))
                }
            }
            val => (None, val),
        };
        if name.is_none() && named_arguments {
            return compile_error(
                span,
                "positional arguments cannot follow named arguments".into(),
            );
        }
        named_arguments |= name.is_some();
        arguments.push(FmtArg {
            name,
            is_const: is_probably_const(&value),
            value,
            used: false,
            runtime: false,
        });
    }

    let mut next_positional = 0;
    let mut format_tokens = Vec::new();
    for part in format_parts {
        match part {
            parser::FormatPart::Text(text) => format_tokens.push(quote!(#text)),
            parser::FormatPart::Placeholder { arg, spec, force_const } => {
                let Some(padding) = spec.get_padding() else {
                    return compile_error(span, "this format spec isn't supported yet".into());
                };
                let placeholder_tgt = match arg {
                    parser::Argument::NextPositional => {
                        let curr_idx = next_positional;
                        next_positional += 1;
                        Placeholder::Argument(curr_idx)
                    }
                    parser::Argument::Index(idx) => Placeholder::Argument(idx),
                    parser::Argument::Expression(e) => {
                        if let Some(idx) = arguments
                            .iter()
                            .position(|arg| arg.name.as_ref().is_some_and(|name| *name == e))
                        {
                            Placeholder::Argument(idx)
                        } else {
                            match LitStr::new(&e, span).parse::<Expr>() {
                                Ok(expr) => Placeholder::Inline(expr),
                                Err(err) => {
                                    return compile_error(
                                        span,
                                        format!("invalid expression `{e}`: {err}"),
                                    );
                                }
                            }
                        }
                    }
                };
                format_tokens.push(match placeholder_tgt {
                    Placeholder::Argument(idx) => {
                        let Some(arg) = arguments.get_mut(idx) else {
                            return compile_error(span, format!("unknown argument {idx}"));
                        };
                        arg.used = true;
                        if force_const || arg.is_const {
                            format_expr(&hformat, &arg.value, padding, true)
                        } else {
                            arg.runtime = true;
                            let arg_binding = runtime_binding(idx);
                            format_expr(&hformat, &syn::parse_quote!(#arg_binding), padding, false)
                        }
                    }
                    Placeholder::Inline(expr) => format_expr(
                        &hformat,
                        &expr,
                        padding,
                        force_const || is_probably_const(&expr),
                    ),
                });
            }
        }
    }
    if !arguments.iter().all(|arg| arg.used) {
        return compile_error(span, "argument never used".into());
    }

    let (runtime_bindings, values): (Vec<_>, Vec<_>) = arguments
        .iter()
        .enumerate()
        .filter(|(_, arg)| arg.runtime)
        .map(|(idx, arg)| (runtime_binding(idx), &arg.value))
        .unzip();

    quote::quote! {
        match (#(&(#values),)*) {
            (#(#runtime_bindings,)*) => #hformat::__hformat_internal!(#buf_tgt #(#format_tokens),*)
        }
    }
    .into()
}
