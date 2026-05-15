use crate::internal::{IoCBuilderScript, TestTrace};
use crate::{
    BeanInstance, CacheInstance, Component, ComponentDefinition, InitMode, Lifecycle, Object, Proxy,
};
use dashmap::DashMap;
use std::any::TypeId;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

// Bean is Proxy
pub struct Bean<T: Component> {
    _mark: std::marker::PhantomData<T>,
    scope: Arc<Scope>,
    instance: OnceCell<BeanInstance>,
}

impl<T: Component> Bean<T> {
    fn new(scope: Arc<Scope>) -> Self {
        Self {
            _mark: std::marker::PhantomData,
            scope,
            instance: OnceCell::new(),
        }
    }

    pub async fn get_instance(&self) -> BeanInstance {
        let instance = self
            .instance
            .get_or_init(|| self.scope.get_raw_instance_with_cache::<T>())
            .await;
        instance.clone()
    }
    pub fn downcast_ref<'a>(&self, obj: &'a Object) -> & 'a T {
        obj.downcast_ref::<T>().expect(&format!(
            "Bean type mismatch: expected {}",
            std::any::type_name::<T>()
        ))
    }

    pub fn downcast_mut<'a>(&self, obj: &'a mut Object) -> & 'a mut T {
        obj.downcast_mut::<T>().expect(&format!(
            "Bean type mismatch: expected {}",
            std::any::type_name::<T>()
        ))
    }
}

pub struct Scope {
    id: u64,
    name: String,
    isolated_map: HashMap<TypeId, CacheInstance>,
    parent: Option<Arc<Scope>>,

    // shared
    script_input: Arc<serde_json::Value>,
    registry: Arc<HashMap<TypeId, ComponentDefinition>>,
    script_cache: Arc<DashMap<TypeId, Box<Object>>>,
    counter: Arc<AtomicU64>,
    test_trace: Arc<Option<DashMap<TypeId, ()>>>,
}

impl Scope {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get<T: Component>(self: &Arc<Self>) -> Proxy<T> {
        Proxy::new(T::proxy(Bean::new(self.clone())))
    }

    pub fn create_sub_scope(self: &Arc<Self>, naming: String) -> Arc<Scope> {
        let mut isolated_map = HashMap::new();
        for (type_id, def) in self.registry.iter() {
            if let Lifecycle::Scoped(ref name_format) = def.lifecycle {
                if name_format.is_match(&naming) {
                    isolated_map.insert(*type_id, RwLock::new(OnceCell::new()));
                }
            }
        }

        let parent = self.clone();
        Arc::new(Scope {
            script_input: parent.script_input.clone(),
            registry: parent.registry.clone(),
            script_cache: parent.script_cache.clone(),
            counter: parent.counter.clone(),
            test_trace: parent.test_trace.clone(),

            id: parent
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            name: naming,
            isolated_map,
            parent: Some(parent),
        })
    }

    pub async fn warmup<T: Component>(self: &Arc<Self>) {
        if self.isolated_map.contains_key(&TypeId::of::<T>()) {
            self.get_raw_instance_with_cache::<T>().await;
        }
    }

    pub async fn force_warmup<T: Component>(self: &Arc<Self>) {
        self.get_raw_instance_with_cache::<T>().await;
    }

    pub fn bump<T: Component>(self: &Arc<Self>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let parent_fut = self
            .parent
            .as_ref()
            .map(|p| p.bump::<T>())
            .unwrap_or_else(|| Box::pin(async move {}));

        let self_clone = self.clone();
        Box::pin(async move {
            match self_clone.isolated_map.get(&TypeId::of::<T>()) {
                Some(cell) => {
                    let mut locked = cell.write().await;
                    parent_fut.await;
                    locked.take();
                }
                None => {
                    parent_fut.await;
                }
            }
        })
    }

    pub async fn run_script<T, S, Fut>(self: &Arc<Self>, script: S) -> T
    where
        S: Fn(Arc<serde_json::Value>) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send,
    {
        let result = script(self.script_input.clone()).await;
        if self.in_test() {
            println!(
                " - [SCRIPT] async (input: Arc<Json>) -> {}",
                std::any::type_name::<T>().to_string()
            );
        }
        result
    }

    pub async fn run_script_with_cache<T, S, Fut>(self: &Arc<Self>, script: S) -> T
    where
        S: Fn(Arc<serde_json::Value>) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send,
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

    // internal methods
    fn get_raw_instance<T: Component>(
        self: &Arc<Self>,
    ) -> Pin<Box<dyn Future<Output = BeanInstance> + Send>> {
        let parent_opt = self.parent.clone();
        let self_clone = self.clone();

        Box::pin(async move {
            match parent_opt {
                Some(p) => p.get_raw_instance_with_cache::<T>().await,
                None => {
                    let type_id = TypeId::of::<T>();
                    self_clone.check_circular_dependency(&type_id);

                    self_clone.enter_trace(&type_id);

                    if self_clone.in_test() {
                        println!("[BUILD START] {}", std::any::type_name::<T>());
                    }
                    let value = T::create(self_clone.clone()).await;
                    if self_clone.in_test() {
                        println!("[BUILD END] {}", std::any::type_name::<T>());
                    }
                    self_clone.exit_trace(&type_id);

                    let boxed = Box::new(value) as Box<Object>;
                    Arc::new(RwLock::new(boxed))
                }
            }
        })
    }

    async fn get_raw_instance_with_cache<T: Component>(self: &Arc<Self>) -> BeanInstance {
        match self.isolated_map.get(&TypeId::of::<T>()) {
            None => self.get_raw_instance::<T>().await,
            Some(cell) => cell
                .read()
                .await
                .get_or_init(|| self.get_raw_instance::<T>())
                .await
                .clone(),
        }
    }
}

pub struct IoC {
    scope: Arc<Scope>,
}

impl IoC {
    pub fn new(
        script_input: serde_json::Value,
        registry: HashMap<TypeId, ComponentDefinition>,
        test_trace: Option<DashMap<TypeId, ()>>,
    ) -> Self {
        let mut isolated_map = HashMap::new();
        registry
            .iter()
            .for_each(|(type_id, def)| match def.lifecycle {
                Lifecycle::Singleton(_) => {
                    isolated_map.insert(*type_id, RwLock::new(OnceCell::new()));
                }
                _ => (),
            });

        let counter = AtomicU64::new(0);
        let root = Arc::new(Scope {
            id: counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            name: "ROOT".to_string(),
            parent: None,
            isolated_map,

            script_input: Arc::new(script_input),
            registry: Arc::new(registry),
            script_cache: Arc::new(DashMap::new()),
            counter: Arc::new(counter),
            test_trace: Arc::new(test_trace),
        });

        IoC { scope: root }
    }

    pub fn id(&self) -> u64 {
        self.scope.id()
    }

    pub fn get<T: Component>(&self) -> Proxy<T> {
        Proxy::new(T::proxy(Bean::new(self.scope.clone())))
    }

    pub fn create_sub_scope(&self, naming: String) -> Arc<Scope> {
        self.scope.create_sub_scope(naming)
    }

    pub async fn warmup<T: Component>(self: &Arc<Self>) {
        self.scope.warmup::<T>().await;
    }

    pub async fn force_warmup<T: Component>(&self) {
        self.scope.force_warmup::<T>().await;
    }

    pub async fn bump<T: Component>(&self) {
        self.scope.bump::<T>().await;
    }
}

// internal methods
impl TestTrace for Scope {
    fn test_trace(&self) -> &Option<DashMap<TypeId, ()>> {
        &self.test_trace
    }
}

impl IoCBuilderScript for IoC {
    // warmup singleton components in eager mode
    async fn on_build(&self) {
        for def in self.scope.registry.values() {
            if let Lifecycle::Singleton(ref mode) = def.lifecycle {
                if let InitMode::Eager = mode {
                    (def.warmup_fn)(self.scope.clone()).await;
                }
            }
        }
    }

    // force warmup all components
    async fn run_test(&self) {
        println!("╭──────────────────────────────────────╮");
        println!("│           TEST SCRIPT START          │");
        println!("╰──────────────────────────────────────╯");
        for def in self.scope.registry.values() {
            println!("[START] {}", def.name);
            (def.warmup_fn)(self.scope.clone()).await;
            println!("[CHECKED] {}", def.name);
        }
        println!("╭──────────────────────────────────────╮");
        println!("│           TEST SCRIPT END            │");
        println!("╰──────────────────────────────────────╯");
    }
}
