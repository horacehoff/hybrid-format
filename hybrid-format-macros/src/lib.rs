use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Expr, Ident, Lit, LitStr, parse_macro_input};

fn import_hformat() -> proc_macro2::TokenStream {
    let found_crate =
        crate_name("hybrid-format").expect("hybrid-format is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!(crate),
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

fn format_literal(expr: &Expr) -> Option<LitStr> {
    if let Expr::Lit(expr_literal) = expr {
        match &expr_literal.lit {
            Lit::Str(string) => Some(LitStr::new(&string.value(), Span::call_site())),
            Lit::Char(c) => {
                let mut buf = [0; 4];
                Some(LitStr::new(
                    c.value().encode_utf8(&mut buf),
                    Span::call_site(),
                ))
            }
            Lit::Bool(b) => Some(LitStr::new(
                if b.value() { "true" } else { "false" },
                Span::call_site(),
            )),
            Lit::Float(float) => match float.suffix() {
                "f64" | "" => Some(LitStr::new(
                    zmij::Buffer::new().format(float.base10_parse::<f64>().ok()?),
                    Span::call_site(),
                )),
                "f32" => Some(LitStr::new(
                    zmij::Buffer::new().format(float.base10_parse::<f32>().ok()?),
                    Span::call_site(),
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn is_probably_const(expr: &Expr) -> bool {
    if let Expr::Path(path) = expr
        && let Some(name) = path.path.segments.last()
    {
        name.ident
            .to_string()
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    } else {
        false
    }
}

fn format_expr(expr: &Expr) -> proc_macro2::TokenStream {
    if let Some(literal) = format_literal(expr) {
        quote!(#literal)
    } else {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Int(int) if int.suffix().is_empty() => quote! ((#expr as i32)),
                _ => quote! { #expr },
            },
            Expr::Const(_) => quote! { #expr },
            Expr::Path(_) if is_probably_const(expr) => quote!(#expr),
            _ => quote! { { #expr } },
        }
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
            #hformat::__hformat_internal!(#(#format_tokens),*)
        }
    }
    .into()
}
