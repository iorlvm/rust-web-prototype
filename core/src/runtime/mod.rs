use crate::error::AppError;
use crate::http::{Request, Response};
use async_trait::async_trait;

pub mod request_chain;

#[async_trait]
pub trait Endpoint: Send + Sync {
    async fn execute(&self, req: &mut Request) -> Result<Response, AppError>;
}
