pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ==========================
// Singleton
// ==========================
pub type SingletonInstance = Arc<dyn Any + Send + Sync>;
pub type SingletonCreateFuture<'a> = Pin<Box<dyn Future<Output = SingletonInstance> + Send + 'a>>;
pub type SingletonCreateFn = for<'a> fn(&'a mut IoC) -> SingletonCreateFuture<'a>;

#[async_trait]
pub trait Singleton: Send + Sync + 'static {
    async fn create(ioc: &mut IoC) -> Self;
}

pub struct SingletonRegistration {
    pub type_id: fn() -> TypeId,
    pub create: SingletonCreateFn,
}

inventory::collect!(SingletonRegistration);
fn registered_singleton() -> Vec<&'static SingletonRegistration> {
    inventory::iter::<SingletonRegistration>
        .into_iter()
        .collect()
}

// ==========================
// Prototype
// ==========================

pub type PrototypeInstance = Box<dyn Any + Send + Sync>;

pub type PrototypeCreateFuture<'a> = Pin<Box<dyn Future<Output = PrototypeInstance> + Send + 'a>>;

pub type PrototypeCreateFn = for<'a> fn(&'a mut IoC) -> PrototypeCreateFuture<'a>;

#[async_trait]
pub trait Prototype: Send + Sync + 'static {
    // build-time：允許修改 IoC（只在初始化階段使用）
    async fn build_time_create(ioc: &mut IoC) -> Self;

    // runtime：只讀 IoC（正常使用）
    async fn create(ioc: &IoC) -> Self;
}

pub struct PrototypeRegistration {
    pub type_id: fn() -> TypeId,
    pub create: PrototypeCreateFn,
}

inventory::collect!(PrototypeRegistration);

pub fn registered_prototype() -> Vec<&'static PrototypeRegistration> {
    inventory::iter::<PrototypeRegistration>
        .into_iter()
        .collect()
}

pub struct IoC {
    map: HashMap<TypeId, SingletonInstance>,
    constructing: HashSet<TypeId>,
}

// ==========================
// IoC
// ==========================
impl IoC {
    pub async fn new() -> Self {
        let mut instance = Self {
            map: HashMap::new(),
            constructing: HashSet::new(),
        };

        // prototype build-time check
        for prototype in registered_prototype() {
            let _ = (prototype.create)(&mut instance).await;
        }

        // build singleton graph
        for singleton in registered_singleton() {
            let key = (singleton.type_id)();

            if !instance.map.contains_key(&key) {
                instance.constructing.insert(key);
                let component = (singleton.create)(&mut instance).await;
                instance.map.insert(key, component);
                instance.constructing.remove(&key);
            }
        }

        instance
    }

    pub fn get<T>(&self) -> Arc<T>
    where
        T: Singleton,
    {
        let value = self
            .map
            .get(&TypeId::of::<T>())
            .cloned()
            .expect("component not found");

        value.downcast::<T>().expect("component type mismatch")
    }

    pub async fn create<T>(&self) -> T
    where
        T: Prototype,
    {
        T::create(self).await
    }

    pub async fn build_time_prototype<T>(&mut self) -> T
    where
        T: Prototype,
    {
        T::build_time_create(self).await
    }

    pub async fn build_time_singleton<T>(&mut self) -> Arc<T>
    where
        T: Singleton,
    {
        let key = TypeId::of::<T>();

        if !self.map.contains_key(&key) {
            if self.constructing.contains(&key) {
                panic!("IoC failed: circular dependency detected");
            }

            self.constructing.insert(key);
            let component = T::create(self).await;
            self.map.insert(key, Arc::new(component));
            self.constructing.remove(&key);
        }

        self.get::<T>()
    }
}
