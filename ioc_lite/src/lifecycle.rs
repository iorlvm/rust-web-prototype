use crate::{ComponentFactory, Object, Shared};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum Action {
    Trigger,
    Destroy,
    None,
}

#[async_trait]
pub trait LifecycleScope: Send + Sync + 'static {
    async fn peek(&self, factory: &ComponentFactory) -> Option<Shared<Object>>;
    async fn resolve(&mut self, factory: &ComponentFactory) -> Shared<Object>;
    async fn destroy(&mut self);

    // lifecycle hook
    fn on_build(&self) -> Action {
        Action::None
    }
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

    async fn destroy(&mut self) {}
}

pub struct SingletonScope {
    lazy: bool,
    instance: Option<Shared<Object>>,
}

impl SingletonScope {
    pub fn lazy() -> Self {
        Self {
            lazy: true,
            instance: None,
        }
    }

    pub fn eager() -> Self {
        Self {
            lazy: false,
            instance: None,
        }
    }
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

    async fn destroy(&mut self) {
        self.instance = None;
    }

    fn on_build(&self) -> Action {
        if self.lazy {
            Action::None
        } else {
            Action::Trigger
        }
    }
}
