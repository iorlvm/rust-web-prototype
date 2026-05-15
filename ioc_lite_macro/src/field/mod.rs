use syn::{Attribute, Field, FieldsNamed, Ident, Result};

mod default;
mod proxy;
mod script;
mod value;

pub type BoxFieldIR = Box<dyn FieldIR>;
pub type BoxFieldIRFactory = Box<dyn FieldIRFactory>;

pub trait FieldIRFactory {
    fn match_field(&self, attr: &Attribute) -> bool;

    fn extract_field_attr(&self, field: &Field, attr: &Attribute) -> Result<BoxFieldIR>;

    fn create(&self, filed: &Field) -> Option<Result<BoxFieldIR>> {
        for attr in &filed.attrs {
            if !self.match_field(attr) {
                continue;
            }

            return Some(self.extract_field_attr(filed, attr));
        }
        None
    }
}

pub trait FieldIR {
    fn where_bound(&self) -> Option<proc_macro2::TokenStream> {
        None
    }

    fn initializer(&self) -> proc_macro2::TokenStream;
}

pub fn extract_field_irs(struct_name: &Ident, fields: FieldsNamed) -> Result<Vec<BoxFieldIR>> {
    let mut result = Vec::new();

    for field in &fields.named {
        result.push(extract_field_ir(field)?);
    }

    if result.is_empty() {
        return Err(syn::Error::new_spanned(
            struct_name,
            "#[derive(Component)] stateless structs should not be managed by IoC",
        ));
    }

    Ok(result)
}

fn extract_field_ir(field: &Field) -> Result<BoxFieldIR> {
    let mut result = None;

    let all_factory: Vec<BoxFieldIRFactory> = vec![
        Box::new(proxy::ProxyFieldIRFactory),
        Box::new(value::ValueFieldIRFactory),
        Box::new(script::ScriptFieldIRFactory),
    ];

    for factory in all_factory {
        let tmp = factory.create(field);
        if tmp.is_none() {
            continue;
        }

        if result.is_some() {
            return Err(syn::Error::new_spanned(field, "duplicate field attribute"));
        }
        result = tmp;
    }

    result.unwrap_or_else(|| Ok(Box::new(default::DefaultFieldIR::new(field))))
}
