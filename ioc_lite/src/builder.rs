use crate::internal::IoCBuilderScript;
use crate::{Component, IoC, Item, ScopeType};
use std::any::TypeId;
use std::collections::HashMap;
use tokio::sync::RwLock;

// 自動註冊機制
inventory::collect!(ComponentRegistration);
pub struct ComponentRegistration {
    pub register: fn(builder: &mut IoCBuilder) -> (),
}

// 將 register 與 runtime 分離, 降低資源管理複雜度
pub struct IoCBuilder {
    map: HashMap<TypeId, Item>,
}

impl Clone for IoCBuilder {
    fn clone(&self) -> Self {
        Self {
            map: self
                .map
                .iter()
                .map(|(type_id, (name, scope_type, force_warmup, _))| {
                    (
                        *type_id,
                        (
                            name.clone(),
                            scope_type.clone(),
                            *force_warmup,
                            RwLock::new(None),
                        ),
                    )
                })
                .collect::<HashMap<TypeId, Item>>(),
        }
    }
}

impl IoCBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            map: HashMap::new(),
        };

        inventory::iter::<ComponentRegistration>
            .into_iter()
            .for_each(|reg| (reg.register)(&mut builder));

        builder
    }

    pub fn register<T>(&mut self, scope_type: ScopeType)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            (
                std::any::type_name::<T>().to_string(),
                scope_type,
                |ioc| {
                    Box::pin(async move {
                        ioc.force_warmup::<T>().await;
                    })
                },
                RwLock::new(None),
            ),
        );
    }

    pub async fn build_with_test(self) -> IoC {
        let test_ioc = IoC::new(self.clone().map).into_test_mode();
        test_ioc.run_test().await;

        self.build().await
    }

    pub async fn build(self) -> IoC {
        let ioc = IoC::new(self.map);
        ioc.on_build().await;
        ioc
    }
}
