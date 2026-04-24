use core::middleware::req_body_extractors::*;
use core::middleware::Middleware;

pub struct KernelMiddlewareChainBuilder {
    kernel: Vec<Box<dyn Middleware>>,
    custom: Vec<Box<dyn Middleware>>,
}

impl Default for KernelMiddlewareChainBuilder {
    fn default() -> Self {
        Self {
            kernel: vec![
                Box::new(MultipartExtractor::default()),
                Box::new(JsonExtractor::default()),
            ],
            custom: vec![],
        }
    }
}

impl KernelMiddlewareChainBuilder {
    pub fn add(mut self, middleware: impl Middleware + 'static) -> Self {
        self.custom.push(Box::new(middleware));
        self
    }
    pub fn build(self) -> Vec<Box<dyn Middleware>> {
        let mut chain = self.kernel;
        chain.extend(self.custom);

        chain
    }
}
