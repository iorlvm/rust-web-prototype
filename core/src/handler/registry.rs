use crate::handler::Handler;
use http::Method;
use std::collections::HashMap;

pub struct HandlerRegistry {
    method_map: HashMap<Method, Vec<Box<dyn Handler>>>,
}

impl HandlerRegistry {
    pub fn get_handlers(&self, method: &Method) -> Option<&Vec<Box<dyn Handler>>> {
        self.method_map.get(method)
    }
}

pub struct HandlerRegistryBuilder {
    handlers: Vec<Box<dyn Handler>>,
}

impl HandlerRegistryBuilder {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    pub fn register(mut self, handler: Box<dyn Handler>) -> Self {
        self.handlers.push(handler);
        self
    }
    pub fn build(self) -> HandlerRegistry {
        let mut map: HashMap<Method, Vec<Box<dyn Handler>>> = HashMap::new();

        for handler in self.handlers {
            map.entry(handler.method().clone())
                .or_insert_with(Vec::new)
                .push(handler);
        }

        HandlerRegistry { method_map: map }
    }
}
