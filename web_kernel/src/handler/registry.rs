use crate::handler::Handler;
use http::Method;
use std::collections::HashMap;

pub struct HandlerRegistry {
    method_map: HashMap<Method, Vec<Handler>>,
}

impl HandlerRegistry {
    pub fn find_handler(&self, method: &Method, path: &str) -> Option<&Handler> {
        self.method_map.get(method).and_then(|handlers| {
            handlers
                .iter()
                .find(|handler| handler.matches(method, path))
        })
    }
}

#[derive(Default)]
pub struct HandlerRegistryBuilder {
    handlers: Vec<Handler>,
}

impl HandlerRegistryBuilder {
    pub fn new(handlers: Vec<Handler>) -> Self {
        Self { handlers }
    }

    pub fn register(mut self, handler: Handler) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn build(self) -> HandlerRegistry {
        let mut map: HashMap<Method, Vec<Handler>> = HashMap::new();

        for handler in self.handlers {
            map.entry(handler.method().clone())
                .or_default()
                .push(handler);
        }

        HandlerRegistry { method_map: map }
    }
}
