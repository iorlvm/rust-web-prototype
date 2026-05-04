mod middleware;
mod service;

use async_trait::async_trait;
use ioc_lite::{IoC, IoCBuilder};
use middleware::TestMiddlewareForShortcut;
use service::{TestService, TestService2};
use tokio::net::TcpListener;
use web_kernel::engine::factory::KernelFactory;
use web_kernel::engine::Context;
use web_kernel::error::KernelError;
use web_kernel::http::{Request, Response, ResponseBuilder};
use web_kernel::{handler, run};

#[handler(
    method = "GET",
    route = "/test",
    middleware(TestMiddlewareForShortcut::default())
)]
pub async fn test_handler(_: &mut Context, _: &mut Request) -> Result<Response, KernelError> {
    Ok(ResponseBuilder::new().text("OK".to_string()))
}

#[handler(method = "GET", route = "/test2")]
pub async fn test_handler2(_: &mut Context, _: &mut Request) -> Result<Response, KernelError> {
    Err(KernelError::BodyReadFailed("failed at Handler".to_string()))
}

#[derive(Default)]
pub struct TestKernelFactory {}

#[async_trait]
impl KernelFactory<IoC> for TestKernelFactory {
    async fn build_injected(&self) -> IoC {
        let ioc = IoCBuilder::new().build();

        let service = ioc.get::<TestService>().await;
        let service = service.read().await;

        println!("{}", service.num);
        println!("{}", service.name);
        println!("{}", service.name().await);
        println!("{}", ioc.get::<TestService2>().await.read().await.name());
        println!("{:?}", service.arr);

        ioc
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("server running on 127.0.0.1:8080");

    run(listener, TestKernelFactory::default()).await;
}
