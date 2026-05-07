use crate::{Bean, ComponentFactory, Object};
use async_trait::async_trait;

pub enum Action {
    Trigger,
    Destroy,
    None,
}

#[async_trait]
pub trait LifecycleScope: Send + Sync + 'static {
    async fn peek(&self, factory: &ComponentFactory) -> Option<Bean<Object>>;
    async fn resolve(&mut self, factory: &ComponentFactory) -> Bean<Object>;
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
    async fn peek(&self, factory: &ComponentFactory) -> Option<Bean<Object>> {
        Some(factory().await)
    }

    async fn resolve(&mut self, _: &ComponentFactory) -> Bean<Object> {
        unreachable!()
    }

    async fn destroy(&mut self) {}
}

pub struct SingletonScope {
    lazy: bool,
    instance: Option<Bean<Object>>,
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
    async fn peek(&self, _: &ComponentFactory) -> Option<Bean<Object>> {
        self.instance.clone()
    }

    async fn resolve(&mut self, factory: &ComponentFactory) -> Bean<Object> {
        if let Some(instance) = &self.instance {
            return instance.clone();
        }

        let instance = factory().await;
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
