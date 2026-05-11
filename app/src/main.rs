mod api;
pub mod error;
mod model;
pub mod repository;
mod security;

use crate::security::middleware::JwtAuthMiddleware;
use async_trait::async_trait;
use ioc_lite::{IoC, IoCBuilder};
use std::sync::Arc;
use tokio::net::TcpListener;
use web_kernel::engine::factory::{KernelFactory, MiddlewareChain};
use web_kernel::run;

#[derive(Default)]
pub struct TestKernelFactory {}

#[async_trait]
impl KernelFactory<IoC> for TestKernelFactory {
    async fn build_injected(&self) -> Arc<IoC> {
        let mut build = IoCBuilder::new();

        build.auto_register();

        build.build_with_test().await.into()
    }

    fn additional_middleware(&self) -> MiddlewareChain {
        vec![Box::new(JwtAuthMiddleware::default())]
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("server running on 127.0.0.1:8080");

    run(listener, TestKernelFactory::default()).await;
}
