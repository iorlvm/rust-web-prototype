use crate::engine::Context;
use crate::error::KernelError;
use crate::http::{Request, Response};
use async_trait::async_trait;

pub mod request_chain;

#[async_trait]
pub trait Endpoint: Send + Sync {
    async fn execute(&self, ctx: &mut Context, req: &mut Request) -> Result<Response, KernelError>;
}
