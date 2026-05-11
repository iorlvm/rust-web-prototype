use syn::{Attribute, Error, Expr, GenericArgument, Lit, Meta, PathArguments, Result, Type};

/// 解析 field attribute
///
/// 支援：
/// - #[component]
/// - #[value = "..."]
///
/// 限制：
/// - 同一 field 不可同時存在多個標註
pub enum FieldAttribute {
    Component,
    Value(Lit),
    Script(Expr, bool),
    None,
}
impl FieldAttribute {
    pub fn is_some(&self) -> bool {
        !matches!(self, FieldAttribute::None)
    }
}

pub fn get_field_attr(attrs: &[Attribute]) -> Result<FieldAttribute> {
    let mut flag = FieldAttribute::None;

    for attr in attrs {
        if attr.path().is_ident("component") {
            if flag.is_some() {
                return Err(Error::new_spanned(attr, "duplicate attribute"));
            }
            match &attr.meta {
                Meta::Path(_) => {
                    flag = FieldAttribute::Component;
                }
                _ => {
                    return Err(Error::new_spanned(
                        attr,
                        "unsupported component attribute, expected #[component]",
                    ));
                }
            }
        } else if attr.path().is_ident("value") {
            if flag.is_some() {
                return Err(Error::new_spanned(attr, "duplicate attribute"));
            }
            match &attr.meta {
                Meta::NameValue(nv) => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        flag = FieldAttribute::Value(expr_lit.lit.clone());
                    } else {
                        return Err(Error::new_spanned(
                            attr,
                            "expected literal, e.g. #[value = 123] or #[value = \"abc\"]",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(attr, "expected #[value = ...]"));
                }
            }
        } else if attr.path().is_ident("script") {
            if flag.is_some() {
                return Err(Error::new_spanned(attr, "duplicate attribute"));
            }

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

            let mut cache = false;

            for arg in iter {
                match arg {
                    Expr::Path(expr_path) if expr_path.path.is_ident("cache") => {
                        cache = true;
                    }
                    _ => {
                        return Err(Error::new_spanned(attr, "unknown script flag"));
                    }
                }
            }

            flag = FieldAttribute::Script(fn_expr, cache);
        }
    }

    Ok(flag)
}

/// 從 Proxy<T> 提取 T
pub fn extract_proxy_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident != "Proxy" {
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
