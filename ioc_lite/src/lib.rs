pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

type Object = Box<dyn Any + Send + Sync>;
pub type Shared<T> = Arc<RwLock<T>>;
pub type Bean<T> = Shared<Box<T>>;

type LifecycleScopeInstance = Shared<dyn LifecycleScope>;
type ComponentFactory = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Object> + Send>> + Send + Sync>;

inventory::collect!(ComponentRegistration);
pub struct ComponentRegistration {
    pub register: fn(builder: &mut IoCBuilder) -> (),
}
pub struct IoCBuilder {
    map: HashMap<TypeId, LifecycleScopeInstance>,
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

    pub fn register<T>(&mut self, scope: impl LifecycleScope)
    where
        T: Component,
    {
        self.map
            .insert(TypeId::of::<T>(), Arc::new(RwLock::new(scope)));
    }

    pub fn build(self) -> IoC {
        IoC {
            map: Arc::new(self.map),
        }
    }
}

pub struct IoC {
    map: Arc<HashMap<TypeId, LifecycleScopeInstance>>,
}

impl Clone for IoC {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl IoC {
    pub async fn get<T>(&self) -> Bean<T>
    where
        T: Component,
    {
        let scope = self
            .map
            .get(&TypeId::of::<T>())
            .expect("component not registered");

        let ioc = self.clone();
        let factory: ComponentFactory = Arc::new(move || {
            let ioc = ioc.clone();
            Box::pin(async move { Box::new(T::create(ioc).await) as Object })
        });

        let hit = { scope.read().await.peek(&factory).await };

        let instance = match hit {
            Some(instance) => instance,
            None => {
                let mut scope = scope.write().await;
                scope.resolve(&factory).await
            }
        };

        let instance: Bean<T> = unsafe { std::mem::transmute(instance) };

        instance.clone()
    }
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
    async fn create(ioc: IoC) -> Self;
}

#[async_trait]
pub trait LifecycleScope: Send + Sync + 'static {
    async fn peek(&self, factory: &ComponentFactory) -> Option<Shared<Object>>;

    async fn resolve(&mut self, factory: &ComponentFactory) -> Shared<Object>;
}

#[derive(Default)]
pub struct PrototypeScope;
#[async_trait]
impl LifecycleScope for PrototypeScope {
    async fn peek(&self, factory: &ComponentFactory) -> Option<Shared<Object>> {
        Some(Arc::new(RwLock::new(factory().await)))
    }

    async fn resolve(&mut self, _: &ComponentFactory) -> Shared<Object> {
        unreachable!()
    }
}

#[derive(Default)]
pub struct SingletonScope {
    instance: Option<Shared<Object>>,
}

#[async_trait]
impl LifecycleScope for SingletonScope {
    async fn peek(&self, _: &ComponentFactory) -> Option<Shared<Object>> {
        self.instance.clone()
    }

    async fn resolve(&mut self, factory: &ComponentFactory) -> Shared<Object> {
        if let Some(instance) = &self.instance {
            return instance.clone();
        }

        let obj = factory().await;
        let instance = Arc::new(RwLock::new(obj));
        self.instance = Some(instance.clone());

        instance
    }
}
