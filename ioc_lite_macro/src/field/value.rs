use crate::field::{BoxFieldIR, FieldIR, FieldIRFactory};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Error, Expr, Field, Lit, Meta};

pub struct ValueFieldIRFactory;

impl FieldIRFactory for ValueFieldIRFactory {
    fn match_field(&self, attr: &Attribute) -> bool {
        attr.path().is_ident("value")
    }

    fn extract_field_attr(&self, field: &Field, attr: &Attribute) -> syn::Result<BoxFieldIR> {
        match &attr.meta {
            Meta::NameValue(nv) => {
                if let Expr::Lit(expr_lit) = &nv.value {
                    Ok(Box::new(ValueIR {
                        field_name: field.ident.clone().unwrap(),
                        lit: expr_lit.lit.clone(),
                    }) as BoxFieldIR)
                } else {
                    Err(Error::new_spanned(
                        attr,
                        "expected literal, e.g. #[value = 123] or #[value = \"abc\"]",
                    ))
                }
            }
            _ => Err(Error::new_spanned(attr, "expected #[value = ...]")),
        }
    }
}

struct ValueIR {
    field_name: syn::Ident,
    lit: syn::Lit,
}

impl FieldIR for ValueIR {
    fn initializer(&self) -> TokenStream {
        let field_name = &self.field_name;
        match &self.lit {
            Lit::Str(str_lit) => quote! {
                #field_name: ::std::convert::From::from(#str_lit)
            },
            _ => {
                let lit = &self.lit;
                quote! { #field_name: #lit }
            }
        }
    }
}
