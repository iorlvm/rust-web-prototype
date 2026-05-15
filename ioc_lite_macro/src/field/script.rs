use crate::field::{BoxFieldIR, FieldIR, FieldIRFactory};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Error, Expr, Field, Meta};

pub struct ScriptFieldIRFactory;

impl FieldIRFactory for ScriptFieldIRFactory {
    fn match_field(&self, attr: &Attribute) -> bool {
        attr.path().is_ident("script")
    }

    fn extract_field_attr(&self, field: &Field, attr: &Attribute) -> syn::Result<BoxFieldIR> {
        let args = match &attr.meta {
            Meta::List(meta_list) => meta_list.parse_args_with(
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated,
            )?,
            _ => {
                return Err(Error::new_spanned(attr, "expected #[script(fn, ...)]"));
            }
        };

        let mut iter = args.into_iter();

        let fn_expr = iter
            .next()
            .ok_or_else(|| Error::new_spanned(attr, "missing script function"))?;

        let mut with_cache = false;
        for arg in iter {
            match arg {
                Expr::Path(expr_path) if expr_path.path.is_ident("cache") => {
                    with_cache = true;
                }
                _ => {
                    return Err(Error::new_spanned(attr, "unknown script flag"));
                }
            }
        }

        Ok(Box::new(ScriptIR {
            field_name: field.ident.clone().unwrap(),
            fn_expr,
            with_cache,
        }))
    }
}

struct ScriptIR {
    field_name: syn::Ident,
    fn_expr: Expr,
    with_cache: bool,
}

impl FieldIR for ScriptIR {
    fn initializer(&self) -> TokenStream {
        let field_name = &self.field_name;
        let fn_expr = &self.fn_expr;

        if self.with_cache {
            quote! { #field_name: scope.run_script_with_cache(#fn_expr).await }
        } else {
            quote! { #field_name: scope.run_script(#fn_expr).await }
        }
    }
}
