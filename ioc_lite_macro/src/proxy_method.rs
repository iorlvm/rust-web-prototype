use crate::utils::{print_debug_info, proxy_struct_ident, ExtraInfo};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, Type, Visibility};

pub fn expand_method(item: ItemImpl) -> TokenStream {
    let self_ty = &item.self_ty;
    let ident = proxy_struct_ident(&quote! { #self_ty }.to_string());

    let mut func_wrap = Vec::new();

    for impl_item in &item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };

        if !matches!(method.vis, Visibility::Public(_)) {
            continue;
        }

        if let Some(tokens) = build_proxy_method(self_ty, method) {
            func_wrap.push(tokens);
        }
    }

    let expanded = quote! {
        impl #ident {
            #(#func_wrap)*
        }
    };
    print_debug_info(&expanded, ExtraInfo::new(None, None));

    quote! {
        #item
        #expanded
    }
}

fn build_proxy_method(self_ty: &Box<Type>, method: &ImplItemFn) -> Option<TokenStream> {
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
                    provider.#ident(#(#arg_names),*).await
                }
            } else {
                quote! {
                    provider.#ident(#(#arg_names),*)
                }
            };

            if receiver.mutability.is_some() {
                Some(quote! {
                    pub async fn #ident #generics (
                        &self, #(#args),*
                    ) #output {
                        let instance = self.inner.get_instance().await;
                        let mut guard = instance.write().await;
                        let provider = self.inner.downcast_mut(guard.as_mut());
                        #invoke
                    }
                })
            } else {
                Some(quote! {
                    pub async fn #ident #generics (
                        &self, #(#args),*
                    ) #output {
                        let instance = self.inner.get_instance().await;
                        let guard = instance.read().await;
                        let provider = self.inner.downcast_ref(guard.as_ref());
                        #invoke
                    }
                })
            }
        }

        _ => None,
    }
}
