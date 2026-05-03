use crate::service::TestService;
use async_trait::async_trait;
use ioc_lite::IoC;
use web_kernel::engine::Context;
use web_kernel::error::KernelError;
use web_kernel::http::{Request, Response};
use web_kernel::middleware::Middleware;

#[derive(Default)]
pub struct TestMiddlewareForShortcut;
#[async_trait]
impl Middleware for TestMiddlewareForShortcut {
    async fn before(
        &self,
        ctx: &mut Context,
        _: &mut Request,
    ) -> Result<Option<Response>, KernelError> {
        let ioc = ctx.get_injected::<IoC>();

        let service = ioc.create::<TestService>().await;
        println!("{}", service.num);

        println!("shortcut_before");
        Err(KernelError::BodyReadFailed(
            "failed at TestMiddlewareForShortcut".to_string(),
        ))
    }

    async fn after(
        &self,
        _: &mut Context,
        _: &Request,
        result: Result<Response, KernelError>,
    ) -> Result<Response, KernelError> {
        println!("shortcut_after");
        result
    }
}
