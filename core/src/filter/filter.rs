use crate::error::AppError;
use crate::http::{Request, Response};
use async_trait::async_trait;

pub enum Order {
    Highest, // only for frameworks
    High,
    Normal,
    Low,
    Lowest, // only for frameworks
}
impl Order {
    fn weight(&self) -> u8 {
        match self {
            Order::Highest => 0,
            Order::High => 63,
            Order::Normal => 127,
            Order::Low => 191,
            Order::Lowest => 255,
        }
    }
}

#[async_trait]
pub trait Filter: Send + Sync {
    fn order(&self) -> Order {
        Order::Normal
    }
    async fn before(&self, req: &mut Request) -> Result<Option<Response>, AppError>;
    async fn after(
        &self,
        req: &mut Request,
        result: Result<Response, AppError>,
    ) -> Result<Response, AppError>;
}
