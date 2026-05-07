use crate::{Bean, ComponentFactory, Object, ScopeId};
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

lazy_static! {
    pub static ref IOC_SCOPE_KEY: String = Uuid::new_v4().to_string();
}

pub enum Action {
    Trigger,
    Destroy,
    None,
}

#[async_trait]
pub trait LifecycleScope: Send + Sync + 'static {
    async fn peek(&self, scope_id: ScopeId, factory: &ComponentFactory) -> Option<Bean<Object>>;
    async fn resolve(&mut self, scope_id: ScopeId, factory: &ComponentFactory) -> Bean<Object>;
    async fn destroy(&mut self, scope_id: ScopeId);

    // lifecycle hook
    fn on_build(&self) -> Action {
        Action::None
    }
}

#[derive(Default)]
pub struct PrototypeScope;
#[async_trait]
impl LifecycleScope for PrototypeScope {
    async fn peek(&self, scope_id: ScopeId, factory: &ComponentFactory) -> Option<Bean<Object>> {
        Some(factory(scope_id).await)
    }

    async fn resolve(&mut self, _: ScopeId, _: &ComponentFactory) -> Bean<Object> {
        unreachable!()
    }

    async fn destroy(&mut self, _: ScopeId) {}
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
    async fn peek(&self, _: ScopeId, _: &ComponentFactory) -> Option<Bean<Object>> {
        self.instance.clone()
    }

    async fn resolve(&mut self, _: ScopeId, factory: &ComponentFactory) -> Bean<Object> {
        if let Some(instance) = &self.instance {
            return instance.clone();
        }

        let instance = factory(Arc::new(IOC_SCOPE_KEY.to_string())).await;
        self.instance = Some(instance.clone());

        instance
    }

    async fn destroy(&mut self, _: ScopeId) {
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

#[derive(Default)]
pub struct PartitionedScope {
    map: HashMap<String, Bean<Object>>,
}

#[async_trait]
impl LifecycleScope for PartitionedScope {
    async fn peek(&self, scope_id: ScopeId, _: &ComponentFactory) -> Option<Bean<Object>> {
        match self.map.get(scope_id.as_ref()) {
            Some(instance) => Some(instance.clone()),
            None => None,
        }
    }

    async fn resolve(&mut self, scope_id: ScopeId, factory: &ComponentFactory) -> Bean<Object> {
        if let Some(instance) = self.map.get(scope_id.as_ref()) {
            return instance.clone();
        }

        let instance = factory(scope_id.clone()).await;
        self.map.insert(scope_id.to_string(), instance.clone());

        instance
    }

    async fn destroy(&mut self, scope_id: ScopeId) {
        self.map.remove(scope_id.as_ref());
    }
}
