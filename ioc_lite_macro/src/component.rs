use crate::field::{extract_field_irs, BoxFieldIR};
use crate::registration::RegistrationIR;
use crate::utils::{print_debug_info, proxy_struct_ident, ExtraInfo};
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Result};

pub struct ComponentIR {
    struct_name: Ident,
    fields: Vec<BoxFieldIR>,
    registration: RegistrationIR,
}

impl ComponentIR {
    pub fn from(input: DeriveInput) -> Result<Self> {
        let struct_name = input.ident;
        let generics = input.generics;

        // 限制只支援 struct
        let fields = match input.data {
            Data::Struct(data_struct) => data_struct.fields,
            _ => {
                return Err(Error::new_spanned(
                    struct_name,
                    "#[derive(Component)] only supports structs",
                ));
            }
        };

        if !generics.params.is_empty() {
            return Err(Error::new_spanned(
                generics,
                "#[derive(Component)] does not support generic structs",
            ));
        }

        let registration = RegistrationIR::from(&struct_name, input.attrs.as_slice())?;

        if let Fields::Named(fields_named) = fields {
            let fields = extract_field_irs(&struct_name, fields_named)?;
            Ok(Self {
                struct_name,
                fields,
                registration,
            })
        } else {
            Err(Error::new_spanned(
                struct_name,
                "#[derive(Component)] only supported for named structs",
            ))
        }
    }

    pub fn token(&self) -> proc_macro2::TokenStream {
        let struct_name = &self.struct_name;
        let proxy_ident = proxy_struct_ident(&self.struct_name.to_string());
        let proxy_struct = self.proxy_struct(&proxy_ident);
        let registration = self.registration.token();

        let field_initializers = &self
            .fields
            .iter()
            .map(|field| field.initializer())
            .collect::<Vec<_>>();

        let expanded = quote! {
            impl ::ioc_lite::Component for #struct_name {
                type ProxyStruct = #proxy_ident;

                fn proxy(input: ::ioc_lite::Bean<Self>) -> Self::ProxyStruct {
                    #proxy_ident {
                        inner: input
                    }
                }

                fn create(scope: std::sync::Arc<::ioc_lite::Scope>) -> impl Future<Output = Self> + Send {
                    async move {
                        Self {
                            #(#field_initializers,)*
                        }
                    }
                }
            }
            #proxy_struct

        };

        print_debug_info(
            &expanded,
            ExtraInfo::new(
                Some(format!(
                    "#[derive(Component)]\npub struct {} {}",
                    struct_name, "{ ... }"
                )),
                None,
            ),
        );

        quote! {
            #expanded
            #registration
        }
    }

    fn proxy_struct(&self, proxy_ident: &syn::Ident) -> proc_macro2::TokenStream {
        let struct_name = &self.struct_name;
        quote! {
            pub struct #proxy_ident {
                inner: ::ioc_lite::Bean<#struct_name>
            }
        }
    }
}
