use core::engine::Kernel;
use core::error::ErrorResponder;
use core::filter::FilterChainBuilder;
use core::handler::HandlerRegistryBuilder;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

pub mod config;

pub fn run() {
    Runtime::new().unwrap().block_on(async {
        let config = config::load_config().await;
        let listener = TcpListener::bind(config.addr()).await.unwrap();
        println!("server running on {}", config.addr());

        let kernel = Arc::new(Kernel::new(
            HandlerRegistryBuilder::new().build(),
            FilterChainBuilder::new().with_default().build(),
            ErrorResponder::new(),
        ));
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let kernel = Arc::clone(&kernel);

            tokio::spawn(async move {
                let io = TokioIo::new(socket);

                let service = hyper::service::service_fn(move |req| {
                    let kernel = Arc::clone(&kernel);
                    async move { kernel.handle(req).await }
                });

                hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                    .unwrap();
            });
        }
    });
}
