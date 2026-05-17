use crate::internal::IoCBuilderScript;
use crate::{Component, ComponentDefinition, IoC, Lifecycle};
use dashmap::DashSet;
use std::any::TypeId;
use std::collections::HashMap;

// 自動註冊機制
inventory::collect!(ComponentRegistration);
pub struct ComponentRegistration {
    pub register: fn(builder: &mut IoCBuilder) -> (),
}

#[derive(Clone)]
pub struct IoCBuilder {
    script_input: Option<serde_json::Value>,
    map: HashMap<TypeId, ComponentDefinition>,
}

impl IoCBuilder {
    pub fn new() -> Self {
        Self {
            script_input: None,
            map: HashMap::new(),
        }
    }

    pub fn auto_register(&mut self) {
        inventory::iter::<ComponentRegistration>
            .into_iter()
            .for_each(|reg| (reg.register)(self));
    }

    pub fn register<T: Component>(&mut self, lifecycle: Lifecycle)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            ComponentDefinition {
                lifecycle,
                name: &std::any::type_name::<T>(),
                warmup_fn: |scope| {
                    Box::pin(async move {
                        scope.force_warmup::<T>().await;
                    })
                },
            },
        );
    }

    pub fn script_input(mut self, script_input: serde_json::Value) -> Self {
        self.script_input = Some(script_input);
        self
    }

    pub async fn build_with_test(self) -> IoC {
        if self.map.is_empty() {
            println!("[IoC] No registered components found, test script cannot be executed.");
        } else {
            let cloned = self.clone();

            let test_ioc = IoC::new(
                cloned.script_input.unwrap_or_else(|| serde_json::json!({})),
                cloned.map,
                Some(DashSet::new()),
            );
            test_ioc.run_test().await;
        }

        self.build().await
    }

    pub async fn build(self) -> IoC {
        if self.map.is_empty() {
            println!(
                "[IoC] No lifecycle metadata registered, components will use Prototype fallback."
            );
        } else {
            println!("[IoC] Note: Unregistered components will use Prototype lifecycle.");
        }
        let ioc = IoC::new(
            self.script_input.unwrap_or_else(|| serde_json::json!({})),
            self.map,
            None,
        );
        ioc.on_build().await;
        ioc
    }
}
