use crate::internal::{IoCBuilderScript, TestTrace};
use crate::{BeanInstance, Component, InitMode, Item, Object, ScopeType};
use dashmap::DashMap;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

// Bean is Proxy
pub struct Bean<T: Component> {
    ioc: IoC,
    instance: OnceCell<BeanInstance<T>>,
}

impl<T: Component> Bean<T> {
    fn new(ioc: IoC) -> Self {
        Self {
            ioc,
            instance: OnceCell::new(),
        }
    }

    pub async fn get_instance(&self) -> BeanInstance<T> {
        let instance = self
            .instance
            .get_or_init(|| self.ioc.get_instance::<T>())
            .await;
        instance.clone()
    }
}

// IoC is Container and Factory
#[derive(Clone)]
pub struct IoC {
    map: Arc<HashMap<TypeId, Item>>,
    script_cache: Arc<DashMap<TypeId, Box<Object>>>,
    test_trace: Option<Arc<DashMap<TypeId, ()>>>,
}

impl IoC {
    pub fn new(map: HashMap<TypeId, Item>) -> Self {
        Self {
            map: Arc::new(map),
            script_cache: Arc::new(DashMap::new()),
            test_trace: None,
        }
    }

    async fn get_instance<T: Component>(&self) -> BeanInstance<T> {
        let type_id = TypeId::of::<T>();
        self.check_circular_dependency(&type_id);

        let (_, scope_type, _, cache) = self.map.get(&type_id).expect("component not registered");

        // Prototype: every time create a new instance
        if let ScopeType::Prototype = scope_type {
            self.enter_trace(type_id);
            let value = T::create(self.clone()).await;
            self.exit_trace(&type_id);
            return Arc::new(RwLock::new(Box::new(value)));
        }

        let hit_cache = { cache.read().await.as_ref().map(|instance| instance.clone()) };
        let instance = match hit_cache {
            Some(instance) => instance,
            None => {
                let mut cache = cache.write().await;
                let double_check = cache.as_ref().map(|instance| instance.clone());

                let created = match double_check {
                    Some(instance) => instance,
                    None => {
                        self.enter_trace(type_id);
                        let value = T::create(self.clone()).await;
                        self.exit_trace(&type_id);
                        let boxed = Box::new(value) as Box<Object>;
                        Arc::new(RwLock::new(boxed))
                    }
                };

                let _ = cache.insert(created.clone());
                created
            }
        };

        let raw = Arc::into_raw(instance);
        let instance: BeanInstance<T> = unsafe { Arc::from_raw(raw as *const RwLock<Box<T>>) };

        instance.clone()
    }

    pub fn get<T: Component>(&self) -> T::Output {
        T::proxy(Bean::new(self.clone()))
    }

    pub async fn warmup<T: Component>(&self) {
        let item = self.map.get(&TypeId::of::<T>());
        if let Some((_, scope_type, _, _)) = item {
            if let ScopeType::Prototype = scope_type {
                return;
            }
            let _ = self.get_instance::<T>().await;
        }
    }

    pub async fn force_warmup<T: Component>(&self) {
        self.get_instance::<T>().await;
    }

    pub async fn destroy<T: Component>(&self) {
        let item = self.map.get(&TypeId::of::<T>());
        if let Some((_, _, _, cache)) = item {
            cache.write().await.take();
        }
    }

    pub async fn run_script<T, S, Fut>(&self, script: S) -> T
    where
        S: Fn() -> Fut + 'static,
        Fut: Future<Output = T>,
        T: Send + Sync + 'static,
    {
        let result = script().await;
        if self.in_test() {
            println!(
                "  - Script checked: async (IoC) -> {}",
                std::any::type_name::<T>().to_string()
            );
        }

        result
    }

    pub async fn run_script_with_cache<T, S, Fut>(&self, script: S) -> T
    where
        S: Fn() -> Fut + 'static,
        Fut: Future<Output = T>,
        T: Clone + Send + Sync + 'static,
    {
        let script_id = TypeId::of::<S>();
        let hit = self.script_cache.get(&script_id).map(|raw| {
            raw.downcast_ref::<T>()
                .expect("script type mismatch")
                .clone()
        });
        if let Some(hit) = hit {
            return hit;
        }

        let result = self.run_script(script).await;
        self.script_cache
            .insert(script_id, Box::new(result.clone()));

        result
    }
}

// internal methods
impl TestTrace for IoC {
    fn test_trace(&self) -> &Option<Arc<DashMap<TypeId, ()>>> {
        &self.test_trace
    }
}

impl IoCBuilderScript for IoC {
    fn into_test_mode(self) -> Self {
        IoC {
            test_trace: Some(Arc::new(DashMap::new())),
            ..self
        }
    }
    
    // warmup singleton components in eager mode
    async fn on_build(&self) {
        for (_, scope_type, force_warmup, _) in self.map.values() {
            if let ScopeType::Singleton(mode) = scope_type {
                if let InitMode::Eager = mode {
                    force_warmup(self.clone()).await;
                }
            }
        }
    }

    // force warmup all components
    async fn run_test(&self) {
        println!("Running test script...");
        for (name, _, force_warmup, _) in self.map.values() {
            force_warmup(self.clone()).await;
            println!("- Component initialization checked: {}", name)
        }
        println!("Test script finished");
    }
}
