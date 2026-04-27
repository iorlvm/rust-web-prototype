pub mod req_body_extractors;

use crate::engine::Context;
use crate::error::KernelError;
use crate::http::{Request, Response};
use async_trait::async_trait;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before(
        &self,
        ctx: &mut Context,
        req: &mut Request,
    ) -> Result<Option<Response>, KernelError>;
    async fn after(
        &self,
        ctx: &mut Context,
        req: &Request,
        result: Result<Response, KernelError>,
    ) -> Result<Response, KernelError>;
}
