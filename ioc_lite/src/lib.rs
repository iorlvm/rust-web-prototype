pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;

use dashmap::{DashMap, Entry};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const SINGLETON_SCOPE_ID: ScopeId = u64::MAX;
pub const RUN_TEST_SCOPE_ID: ScopeId = u64::MAX - 1;

// types
pub type ScopeId = u64;
pub type Bean<T> = Arc<RwLock<Box<T>>>;

type Object = dyn Any + Send + Sync;
type ComponentInitTrigger =
    fn(ioc: IoC, scope_id: ScopeId) -> Pin<Box<dyn Future<Output = ()> + Send>>;
type Item = (
    String,
    ScopeType,
    ComponentInitTrigger,
    DashMap<ScopeId, Bean<Object>>,
);

pub enum InitMode {
    Eager,
    Lazy,
}

pub enum ScopeType {
    Singleton(InitMode),
    Prototype,
    Partitioned,
}

// 自動註冊機制
inventory::collect!(ComponentRegistration);
pub struct ComponentRegistration {
    pub register: fn(builder: &mut IoCBuilder) -> (),
}

// 將 register 與 runtime 分離, 降低資源管理複雜度
pub struct IoCBuilder {
    map: HashMap<TypeId, Item>,
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

    pub fn register<T>(&mut self, scope_type: ScopeType, init_trigger: ComponentInitTrigger)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            (
                std::any::type_name::<T>().to_string(),
                scope_type,
                init_trigger,
                DashMap::new(),
            ),
        );
    }

    pub async fn build(self) -> IoC {
        let ioc = IoC {
            map: Arc::new(self.map),
        };
        ioc.on_build().await;
        ioc
    }
}

pub struct IoC {
    map: Arc<HashMap<TypeId, Item>>,
}

impl Clone for IoC {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl IoC {
    pub async fn get<T>(&self, scope_id: ScopeId) -> Bean<T>
    where
        T: Component,
    {
        let (_, scope_type, _, bean_map) = self
            .map
            .get(&TypeId::of::<T>())
            .expect("component not registered");

        if let ScopeType::Prototype = scope_type {
            return Arc::new(RwLock::new(Box::new(
                T::create(self.clone(), scope_id).await,
            )));
        }

        let scope_id = match scope_type {
            ScopeType::Singleton(_) => SINGLETON_SCOPE_ID,
            _ => scope_id,
        };

        let hit = {
            bean_map
                .get(&scope_id)
                .map(|instance| instance.clone())
        };

        let instance = match hit {
            Some(instance) => instance.clone(),
            None => {
                // 調整初始化順序, 降低堵塞時間 (副作用: create 會觸發多次)
                let created = {
                    let value = T::create(self.clone(), scope_id.clone()).await;
                    let boxed = Box::new(value) as Box<Object>;
                    Arc::new(RwLock::new(boxed))
                };

                match bean_map.entry(scope_id) {
                    Entry::Occupied(o) => o.get().clone(),
                    Entry::Vacant(v) => v.insert(created).clone(),
                }
            }
        };

        let raw = Arc::into_raw(instance);
        let instance: Bean<T> = unsafe { Arc::from_raw(raw as *const RwLock<Box<T>>) };

        instance.clone()
    }

    pub async fn trigger<T>(&self, scope_id: ScopeId)
    where
        T: Component,
    {
        let item = self.map.get(&TypeId::of::<T>());
        if let Some((_, scope_type, _, _)) = item {
            if let ScopeType::Prototype = scope_type {
                return;
            }
            let _ = self.get::<T>(scope_id).await;
        }
    }

    pub async fn destroy<T>(&self, scope_id: ScopeId)
    where
        T: Component,
    {
        let item = self.map.get(&TypeId::of::<T>());
        if let Some((_, _, _, bean_map)) = item {
            bean_map.remove(&scope_id);
        }
    }

    // test script
    pub async fn run_test(&self) {
        println!("Running test script...");
        for (name, scope_type, init_trigger, _) in self.map.values() {
            init_trigger(self.clone(), RUN_TEST_SCOPE_ID).await;
            let scope = match scope_type {
                ScopeType::Singleton(_) => "singleton",
                ScopeType::Prototype => "prototype",
                ScopeType::Partitioned => "test",
            };
            println!("- Triggered in {} scope: {}", scope, name)
        }
        for (name, scope_type, _, bean_map) in self.map.values() {
            match scope_type {
                ScopeType::Singleton(_) => {
                    bean_map.remove(&RUN_TEST_SCOPE_ID);
                    println!("- Destroyed in singleton scope: {}", name);
                }
                ScopeType::Partitioned => {
                    bean_map.remove(&RUN_TEST_SCOPE_ID);
                    println!("- Destroyed in test scope: {}", name);
                }
                ScopeType::Prototype => {
                    println!("- Prototype scope not need destroy: {}", name);
                }
            }
        }
        println!("Test script finished");

        println!("Rebuilding IoC...");
        self.on_build().await;
    }

    // hooks
    async fn on_build(&self) {
        for (_, scope_type, init_trigger, _) in self.map.values() {
            if let ScopeType::Singleton(mode) = scope_type {
                if let InitMode::Eager = mode {
                    init_trigger(self.clone(), SINGLETON_SCOPE_ID).await;
                }
            }
        }
    }
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
    async fn create(ioc: IoC, scope_id: ScopeId) -> Self;
}
