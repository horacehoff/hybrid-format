use hybrid_format_impls::*;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, LitStr, parse_macro_input};

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
                    format_tokens.push(quote! { #arg });
                    continue;
                }
                let mut expr = String::new();
                let mut expr_depth = 1;
                while let Some(c) = input_chars.next() {
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
                format_tokens.push(quote! { #expr })
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

    // This is just to check it works
    let hformat = import_hformat();
    quote::quote! {
        {
            use #hformat;
            format!("hello")
            // format!("{}", #(#format_tokens)*)
        }
    }
    .into()
}
