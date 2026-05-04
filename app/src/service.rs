use ioc_lite::{Bean, Component};

#[derive(Component)]
#[scope = "prototype"]
pub struct TestService {
    #[component]
    test: Bean<TestService2>,

    #[value = "test"]
    pub name: String,

    #[value = 123]
    pub num: i32,

    #[script(async |_| vec![1, 2, 3])]
    pub arr: Vec<i32>,
}
impl TestService {
    pub async fn name(&self) -> String {
        self.test.read().await.name().to_string()
    }
}

#[derive(Component)]
pub struct TestService2;

impl TestService2 {
    pub fn name(&self) -> String {
        "test2".to_string()
    }
}
