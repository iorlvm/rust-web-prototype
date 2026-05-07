use crate::engine::Context;
use crate::error::KernelError;
use crate::http::{Request, Response};
use crate::middleware::Middleware;
use crate::runtime::request_chain::request_chain;
use crate::runtime::Endpoint;
use async_trait::async_trait;
use http::Method;
use std::collections::HashMap;

#[derive(Default)]
pub struct PathVariables {
    map: HashMap<String, String>,
}

impl PathVariables {
    pub fn insert(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }

    pub fn map(&self) -> &HashMap<String, String> {
        &self.map
    }
}

pub struct Handler {
    method: Method,
    route: String,
    endpoint: Box<dyn Endpoint>,
    middleware: Vec<Box<dyn Middleware>>,
}

impl Handler {
    pub fn new(
        method: Method,
        route: String,
        endpoint: Box<dyn Endpoint>,
        middleware: Vec<Box<dyn Middleware>>,
    ) -> Self {
        Self {
            method,
            route,
            endpoint,
            middleware,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn route(&self) -> &str {
        &self.route
    }

    pub fn matches(&self, method: &Method, path: &str) -> bool {
        if &self.method != method {
            return false;
        }

        let pattern_parts = self.route.trim_matches('/').split('/');
        let path_parts = path.trim_matches('/').split('/');

        for (p, v) in pattern_parts.zip(path_parts) {
            if p.starts_with('{') && p.ends_with('}') {
                continue;
            }

            if p != v {
                return false;
            }
        }

        // 長度一致檢查
        self.route.trim_matches('/').split('/').count() == path.trim_matches('/').split('/').count()
    }

    pub fn extract_path_variables(&self, path: &str) -> PathVariables {
        let mut path_variables = PathVariables::default();

        let pattern_parts = self.route.trim_matches('/').split('/');
        let path_parts = path.trim_matches('/').split('/');

        for (p, v) in pattern_parts.zip(path_parts) {
            if p.starts_with('{') && p.ends_with('}') {
                let key = &p[1..p.len() - 1];
                path_variables.insert(key.to_string(), v.to_string());
            }
        }

        path_variables
    }
}

#[async_trait]
impl Endpoint for Handler {
    async fn execute(&self, ctx: &mut Context, req: &mut Request) -> Result<Response, KernelError> {
        request_chain(ctx, req, self.endpoint.as_ref(), &self.middleware).await
    }
}
