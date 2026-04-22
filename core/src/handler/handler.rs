use crate::error::AppError;
use crate::http::{Request, Response};
use async_trait::async_trait;
use http::Method;

#[async_trait]
pub trait Handler: Send + Sync {
    fn method(&self) -> &Method;
    fn path_pattern(&self) -> &str;
    fn matches(&self, method: &Method, path: &str) -> bool;
    async fn execute(&self, ctx: &mut Request) -> Result<Response, AppError>;
}
