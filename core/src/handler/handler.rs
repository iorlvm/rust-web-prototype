use crate::error::AppError;
use crate::http::{Request, Response};
use crate::middleware::Middleware;
use crate::runtime::request_chain::request_chain;
use crate::runtime::Endpoint;
use async_trait::async_trait;
use http::Method;

pub struct Handler {
    method: Method,
    path_pattern: String,
    endpoint: Box<dyn Endpoint>,
    middleware: Vec<Box<dyn Middleware>>,
}

impl Handler {
    pub fn new(
        method: Method,
        path_pattern: String,
        endpoint: Box<dyn Endpoint>,
        middleware: Vec<Box<dyn Middleware>>,
    ) -> Self {
        Self {
            method,
            path_pattern,
            endpoint,
            middleware,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path_pattern(&self) -> &str {
        &self.path_pattern
    }

    pub fn matches(&self, method: &Method, path: &str) -> bool {
        // TODO: Implement path pattern matching logic
        self.method == method && self.path_pattern == path
    }
}

#[async_trait]
impl Endpoint for Handler {
    async fn execute(&self, req: &mut Request) -> Result<Response, AppError> {
        request_chain(req, self.endpoint.as_ref(), &self.middleware).await
    }
}
