mod builder;
mod core;
mod internal;

pub use builder::{ComponentRegistration, IoCBuilder};
pub use core::*;

pub use async_trait::async_trait;
pub use ioc_lite_macro::{proxy_method, Component};

use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

// types
pub type Object = dyn Any + Send + Sync;
pub type BeanInstance<T> = Arc<RwLock<Box<T>>>;

pub type ComponentForceWarmupFn = fn(ioc: IoC) -> Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Item = (
    String,
    ScopeType,
    ComponentForceWarmupFn,
    RwLock<Option<BeanInstance<Object>>>,
);

#[derive(Clone)]
pub enum InitMode {
    Eager,
    Lazy,
}

#[derive(Clone)]
pub enum ScopeType {
    Prototype,
    Singleton(InitMode),
}

#[async_trait]
pub trait Component: Sized + Send + Sync + 'static {
    type Output;
    fn proxy(input: Bean<Self>) -> Self::Output;
    async fn create(ioc: IoC) -> Self;
}
