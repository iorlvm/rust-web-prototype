use crate::error::ErrorResponder;
use crate::handler::{Handler, HandlerRegistry};
use crate::http::{HttpRequest, HttpResponse, Request, Response};
use crate::middleware::Middleware;
use crate::runtime::request_chain::request_chain;
use http::StatusCode;
use std::convert::Infallible;
use std::sync::Arc;

pub struct Kernel<T: Send + Sync + 'static> {
    injected: Arc<T>,
    registry: HandlerRegistry,
    middleware: Vec<Box<dyn Middleware>>,
    error_responder: ErrorResponder,
}

impl<T: Send + Sync + 'static> Kernel<T> {
    pub fn new(
        injected: T,
        registry: HandlerRegistry,
        middleware: Vec<Box<dyn Middleware>>,
        error_responder: ErrorResponder,
    ) -> Self {
        Self {
            injected: Arc::new(injected),
            registry,
            middleware,
            error_responder,
        }
    }
    pub async fn handle(&self, req: HttpRequest) -> Result<HttpResponse, Infallible> {
        let handler = self.find_handler(req.method(), req.uri().path());

        let resp = match handler {
            Some(handler) => {
                let mut req = Request::from(req);
                req.insert(self.injected.clone());

                request_chain(&mut req, handler, &self.middleware)
                    .await
                    .unwrap_or_else(|err| self.error_responder.handle(err))
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .text("Not Found".to_string()),
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
