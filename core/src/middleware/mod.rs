pub mod req_body_extractors;

use crate::error::KernelError;
use crate::http::{Request, Response};
use async_trait::async_trait;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before(&self, req: &mut Request) -> Result<Option<Response>, KernelError>;
    async fn after(
        &self,
        req: &mut Request,
        result: Result<Response, KernelError>,
    ) -> Result<Response, KernelError>;
}
