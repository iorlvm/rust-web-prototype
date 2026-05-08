pub use async_trait::async_trait;
pub use ioc_lite_macro::Component;

use dashmap::{DashMap, Entry};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const SINGLETON_SCOPE_ID: ScopeId = u64::MAX;

// types
pub type ScopeId = u64;
pub type Bean<T> = Arc<RwLock<Box<T>>>;

type Object = dyn Any + Send + Sync;
type ComponentForceWarmupFn =
    fn(ioc: IoC, scope_id: ScopeId) -> Pin<Box<dyn Future<Output = ()> + Send>>;
type Item = (
    String,
    ScopeType,
    ComponentForceWarmupFn,
    DashMap<ScopeId, Bean<Object>>,
);

#[derive(Clone)]
pub enum InitMode {
    Eager,
    Lazy,
}

#[derive(Clone)]
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

impl Clone for IoCBuilder {
    fn clone(&self) -> Self {
        Self {
            map: self
                .map
                .iter()
                .map(|(type_id, (name, scope_type, force_warmup, _))| {
                    (
                        *type_id,
                        (
                            name.clone(),
                            scope_type.clone(),
                            *force_warmup,
                            DashMap::new(),
                        ),
                    )
                })
                .collect::<HashMap<TypeId, Item>>(),
        }
    }
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

    pub fn register<T>(&mut self, scope_type: ScopeType, force_warmup: ComponentForceWarmupFn)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            (
                std::any::type_name::<T>().to_string(),
                scope_type,
                force_warmup,
                DashMap::new(),
            ),
        );
    }

    pub async fn build_with_test(self) -> IoC {
        let cloned = self.clone();
        let test_ioc = IoC {
            map: Arc::new(cloned.map),
            test_trace: Some(Arc::new(DashMap::new())),
        };
        test_ioc.run_test().await;

        self.build().await
    }

    pub async fn build(self) -> IoC {
        let ioc = IoC {
            map: Arc::new(self.map),
            test_trace: None,
        };
        ioc.on_build().await;
        ioc
    }
}

pub struct IoC {
    map: Arc<HashMap<TypeId, Item>>,
    test_trace: Option<Arc<DashMap<TypeId, ()>>>,
}

impl Clone for IoC {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            test_trace: self.test_trace.clone(),
        }
    }
}

impl IoC {
    pub async fn get<T>(&self, scope_id: ScopeId) -> Bean<T>
    where
        T: Component,
    {
        let type_id = TypeId::of::<T>();
        if let Some(test_trace) = &self.test_trace {
            if test_trace.contains_key(&type_id) {
                panic!("circular dependency detected");
            }
        }

        let (_, scope_type, _, bean_map) =
            self.map.get(&type_id).expect("component not registered");

        if let ScopeType::Prototype = scope_type {
            if let Some(test_trace) = &self.test_trace {
                test_trace.insert(type_id, ());
            }
            let value = T::create(self.clone(), scope_id).await;
            if let Some(test_trace) = &self.test_trace {
                test_trace.remove(&type_id);
            }
            return Arc::new(RwLock::new(Box::new(value)));
        }

        let scope_id = match scope_type {
            ScopeType::Singleton(_) => SINGLETON_SCOPE_ID,
            _ => scope_id,
        };

        let hit = { bean_map.get(&scope_id).map(|instance| instance.clone()) };

        let instance = match hit {
            Some(instance) => instance.clone(),
            None => {
                // 調整初始化順序, 降低堵塞時間 (副作用: create 會觸發多次)
                let created = {
                    if let Some(test_trace) = &self.test_trace {
                        test_trace.insert(type_id, ());
                    }
                    let value = T::create(self.clone(), scope_id.clone()).await;
                    if let Some(test_trace) = &self.test_trace {
                        test_trace.remove(&type_id);
                    }
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

    pub async fn warmup<T>(&self, scope_id: ScopeId)
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

    pub async fn force_warmup<T>(&self, scope_id: ScopeId)
    where
        T: Component,
    {
        self.get::<T>(scope_id).await;
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
    async fn run_test(&self) {
        println!("Running test script...");
        for (name, _, force_warmup, _) in self.map.values() {
            force_warmup(self.clone(), SINGLETON_SCOPE_ID).await;
            println!("- Component initialization checked: {}", name)
        }
        println!("Test script finished");
    }

    async fn on_build(&self) {
        for (_, scope_type, force_warmup, _) in self.map.values() {
            if let ScopeType::Singleton(mode) = scope_type {
                if let InitMode::Eager = mode {
                    force_warmup(self.clone(), SINGLETON_SCOPE_ID).await;
                }
            }
        }
    }
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
    async fn create(ioc: IoC, scope_id: ScopeId) -> Self;
}
