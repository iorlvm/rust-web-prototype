mod attributes;
mod endpoint;

use proc_macro::TokenStream;
use quote::quote;

/// Handler attribute macro
///
/// 將 endpoint function 包裝成 handler，並註冊至 inventory。
///
/// ## 用法
/// ```rust
/// #[handler(
///     method = "GET",
///     route = "/test",
///     middleware(
///         AuthMiddleware::new(...),
///         LogMiddleware::new()
///     )
/// )]
/// pub async fn execute(ctx: &mut Context, req: &mut Request) -> Result<Response, KernelError> {
///     Ok(...)
/// }
/// ```
///
/// ## 注意
/// - middleware 需實作 `Middleware` trait
#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let result = endpoint::parse_endpoint(item);
    if result.is_err() {
        return result.unwrap_err().to_compile_error().into();
    }
    let (endpoint_ident, endpoint_struct) = result.unwrap();

    let result = attributes::parse_attributes(attr);
    if result.is_err() {
        return result.unwrap_err().to_compile_error().into();
    }
    let (method, route, middleware_token) = result.unwrap();
    let route_lit = syn::LitStr::new(&route, proc_macro2::Span::call_site());

    let endpoint_factory_fn_ident = syn::Ident::new(
        &format!("__endpoint_factory_{}", endpoint_ident),
        proc_macro2::Span::call_site(),
    );

    let middleware_factory_fn_ident = syn::Ident::new(
        &format!("__middleware_factory_{}", endpoint_ident),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #endpoint_struct

        fn #endpoint_factory_fn_ident() -> Box<dyn ::web_kernel::Endpoint> {
            Box::new(#endpoint_ident::default())
        }

        fn #middleware_factory_fn_ident() -> Vec<Box<dyn ::web_kernel::middleware::Middleware>> {
             #middleware_token
        }

        // ::inventory::submit! HandlerRegistration 進行自動註冊
        ::inventory::submit! {
            // 使用 attributes(method, route, middleware) 以及 endpoint 組合成 HandlerRegistration
            ::web_kernel::engine::factory::HandlerRegistration {
                method: #method,
                route: #route_lit,
                endpoint: #endpoint_factory_fn_ident,
                middleware: #middleware_factory_fn_ident
            }
        }
    };

    expanded.into()
}
