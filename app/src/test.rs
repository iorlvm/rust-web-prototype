use std::sync::Arc;
use ioc_lite::{Component, Proxy};

#[derive(Component)]
pub struct Test {
    #[component]
    test2: Proxy<Test2>,
}

#[derive(Component)]
pub struct Test2 {
    i: i32,
}