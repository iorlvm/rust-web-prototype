use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream}, Expr, Ident, LitStr, Result,
    Token,
};

struct HandlerArgs {
    method: Option<LitStr>,
    route: Option<LitStr>,
    middleware: Option<proc_macro2::TokenStream>,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut method = None;
        let mut route = None;
        let mut middleware = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "method" => {
                    input.parse::<Token![=]>()?;
                    method = Some(input.parse()?);
                }
                "route" => {
                    input.parse::<Token![=]>()?;
                    route = Some(input.parse()?);
                }
                "middleware" => {
                    let content;
                    syn::parenthesized!(content in input);

                    let middleware_expressions: Vec<Expr> = content
                        .parse_terminated(Expr::parse, Token![,])?
                        .into_iter()
                        .collect();

                    middleware = Some(quote! { vec![#(Box::new(#middleware_expressions)),*] });
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown attribute key"));
                }
            }

            // 處理逗號
            let _ = input.parse::<Token![,]>();
        }

        Ok(HandlerArgs {
            method,
            route,
            middleware,
        })
    }
}

pub fn parse_attributes(
    attr: TokenStream,
) -> Result<(proc_macro2::TokenStream, String, proc_macro2::TokenStream)> {
    let args = syn::parse::<HandlerArgs>(attr)?;

    let method_str = args
        .method
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing method"))?
        .value();

    let route = args
        .route
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing route"))?
        .value();

    let method = match method_str.as_str() {
        "GET" => quote! { http::Method::GET },
        "POST" => quote! { http::Method::POST },
        "PUT" => quote! { http::Method::PUT },
        "DELETE" => quote! { http::Method::DELETE },
        "PATCH" => quote! { http::Method::PATCH },
        "HEAD" => quote! { http::Method::HEAD },
        "OPTIONS" => quote! { http::Method::OPTIONS },
        "TRACE" => quote! { http::Method::TRACE },
        "CONNECT" => quote! { http::Method::CONNECT },
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "invalid HTTP method",
            ));
        }
    };

    let middleware = args.middleware.unwrap_or_else(|| {
        quote! { Vec::<Box<dyn ::web_kernel::middleware::Middleware>>::new() }
    });

    Ok((method, route, middleware))
}
