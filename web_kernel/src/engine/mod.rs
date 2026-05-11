pub mod factory;

use crate::error::{ErrorDispatcher, KernelError};
use crate::handler::HandlerRegistry;
use crate::http::{HttpRequest, HttpResponse, Request};
use crate::middleware::Middleware;
use crate::runtime::request_chain::request_chain;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;
use xxhash_rust::xxh64::xxh64;

pub struct Kernel<T: Send + Sync + 'static> {
    injected: Arc<T>,
    registry: HandlerRegistry,
    middleware: Vec<Box<dyn Middleware>>,
    error_dispatcher: ErrorDispatcher,
}

impl<T: Send + Sync + 'static> Kernel<T> {
    pub fn new(
        injected: Arc<T>,
        registry: HandlerRegistry,
        middleware: Vec<Box<dyn Middleware>>,
        error_responder: ErrorDispatcher,
    ) -> Self {
        Self {
            injected,
            registry,
            middleware,
            error_dispatcher: error_responder,
        }
    }
    pub async fn handle(&self, req: HttpRequest) -> Result<HttpResponse, Infallible> {
        let handler = self.registry.find_handler(req.method(), req.uri().path());

        let result = match handler {
            Some(handler) => {
                let mut req = Request::from(req);
                let mut ctx = Context::default();
                ctx.insert(self.injected.clone());
                ctx.insert(handler.extract_path_variables(req.uri().path()));
                request_chain(&mut ctx, &mut req, handler, &self.middleware).await
            }
            None => Err(KernelError::NotFound(
                req.method().clone(),
                req.uri().clone(),
            )),
        };

        let resp = match result {
            Ok(resp) => resp,
            Err(err) => self.error_dispatcher.dispatch(err),
        };

        Ok(resp.into_http_response())
    }
}

pub struct Context {
    trace_id: Arc<String>,
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            trace_id: Arc::new(Uuid::now_v7().to_string()),
            map: HashMap::new(),
        }
    }
}

impl Context {
    pub fn trace_id(&self) -> Arc<String> {
        self.trace_id.clone()
    }

    pub fn trace_id_as_u64(&self) -> u64 {
        xxh64(self.trace_id.as_bytes(), 0)
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn remove<T: 'static + Send + Sync>(&mut self) {
        self.map.remove(&TypeId::of::<T>());
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_injected<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.get::<Arc<T>>()
            .cloned()
            .expect("injected resource missing: check the requested type T")
    }
}
