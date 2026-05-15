use quote::quote;
use regex::Regex;
use syn::{Attribute, Error, Ident, Lit, Result};

pub enum InitMode {
    Eager,
    Lazy,
}
impl InitMode {
    pub fn token(&self) -> proc_macro2::TokenStream {
        match self {
            InitMode::Eager => {
                quote! { ::ioc_lite::InitMode::Eager }
            }
            InitMode::Lazy => {
                quote! { ::ioc_lite::InitMode::Lazy }
            }
        }
    }
}
pub enum RegistrationIR {
    Prototype(Ident),
    Singleton(Ident, InitMode),
    Scoped(Ident, Regex),
}
impl RegistrationIR {
    pub fn from(struct_name: &Ident, attrs: &[Attribute]) -> Result<Self> {
        let attr = extract_lifecycle(attrs);

        let struct_name = struct_name.clone();

        let ir = match attr.as_deref() {
            None | Some("Singleton") => {
                RegistrationIR::Singleton(struct_name, InitMode::Eager)
            }
            Some("Prototype") => RegistrationIR::Prototype(struct_name),
            Some("Singleton(Lazy)") => {
                RegistrationIR::Singleton(struct_name, InitMode::Lazy)
            }
            Some(name_format) => {
                let regex = Regex::new(name_format).map_err(|e| {
                    Error::new_spanned(name_format, format!("invalid lifecycle regex: {}", e))
                })?;

                RegistrationIR::Scoped(struct_name, regex)
            }
        };

        Ok(ir)
    }

    pub fn token(&self) -> proc_macro2::TokenStream {
        let (struct_name, lifecycle) = match self {
            RegistrationIR::Prototype(struct_name) => {
                (struct_name, quote! { ::ioc_lite::Lifecycle::Prototype })
            }
            RegistrationIR::Singleton(struct_name, mode) => {
                let mode = mode.token();
                (
                    struct_name,
                    quote! { ::ioc_lite::Lifecycle::Singleton(#mode) },
                )
            }
            RegistrationIR::Scoped(struct_name, name_format) => {
                let name_format = name_format.as_str();
                (
                    struct_name,
                    quote! { ::ioc_lite::Lifecycle::Scoped(::ioc_lite::Regex::new(#name_format).unwrap()) },
                )
            }
        };

        quote! {
            ::inventory::submit! {
                ::ioc_lite::ComponentRegistration {
                    register: |builder| {
                        builder.register::<#struct_name>(#lifecycle);
                    },
                }
            }
        }
    }
}

fn extract_lifecycle(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("lifecycle") {
            return None;
        }

        match &attr.meta {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
                None
            }
            _ => None,
        }
    })
}