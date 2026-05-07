use crate::{IoC, LifecycleScope};
use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Object = dyn Any + Send + Sync;
pub type Bean<T> = Arc<RwLock<Box<T>>>;

pub type LifecycleScopeInstance = Arc<RwLock<dyn LifecycleScope>>;
pub type ComponentInitTrigger = fn(ioc: IoC) -> Pin<Box<dyn Future<Output = ()> + Send>>;
pub type ComponentFactory =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Bean<Object>> + Send>> + Send + Sync>;
