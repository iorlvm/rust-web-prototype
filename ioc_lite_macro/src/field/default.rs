use crate::field::FieldIR;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Field, Type};

pub struct DefaultFieldIR {
    field_type: Type,
    field_name: Ident,
}

impl DefaultFieldIR {
    pub fn new(field: &Field) -> Self {
        Self {
            field_type: field.ty.clone(),
            field_name: field.ident.clone().unwrap(),
        }
    }
}

impl FieldIR for DefaultFieldIR {
    fn initializer(&self) -> TokenStream {
        let field_name = &self.field_name;
        let field_type = &self.field_type;
        quote! {
            #field_name: <#field_type as ::std::default::Default>::default()
        }
    }
}
