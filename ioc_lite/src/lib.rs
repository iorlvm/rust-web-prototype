mod builder;
mod core;
mod internal;

pub use builder::{ComponentRegistration, IoCBuilder};
pub use core::*;

pub use regex::Regex;
pub use async_trait::async_trait;
pub use ioc_lite_macro::{proxy_method, Component};

use std::any::Any;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

// types
pub type Object = dyn Any + Send + Sync;
pub type BeanInstance = Arc<RwLock<Box<Object>>>;
pub type CacheInstance = RwLock<OnceCell<BeanInstance>>;

pub type ComponentWarmupFn = fn(scope: Arc<Scope>) -> Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone)]
pub struct ComponentDefinition {
    pub name: &'static str,
    pub lifecycle: Lifecycle,
    pub warmup_fn: ComponentWarmupFn,
}

#[derive(Clone)]
pub enum InitMode {
    Eager,
    Lazy,
}

#[derive(Clone)]
pub enum Lifecycle {
    Prototype,
    Singleton(InitMode),
    Scoped(Regex),
}

#[async_trait]
pub trait Component: Sized + Send + Sync + 'static {
    type ProxyStruct: Sized;
    fn proxy(input: Bean<Self>) -> Self::ProxyStruct;
    async fn create(scope: Arc<Scope>) -> Self;
}

pub struct Proxy<T: Component> {
    inner: T::ProxyStruct,
}

impl<T: Component> Proxy<T> {
    pub fn new(inner: T::ProxyStruct) -> Self {
        Self { inner }
    }
}

impl<T: Component> Deref for Proxy<T> {
    type Target = T::ProxyStruct;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
