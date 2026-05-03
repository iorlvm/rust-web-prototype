use ioc_lite::Component;
use std::sync::Arc;

#[derive(Component)]
#[prototype]
pub struct TestService {
    #[component]
    test: Arc<TestService2>,

    #[value = "test"]
    pub name: String,

    #[value = 123]
    pub num: i32,

    #[script(async || vec![1, 2, 3])]
    pub arr: Vec<i32>,
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
