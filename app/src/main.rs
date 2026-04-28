use async_trait::async_trait;
use ioc_lite::{Component, IoC};
use std::sync::Arc;
use tokio::net::TcpListener;
use web_kernel::engine::factory::KernelFactory;
use web_kernel::handler::Handler;
use web_kernel::run;

#[derive(Component)]
pub struct TestService {
    #[component]
    test: Arc<TestService2>,

    #[value = "test"]
    pub name: String,

    #[value = 123]
    num: i32,

    #[script(async |_| vec![1, 2, 3])]
    arr: Vec<i32>,
}
impl TestService {
    pub fn name(&self) -> String {
        self.test.name().to_string()
    }
}

#[derive(Component)]
pub struct TestService2;

impl TestService2 {
    pub fn name(&self) -> String {
        "test2".to_string()
    }
}

#[derive(Default)]
pub struct TestKernelFactory {}

#[async_trait]
impl KernelFactory<IoC> for TestKernelFactory {
    async fn build_injected(&self) -> IoC {
        let ioc = IoC::new().await;

        println!("{}", ioc.get::<TestService>().num);
        println!("{}", ioc.get::<TestService>().name);
        println!("{}", ioc.get::<TestService>().name());
        println!("{}", ioc.get::<TestService2>().name());
        println!("{:?}", ioc.get::<TestService>().arr);

        ioc
    }

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
