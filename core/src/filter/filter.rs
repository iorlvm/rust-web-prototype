use crate::error::AppError;
use crate::http::{Request, Response};
use async_trait::async_trait;

#[async_trait]
pub trait Filter: Send + Sync {
    async fn before(&self, req: &mut Request) -> Result<Option<Response>, AppError>;
    async fn after(
        &self,
        req: &mut Request,
        result: Result<Response, AppError>,
    ) -> Result<Response, AppError>;
}
