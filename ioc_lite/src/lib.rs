mod lifecycle;
mod types;

pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;
pub use lifecycle::*;
pub use types::*;

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// 自動註冊機制
inventory::collect!(ComponentRegistration);
pub struct ComponentRegistration {
    pub register: fn(builder: &mut IoCBuilder) -> (),
}

type Item = (String, LifecycleScopeInstance, ComponentInitTrigger);

// 將 register 與 runtime 分離, 降低資源管理複雜度
pub struct IoCBuilder {
    map: HashMap<TypeId, Item>,
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

    pub fn register<T>(&mut self, init_trigger: ComponentInitTrigger, scope: impl LifecycleScope)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            (
                std::any::type_name::<T>().to_string(),
                Arc::new(RwLock::new(scope)),
                init_trigger,
            ),
        );
    }

    pub async fn build(self) -> IoC {
        let ioc = IoC {
            map: Arc::new(self.map),
        };
        ioc.on_build().await;
        ioc
    }
}

pub struct IoC {
    map: Arc<HashMap<TypeId, Item>>,
}

impl Clone for IoC {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl IoC {
    pub async fn get<T>(&self, scope_id: ScopeId) -> Bean<T>
    where
        T: Component,
    {
        let (_, scope, _) = self
            .map
            .get(&TypeId::of::<T>())
            .expect("component not registered");

        let ioc = self.clone();
        let factory: ComponentFactory = Arc::new(move |scope_id| {
            let ioc = ioc.clone();
            let scope_id = scope_id.clone();
            Box::pin(async move {
                let boxed = Box::new(T::create(ioc, scope_id).await) as Box<Object>;
                Arc::new(RwLock::new(boxed)) as Bean<Object>
            })
        });

        let hit = { scope.read().await.peek(scope_id.clone(), &factory).await };

        let instance = match hit {
            Some(instance) => instance,
            None => {
                let mut scope = scope.write().await;
                scope.resolve(scope_id, &factory).await
            }
        };

        let raw = Arc::into_raw(instance);
        let instance: Bean<T> = unsafe { Arc::from_raw(raw as *const RwLock<Box<T>>) };

        instance.clone()
    }

    pub async fn trigger<T>(&self, scope_id: ScopeId)
    where
        T: Component,
    {
        let _ = self.get::<T>(scope_id).await;
    }

    pub async fn destroy<T>(&self, scope_id: ScopeId)
    where
        T: Component,
    {
        if let Some((_, scope, _)) = self.map.get(&TypeId::of::<T>()) {
            scope.write().await.destroy(scope_id).await;
        }
    }

    // test script
    pub async fn run_test(&self) {
        let scope_id = Arc::new(IOC_SCOPE_KEY.to_string());

        println!("Running test script...");
        for (name, scope, init_trigger) in self.map.values() {
            self.handle_action(scope_id.clone(), Action::Trigger, scope, init_trigger)
                .await;
            println!("- Triggered: {}", name)
        }
        for (name, scope, init_trigger) in self.map.values() {
            self.handle_action(scope_id.clone(), Action::Destroy, scope, init_trigger)
                .await;
            println!("- Destroyed: {}", name)
        }
        println!("Test script finished");

        println!("Rebuilding IoC...");
        self.on_build().await;
    }

    // hooks
    async fn on_build(&self) {
        for (_, scope, init_trigger) in self.map.values() {
            let action = { scope.read().await.on_build() };
            self.handle_action(
                Arc::new(IOC_SCOPE_KEY.to_string()),
                action,
                scope,
                init_trigger,
            )
            .await;
        }
    }

    // utils
    async fn handle_action(
        &self,
        scope_id: ScopeId,
        action: Action,
        scope: &LifecycleScopeInstance,
        init_trigger: &ComponentInitTrigger,
    ) {
        match action {
            Action::Trigger => init_trigger(self.clone(), scope_id).await,
            Action::Destroy => scope.write().await.destroy(scope_id).await,
            Action::None => (),
        }
    }
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
    async fn create(ioc: IoC, scope_id: ScopeId) -> Self;
}
