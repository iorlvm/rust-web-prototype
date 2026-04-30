use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syn::ItemFn;

pub fn parse_endpoint(item: TokenStream) -> Result<(Ident, proc_macro2::TokenStream), syn::Error> {
    let func = syn::parse::<ItemFn>(item)?;

    // 1. async
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(&func.sig, "必須是 async fn"));
    }

    let inputs = &func.sig.inputs;

    // 2. arity
    if inputs.len() != 2 {
        return Err(syn::Error::new_spanned(
            &func.sig,
            "參數必須為 (&mut Context, &mut Request)",
        ));
    }

    // 3. ctx / req
    for (i, expected) in ["Context", "Request"].iter().enumerate() {
        match &inputs[i] {
            syn::FnArg::Typed(p) => {
                let ty = p.ty.to_token_stream().to_string();
                if !ty.contains(expected) {
                    return Err(syn::Error::new_spanned(
                        &p.ty,
                        format!("必須是 &mut {}", expected),
                    ));
                }
            }
            _ => {}
        }
    }

    // 4. return type
    let out = func.sig.output.to_token_stream().to_string();
    if !out.contains("Result") || !out.contains("Response") || !out.contains("KernelError") {
        return Err(syn::Error::new_spanned(
            &func.sig.output,
            "回傳必須是 Result<Response, KernelError>",
        ));
    }

    // 5. endpoint name generation
    let fn_ident = func.sig.ident.to_string();

    let module_path = module_path!().replace("::", "_");

    let mut hasher = DefaultHasher::new();
    module_path.hash(&mut hasher);
    fn_ident.hash(&mut hasher);

    let hash = format!("{:x}", hasher.finish());

    let endpoint_name = format!("Endpoint_{}_{}_{}", module_path, fn_ident, hash);

    let endpoint_ident = syn::Ident::new(&endpoint_name, proc_macro2::Span::call_site());

    // 7. endpoint impl
    let fn_ident = &func.sig.ident;

    let expanded = quote! {
        #func

        #[derive(Default)]
        pub struct #endpoint_ident;

        #[::web_kernel::async_trait]
        impl ::web_kernel::Endpoint for #endpoint_ident {
            async fn execute(
                &self,
                ctx: &mut ::web_kernel::engine::Context,
                req: &mut ::web_kernel::http::Request
            ) -> Result<::web_kernel::http::Response, ::web_kernel::error::KernelError> {
                #fn_ident(ctx, req).await
            }
        }
    };

    Ok((endpoint_ident, expanded))
}
