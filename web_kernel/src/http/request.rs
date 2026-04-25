use http::HeaderValue;
use hyper::body::Incoming;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

pub type HttpRequest = http::Request<Incoming>;

pub struct Request {
    trace_id: String,
    body_consumed: bool,
    http_req: HttpRequest,
    context: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Request {
    pub fn from(http_req: HttpRequest) -> Self {
        Self {
            trace_id: uuid::Uuid::now_v7().to_string(),
            body_consumed: false,
            http_req,
            context: HashMap::new(),
        }
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    // context manipulation methods
    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.context.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.context
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_injected<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.get::<Arc<T>>()
            .cloned()
            .expect("injected resource missing: check the requested type T")
    }

    // http request methods
    pub fn method(&self) -> &http::Method {
        &self.http_req.method()
    }

    pub fn uri(&self) -> &http::uri::Uri {
        &self.http_req.uri()
    }

    pub fn header(&self, key: &str) -> Option<&HeaderValue> {
        self.http_req.headers().get(key)
    }

    pub fn headers(&self, key: &str) -> impl Iterator<Item = &HeaderValue> {
        self.http_req.headers().get_all(key).iter()
    }

    pub fn content_type(&self) -> Option<&str> {
        self.header("Content-Type").and_then(|v| v.to_str().ok())
    }

    pub fn take_body(&mut self) -> Option<&mut Incoming> {
        if self.body_consumed {
            return None;
        }

        self.body_consumed = true;
        Some(self.http_req.body_mut())
    }
}
