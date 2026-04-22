use crate::filter::extensions::MultipartResolver;
use crate::filter::Filter;

pub struct FilterChainBuilder {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterChainBuilder {
    pub fn new() -> Self {
        Self { filters: vec![] }
    }

    pub fn with_default(mut self) -> Self {
        self.with_filter(MultipartResolver::new())
    }

    pub fn with_filter(mut self, filter: impl Filter + 'static) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    pub fn build(mut self) -> Vec<Box<dyn Filter>> {
        self.filters.sort_by_key(|f| f.order().weight());
        self.filters
    }
}
