pub use async_trait::async_trait;
pub use ioc_lite_macro::{proxy_method, Component};

use dashmap::{DashMap, Entry};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const SINGLETON_SCOPE_ID: ScopeId = u64::MAX;

// types
pub type ScopeId = u64;

type Object = dyn Any + Send + Sync;
type ComponentForceWarmupFn =
    fn(ioc: IoC, scope_id: ScopeId) -> Pin<Box<dyn Future<Output = ()> + Send>>;
type Item = (
    String,
    ScopeType,
    ComponentForceWarmupFn,
    DashMap<ScopeId, BeanInstance<Object>>,
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

pub type BeanInstance<T> = Arc<RwLock<Box<T>>>;
pub struct Bean<T: Component> {
    _marker: PhantomData<T>,
    ioc: IoC,
    look_at: u64,
}

impl<T: Component> Bean<T> {
    fn new(ioc: IoC, look_at: u64) -> Self {
        Self {
            ioc,
            look_at,
            _marker: PhantomData,
        }
    }

    pub async fn get_instance(&self) -> BeanInstance<T> {
        self.ioc.get_instance::<T>(self.look_at).await
    }
}

#[derive(Clone)]
pub struct IoC {
    map: Arc<HashMap<TypeId, Item>>,
    script_cache: Arc<DashMap<TypeId, Box<Object>>>,
    test_trace: Option<Arc<DashMap<TypeId, ()>>>,
}

impl IoC {
    async fn get_instance<T: Component>(&self, scope_id: ScopeId) -> BeanInstance<T> {
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

        let look_at = match scope_type {
            ScopeType::Singleton(_) => SINGLETON_SCOPE_ID,
            _ => scope_id,
        };

        let hit = { bean_map.get(&look_at).map(|instance| instance.clone()) };
        let instance = match hit {
            Some(instance) => instance.clone(),
            None => {
                let created = {
                    if let Some(test_trace) = &self.test_trace {
                        test_trace.insert(type_id, ());
                    }
                    let value = T::create(self.clone(), scope_id).await;
                    if let Some(test_trace) = &self.test_trace {
                        test_trace.remove(&type_id);
                    }
                    let boxed = Box::new(value) as Box<Object>;
                    Arc::new(RwLock::new(boxed))
                };

                match bean_map.entry(look_at) {
                    Entry::Occupied(o) => o.get().clone(),
                    Entry::Vacant(v) => v.insert(created).clone(),
                }
            }
        };

        let raw = Arc::into_raw(instance);
        let instance: BeanInstance<T> = unsafe { Arc::from_raw(raw as *const RwLock<Box<T>>) };

        instance.clone()
    }

    pub fn get<T: Component>(&self, scope_id: ScopeId) -> T::Output {
        T::proxy(Bean::new(self.clone(), scope_id))
    }

    pub async fn warmup<T: Component>(&self, scope_id: ScopeId) {
        let item = self.map.get(&TypeId::of::<T>());
        if let Some((_, scope_type, _, _)) = item {
            if let ScopeType::Prototype = scope_type {
                return;
            }
            let _ = self.get_instance::<T>(scope_id).await;
        }
    }

    pub async fn force_warmup<T: Component>(&self, scope_id: ScopeId) {
        self.get_instance::<T>(scope_id).await;
    }

    pub async fn destroy<T: Component>(&self, scope_id: ScopeId) {
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
pub trait Component: Sized + Send + Sync + 'static {
    type Output;
    fn proxy(input: Bean<Self>) -> Self::Output;
    async fn create(ioc: IoC, scope_id: ScopeId) -> Self;
}

// macro script utility function
pub async fn run_script<T, S, Fut>(ioc: &IoC, script: S) -> T
where
    S: Fn() -> Fut + 'static,
    Fut: Future<Output = T>,
    T: Send + Sync + 'static,
{
    let result = script().await;
    if ioc.test_trace.is_some() {
        println!(
            "  - Script checked: async (IoC) -> {}",
            std::any::type_name::<T>().to_string()
        );
    }

    result
}

pub async fn run_script_with_cache<T, S, Fut>(ioc: &IoC, script: S) -> T
where
    S: Fn() -> Fut + 'static,
    Fut: Future<Output = T>,
    T: Clone + Send + Sync + 'static,
{
    let script_id = TypeId::of::<S>();
    let hit = ioc.script_cache.get(&script_id).map(|raw| {
        raw.downcast_ref::<T>()
            .expect("script type mismatch")
            .clone()
    });
    if let Some(hit) = hit {
        return hit;
    }

    let result = run_script(ioc, script).await;
    ioc.script_cache.insert(script_id, Box::new(result.clone()));

    result
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

    pub fn register<T>(&mut self, scope_type: ScopeType)
    where
        T: Component,
    {
        self.map.insert(
            TypeId::of::<T>(),
            (
                std::any::type_name::<T>().to_string(),
                scope_type,
                |ioc, scope_id| {
                    Box::pin(async move {
                        ioc.force_warmup::<T>(scope_id).await;
                    })
                },
                DashMap::new(),
            ),
        );
    }

    pub async fn build_with_test(self) -> IoC {
        let cloned = self.clone();
        let test_ioc = IoC {
            map: Arc::new(cloned.map),
            script_cache: Arc::new(DashMap::new()),
            test_trace: Some(Arc::new(DashMap::new())),
        };
        test_ioc.run_test().await;

        self.build().await
    }

    pub async fn build(self) -> IoC {
        let ioc = IoC {
            map: Arc::new(self.map),
            script_cache: Arc::new(DashMap::new()),
            test_trace: None,
        };
        ioc.on_build().await;
        ioc
    }
}
