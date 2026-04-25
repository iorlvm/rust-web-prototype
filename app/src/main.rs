use async_trait::async_trait;
use tokio::net::TcpListener;
use web_kernel::engine::factory::KernelFactory;
use web_kernel::handler::Handler;
use web_kernel::run;

#[derive(Default)]
pub struct TestKernelFactory {}

#[async_trait]
impl KernelFactory<()> for TestKernelFactory {
    async fn build_injected(&self) -> () {}

    fn handlers(&self) -> Vec<Handler> {
        vec![]
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("server running on 127.0.0.1:8080");

    run(listener, TestKernelFactory::default()).await;
}
