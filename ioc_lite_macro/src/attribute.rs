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
    Script(Expr),
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

            let expr = match &attr.meta {
                Meta::List(_) => attr.parse_args::<Expr>()?,
                _ => {
                    return Err(Error::new_spanned(attr, "expected #[script(fn)]"));
                }
            };

            flag = FieldAttribute::Script(expr);
        }
    }

    Ok(flag)
}

/// 從 Bean<T> 提取 T
pub fn extract_bean_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident != "Bean" {
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
