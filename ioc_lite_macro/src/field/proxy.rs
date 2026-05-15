use crate::field::{BoxFieldIR, FieldIR, FieldIRFactory};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, Error, Field, GenericArgument, Meta, PathArguments, Result, Type};

pub struct ProxyFieldIRFactory;

impl FieldIRFactory for ProxyFieldIRFactory {
    fn match_field(&self, attr: &Attribute) -> bool {
        attr.path().is_ident("component")
    }

    fn extract_field_attr(&self, field: &Field, attr: &Attribute) -> Result<BoxFieldIR> {
        match &attr.meta {
            Meta::Path(_) => {}
            _ => {
                return Err(Error::new_spanned(
                    attr,
                    "Unsupported attribute type for component field",
                ));
            }
        }

        match extract_proxy_inner_type(&field.ty) {
            Some(ty) => Ok(Box::new(ProxyIR {
                field_type: ty.clone(),
                field_name: field.ident.clone().unwrap(),
            })),
            None => Err(Error::new_spanned(
                &field.ty,
                "component type must be Proxy<T>",
            )),
        }
    }
}

struct ProxyIR {
    field_type: Type,
    field_name: Ident,
}

impl FieldIR for ProxyIR {
    fn initializer(&self) -> TokenStream {
        let field_name = &self.field_name;
        let field_type = &self.field_type;
        quote! {
            #field_name: scope.get::<#field_type>()
        }
    }
}

fn extract_proxy_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident != "::ioc_lite::Proxy" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    if arguments.args.len() != 1 {
        return None;
    }

    let GenericArgument::Type(inner_type) = arguments.args.first()? else {
        return None;
    };

    Some(inner_type)
}
