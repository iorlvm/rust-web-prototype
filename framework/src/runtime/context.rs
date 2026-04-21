use crate::infra::response::Response;
use http::Request;
use hyper::body::Incoming;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub struct Context {
    pub req: Request<Incoming>,
    pub res: Option<Response>,
    extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn from(req: Request<Incoming>) -> Self {
        Self {
            req,
            res: None,
            extensions: HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.extensions.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }
}
