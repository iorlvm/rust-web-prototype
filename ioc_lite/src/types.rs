use crate::{IoC, LifecycleScope};
use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Object = Box<dyn Any + Send + Sync>;
pub type Shared<T> = Arc<RwLock<T>>;
pub type Bean<T> = Shared<Box<T>>;

pub type LifecycleScopeInstance = Shared<dyn LifecycleScope>;
pub type ComponentInitTrigger = fn(ioc: IoC) -> Pin<Box<dyn Future<Output = ()> + Send>>;
pub type ComponentFactory =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Object> + Send>> + Send + Sync>;
