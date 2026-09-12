use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn hformat(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::LitStr).value();

    let mut format_tokens = Vec::new();
    let mut temp_str = String::new();

    let mut input_chars = input.chars();
    while let Some(c) = input_chars.next() {
        match c {
            '{' => {
                if !temp_str.is_empty() {
                    format_tokens.push(quote! { #temp_str });
                    temp_str.clear();
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
        if !temp_str.is_empty() {
            format_tokens.push(quote! {
                #temp_str
            })
        }
    }

    // This is just to check it works
    quote::quote! {
        {
            format!("{}", #(#format_tokens)*)
        }
    }
    .into()
}
