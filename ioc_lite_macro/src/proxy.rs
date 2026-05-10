use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, Visibility};

pub fn expand_method(item: ItemImpl) -> TokenStream {
    let self_ty = &item.self_ty;
    let struct_name = quote! { #self_ty }.to_string();
    let ident = format_ident!("{}Proxy", struct_name);

    let mut func_wrap = Vec::new();

    for impl_item in &item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };

        if !matches!(method.vis, Visibility::Public(_)) {
            continue;
        }

        if let Some(tokens) = build_proxy_method(method) {
            func_wrap.push(tokens);
        }
    }

    quote! {
        #item

        impl #ident {
            #(#func_wrap)*
        }
    }
}

fn build_proxy_method(method: &ImplItemFn) -> Option<TokenStream> {
    let sig = &method.sig;
    let ident = &sig.ident;
    let output = &sig.output;
    let inputs = &sig.inputs;
    let generics = &sig.generics;
    let is_async = sig.asyncness.is_some();

    let receiver = inputs.first()?;

    match receiver {
        FnArg::Receiver(receiver) => {
            if receiver.reference.is_none() {
                return None;
            }

            let args: Vec<_> = inputs.iter().skip(1).collect();

            let arg_names = args.iter().filter_map(|arg| {
                let FnArg::Typed(pat) = arg else {
                    return None;
                };

                Some(&pat.pat)
            });

            let invoke = if is_async {
                quote! {
                    instance.#ident(#(#arg_names),*).await
                }
            } else {
                quote! {
                    instance.#ident(#(#arg_names),*)
                }
            };

            if receiver.mutability.is_some() {
                Some(quote! {
                    pub async fn #ident #generics (
                        &self, #(#args),*
                    ) #output {
                        let instance = self.inner.get_instance().await;
                        let mut instance = instance.write().await;
                        #invoke
                    }
                })
            } else {
                Some(quote! {
                    pub async fn #ident #generics (
                        &self, #(#args),*
                    ) #output {
                        let instance = self.inner.get_instance().await;
                        let instance = instance.read().await;
                        #invoke
                    }
                })
            }
        }

        _ => None,
    }
}
