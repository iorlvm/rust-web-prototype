mod runtime;

pub mod engine;
pub mod error;
pub mod handler;
pub mod http;
pub mod middleware;
pub mod types;

use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn run<T: Send + Sync + 'static>(
    listener: TcpListener,
    kernel_creator: impl engine::factory::KernelCreator<T>,
) {
    let kernel = Arc::new(kernel_creator.create().await);
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
}
