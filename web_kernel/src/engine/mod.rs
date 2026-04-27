pub mod factory;

use crate::error::{ErrorDispatcher, KernelError};
use crate::handler::{Handler, HandlerRegistry};
use crate::http::{HttpRequest, HttpResponse, Request};
use crate::middleware::Middleware;
use crate::runtime::request_chain::request_chain;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

pub struct Kernel<T: Send + Sync + 'static> {
    injected: Arc<T>,
    registry: HandlerRegistry,
    middleware: Vec<Box<dyn Middleware>>,
    error_dispatcher: ErrorDispatcher,
}

impl<T: Send + Sync + 'static> Kernel<T> {
    pub fn new(
        injected: T,
        registry: HandlerRegistry,
        middleware: Vec<Box<dyn Middleware>>,
        error_responder: ErrorDispatcher,
    ) -> Self {
        Self {
            injected: Arc::new(injected),
            registry,
            middleware,
            error_dispatcher: error_responder,
        }
    }
    pub async fn handle(&self, req: HttpRequest) -> Result<HttpResponse, Infallible> {
        let handler = self.find_handler(req.method(), req.uri().path());

        let result = match handler {
            Some(handler) => {
                let mut req = Request::from(req);

                let mut ctx = Context::default();
                ctx.insert(self.injected.clone());

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

    fn find_handler(&self, method: &http::Method, path: &str) -> Option<&Handler> {
        self.registry.get_handlers(method).and_then(|handlers| {
            handlers
                .iter()
                .find(|handler| handler.matches(method, path))
        })
    }
}

#[derive(Default)]
pub struct Context {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
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
