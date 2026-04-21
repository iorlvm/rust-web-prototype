mod runtime;
mod config;

pub mod contract;
pub mod types;
pub mod infra;
pub mod routing;

use crate::runtime::dispatcher::Dispatcher;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

pub fn run() {
    Runtime::new().unwrap().block_on(async {
        let config = config::load_config().await;
        let listener = TcpListener::bind(config.addr()).await.unwrap();
        println!("server running on {}", config.addr());


        let dispatcher = Arc::new(Dispatcher::build());
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let dispatcher = Arc::clone(&dispatcher);

            tokio::spawn(async move {
                let io = TokioIo::new(socket);

                let service = hyper::service::service_fn(move |req| {
                    let dispatcher = Arc::clone(&dispatcher);
                    async move { dispatcher.dispatch(req).await }
                });

                hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                    .unwrap();
            });
        }
    });
}
