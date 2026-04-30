use crate::engine::Kernel;
use crate::error::{
    DefaultFrameworkErrorHandler, ErrorDispatcher, ExternalErrorHandler, FrameworkErrorHandler,
};
use crate::handler::{Handler, HandlerRegistryBuilder};
use crate::middleware::req_body_extractors::{JsonExtractor, MultipartExtractor};
use crate::middleware::Middleware;
use crate::Endpoint;
use async_trait::async_trait;
use http::Method;

pub struct HandlerRegistration {
    pub method: Method,
    pub route: &'static str,
    pub endpoint: fn() -> Box<dyn Endpoint>,
    pub middleware: fn() -> Vec<Box<dyn Middleware>>,
}
inventory::collect!(HandlerRegistration);
pub fn registered_handlers() -> Vec<&'static HandlerRegistration> {
    inventory::iter::<HandlerRegistration>.into_iter().collect()
}

type MiddlewareChain = Vec<Box<dyn Middleware>>;
type FrameworkErrorHandlerBox = Box<dyn FrameworkErrorHandler>;
type ExternalErrorHandlerBox = Box<dyn ExternalErrorHandler>;

#[async_trait]
pub trait KernelFactory<T: Send + Sync + 'static>: Send + Sync + 'static {
    async fn build_injected(&self) -> T;

    fn handlers(&self) -> Vec<Handler> {
        vec![]
    }

    fn additional_middleware(&self) -> MiddlewareChain {
        vec![]
    }

    fn framework_error_handler(&self) -> Option<FrameworkErrorHandlerBox> {
        None
    }

    fn external_error_handlers(&self) -> Vec<ExternalErrorHandlerBox> {
        vec![]
    }
}

#[async_trait]
pub trait KernelCreator<T: Send + Sync + 'static> {
    async fn create(&self) -> Kernel<T>;
}

#[async_trait]
impl<T, F> KernelCreator<T> for F
where
    F: KernelFactory<T> + Send + Sync,
    T: Send + Sync + 'static,
{
    async fn create(&self) -> Kernel<T> {
        let injected = self.build_injected().await;

        let mut handlers = self.handlers();
        for registration in registered_handlers() {
            handlers.push(Handler::new(
                registration.method.clone(),
                registration.route.to_string(),
                (registration.endpoint)(),
                (registration.middleware)(),
            ));
        }
        let registry = HandlerRegistryBuilder::new(handlers).build();

        let mut middleware: MiddlewareChain = vec![
            Box::new(MultipartExtractor::default()),
            Box::new(JsonExtractor::default()),
        ];
        middleware.extend(self.additional_middleware());

        let error_dispatcher = ErrorDispatcher::new(
            self.framework_error_handler()
                .unwrap_or_else(|| Box::new(DefaultFrameworkErrorHandler::default())),
            self.external_error_handlers(),
        );

        Kernel::new(injected, registry, middleware, error_dispatcher)
    }
}
