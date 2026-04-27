pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ComponentInstance = Arc<dyn Any + Send + Sync>;

pub type ComponentCreateFuture<'a> = Pin<Box<dyn Future<Output = ComponentInstance> + Send + 'a>>;

pub type ComponentCreateFn = for<'a> fn(&'a mut IoC) -> ComponentCreateFuture<'a>;

#[async_trait]
pub trait Component: Send + Sync + 'static {
    async fn create(ioc: &mut IoC) -> Self;
}

pub struct ComponentRegistration {
    pub type_id: fn() -> TypeId,
    pub create: ComponentCreateFn,
}

inventory::collect!(ComponentRegistration);

pub fn registered_components() -> Vec<&'static ComponentRegistration> {
    inventory::iter::<ComponentRegistration>
        .into_iter()
        .collect()
}

pub struct IoC {
    map: HashMap<TypeId, ComponentInstance>,
    constructing: HashSet<TypeId>,
}

impl IoC {
    pub async fn new() -> Self {
        let mut instance = Self {
            map: HashMap::new(),
            constructing: HashSet::new(),
        };

        for registration in registered_components() {
            let key = (registration.type_id)();

            if !instance.map.contains_key(&key) {
                instance.constructing.insert(key);
                let component = (registration.create)(&mut instance).await;
                instance.map.insert(key, component);
                instance.constructing.remove(&key);
            }
        }

        instance
    }

    pub fn get<T>(&self) -> Arc<T>
    where
        T: Component,
    {
        let value = self
            .map
            .get(&TypeId::of::<T>())
            .cloned()
            .expect("component not found");

        value.downcast::<T>().expect("component type mismatch")
    }

    pub async fn get_or_insert<T>(&mut self) -> Arc<T>
    where
        T: Component,
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
