use crate::attribute::{extract_bean_inner_type, get_field_attr, FieldAttribute};

use proc_macro2::Ident;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Fields, FieldsNamed, Generics, Lit, Result};

/// 展開 Component derive 實作
///
/// 主要流程：
/// 1. 驗證 struct 類型
/// 2. 禁止 generic
/// 3. 解析 fields
/// 4. 根據 attribute 生成初始化邏輯
/// 5. 組裝 where bounds
/// 6. 生成 Component trait impl
/// 7. 生成 inventory registration
pub fn expand_component(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
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

    let scope_value = get_scope_value(input.attrs.as_slice())?;
    match fields {
        Fields::Unit => expand_unit_struct_component(&scope_value, struct_name, generics),
        Fields::Named(fields_named) => {
            expand_named_struct_component(&scope_value, struct_name, generics, fields_named)
        }
        Fields::Unnamed(_) => Err(Error::new_spanned(
            struct_name,
            "#[derive(Component)] does not support tuple structs yet",
        )),
    }
}

fn expand_named_struct_component(
    scope: &Scope,
    struct_name: Ident,
    generics: Generics,
    fields: FieldsNamed,
) -> Result<proc_macro2::TokenStream> {
    // 每個 field 會轉換成：
    // 1. where bound（型別限制）
    // 2. initializer（建構邏輯）
    let mut where_bounds = Vec::new();
    let mut field_initializers = Vec::new();
    for field in fields.named {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new_spanned(&field, "expected named field"))?;

        let field_type = field.ty;

        match get_field_attr(&field.attrs)? {
            FieldAttribute::Component => {
                let component_type = match extract_bean_inner_type(&field_type) {
                    Some(ty) => ty,
                    None => {
                        return Err(Error::new_spanned(
                            &field_type,
                            "component type must be Bean<T>",
                        ));
                    }
                };
                // 加入 IoC bound（確保該型別可被 IoC 管理）
                where_bounds.push(quote! {
                    #component_type: ::ioc_lite::Component
                });

                // 生成 IoC 取用邏輯
                field_initializers.push(quote! {
                    #field_name: ioc.get::<#component_type>().await
                });
            }
            FieldAttribute::None => {
                // 沒有標註 => 使用 Default
                where_bounds.push(quote! {
                    #field_type: ::std::default::Default
                });

                field_initializers.push(quote! {
                    #field_name: <#field_type as ::std::default::Default>::default()
                });
            }
            FieldAttribute::Value(value) => {
                // 支援 literal injection
                // - string => From::from
                // - 其他 => 原樣塞入
                let initializers = if let Lit::Str(lit_str) = value {
                    quote! { #field_name: ::std::convert::From::from(#lit_str) }
                } else {
                    quote! { #field_name: #value }
                };

                field_initializers.push(initializers);
            }
            FieldAttribute::Script(func) => {
                field_initializers.push(quote! { #field_name: (#func)(ioc.clone()).await });
            }
        }
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    // 保留使用者原本 where + macro 產生條件
    let existing_where_predicates = where_clause
        .map(|where_clause| {
            let predicates = &where_clause.predicates;
            quote! {
                #predicates,
            }
        })
        .unwrap_or_default();

    // 最終輸出（Component impl + registration）
    let registration = expand_component_registration(&scope, &struct_name);
    let expanded = quote! {
        #[::ioc_lite::async_trait]
        impl #impl_generics ::ioc_lite::Component for #struct_name #type_generics
        where
            #existing_where_predicates
            #(#where_bounds,)*
        {
            async fn create(ioc: ::ioc_lite::IoC) -> Self {
                Self {
                    #(#field_initializers,)*
                }
            }
        }

        #registration
    };

    Ok(expanded)
}

/// unit struct（無 field）
///
/// - 直接回傳 Self
/// - 不做 IoC 注入
fn expand_unit_struct_component(
    scope: &Scope,
    struct_name: Ident,
    generics: Generics,
) -> Result<proc_macro2::TokenStream> {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let registration = expand_component_registration(&scope, &struct_name);
    let expanded = quote! {
        #[::ioc_lite::async_trait]
        impl #impl_generics ::ioc_lite::Component for #struct_name #type_generics #where_clause {
            async fn create(_ioc: ::ioc_lite::IoC) -> Self {
                Self
            }
        }

        #registration
    };

    Ok(expanded)
}

/// 將 Component 註冊進 inventory
///
/// 作用：
/// - runtime 掃描所有 Component
/// - IoC 容器可動態建立 instance
fn expand_component_registration(scope: &Scope, struct_name: &Ident) -> proc_macro2::TokenStream {
    let scope_token = match scope {
        Scope::Prototype => quote! { ::ioc_lite::PrototypeScope::default() },
        Scope::Singleton(mode) => match mode {
            InitMode::Eager => quote! { ::ioc_lite::SingletonScope::eager() },
            InitMode::Lazy => quote! { ::ioc_lite::SingletonScope::lazy() },
        },
    };

    quote! {
        ::inventory::submit! {
            ::ioc_lite::ComponentRegistration {
                register: |builder| {
                    builder.register::<#struct_name>(
                        |ioc| {
                            Box::pin(async move {
                                let _ = ioc.get::<#struct_name>().await;
                            })
                        },
                        #scope_token
                    );
                },
            }
        }
    }
}

enum InitMode {
    Eager,
    Lazy,
}

enum Scope {
    Singleton(InitMode),
    Prototype,
}

fn get_scope_value(attrs: &[Attribute]) -> Result<Scope> {
    attrs
        .iter()
        .find_map(|attr| {
            if !attr.path().is_ident("scope") {
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
        .map(|v| match v.as_str() {
            "prototype" => Ok(Scope::Prototype),
            "singleton" => Ok(Scope::Singleton(InitMode::Eager)),
            "lazy_singleton" => Ok(Scope::Singleton(InitMode::Lazy)),
            _ => Err(Error::new_spanned(
                v,
                "invalid scope, expected 'singleton'|'lazy_singleton'|'prototype'",
            )),
        })
        .unwrap_or(Ok(Scope::Singleton(InitMode::Eager)))
}
