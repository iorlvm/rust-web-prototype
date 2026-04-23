use crate::filter::extensions::MultipartResolver;
use crate::filter::Filter;

pub struct FilterChainBuilder {
    kernel_filters: Vec<Box<dyn Filter>>,
    custom_filters: Vec<Box<dyn Filter>>,
}

impl FilterChainBuilder {
    pub fn new() -> Self {
        Self {
            kernel_filters: vec![Box::new(MultipartResolver::new())],
            custom_filters: vec![],
        }
    }

    pub fn add_filter(mut self, filter: impl Filter + 'static) -> Self {
        self.custom_filters.push(Box::new(filter));
        self
    }
    pub fn build(self) -> Vec<Box<dyn Filter>> {
        let mut chain = self.kernel_filters;
        chain.extend(self.custom_filters);

        chain
    }
}
