use crate::field::{extract_field_irs, BoxFieldIR};
use crate::registration::RegistrationIR;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields, Generics, Ident, Result};

pub struct ComponentIR {
    struct_name: Ident,
    generics: Generics,
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

        // 禁止 generic（避免 IoC 推導複雜化）
        if !generics.params.is_empty() {
            return Err(Error::new_spanned(
                generics,
                "#[derive(Component)] does not support generic structs",
            ));
        }

        let registration = RegistrationIR::from(
            &struct_name,
            generics.params.len() > 0,
            input.attrs.as_slice(),
        )?;

        if let Fields::Named(fields_named) = fields {
            let fields = extract_field_irs(&struct_name, fields_named)?;
            Ok(Self {
                struct_name,
                generics,
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
        let proxy_ident = proxy_struct_ident(&self.struct_name);
        let proxy_struct = self.proxy_struct(&proxy_ident);
        let registration = self.registration.token();

        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        // 保留使用者原本 where + macro 產生條件
        let existing_where_predicates = where_clause
            .map(|where_clause| {
                let predicates = &where_clause.predicates;
                quote! { #predicates, }
            })
            .unwrap_or_default();

        let where_bounds = &self
            .fields
            .iter()
            .map(|field| field.where_bound())
            .filter(|bound| bound.is_some())
            .map(|bound| bound.unwrap())
            .collect::<Vec<_>>();

        let field_initializers = &self
            .fields
            .iter()
            .map(|field| field.initializer())
            .collect::<Vec<_>>();

        quote! {
            #proxy_struct

            #[::ioc_lite::async_trait]
            impl #impl_generics ::ioc_lite::Component for #struct_name #type_generics
            where
                #existing_where_predicates
                #(#where_bounds,)*
            {
                type ProxyStruct = #proxy_ident;

                fn proxy(input: ::ioc_lite::Bean<Self>) -> Self::ProxyStruct {
                    #proxy_ident {
                        inner: input
                    }
                }

                async fn create(scope: std::sync::Arc<::ioc_lite::Scope>) -> Self {
                    Self {
                        #(#field_initializers,)*
                    }
                }
            }

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

pub fn proxy_struct_ident(struct_name: &Ident) -> Ident {
    format_ident!("__PROXY_{}", struct_name.to_string().to_uppercase())
}
