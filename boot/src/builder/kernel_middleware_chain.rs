use core::middleware::extensions::MultipartResolver;
use core::middleware::Middleware;

pub struct KernelMiddlewareChainBuilder {
    kernel: Vec<Box<dyn Middleware>>,
    custom: Vec<Box<dyn Middleware>>,
}

impl KernelMiddlewareChainBuilder {
    pub fn new() -> Self {
        Self {
            kernel: vec![Box::new(MultipartResolver::new())],
            custom: vec![],
        }
    }

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
