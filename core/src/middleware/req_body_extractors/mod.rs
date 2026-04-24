mod json_extractor;
mod multipart_extractor;

use crate::error::KernelError;
use crate::http::{Request, Response};
use crate::middleware::Middleware;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;

pub use json_extractor::*;
pub use multipart_extractor::*;

pub trait RequestBodyExtractor: Send + Sync {
    type Output: 'static + Send + Sync;

    fn matches(&self, content_type: &str) -> bool;

    fn convert(&self, bytes: Bytes) -> Result<Self::Output, KernelError>;
}

#[async_trait]
impl<T> Middleware for T
where
    T: RequestBodyExtractor + Send + Sync,
{
    async fn before(&self, req: &mut Request) -> Result<Option<Response>, KernelError> {
        let Some(content_type) = req.content_type() else {
            return Ok(None);
        };

        if !self.matches(content_type) {
            return Ok(None);
        }

        let body = match req.take_body() {
            Some(body) => body,
            None => return Ok(None),
        };

        let bytes = body
            .collect()
            .await
            .map_err(|_| KernelError::BodyReadFailed(String::from("Body collect error")))?
            .to_bytes();

        let body = self.convert(bytes)?;
        req.insert(body);

        Ok(None)
    }

    async fn after(
        &self,
        _: &mut Request,
        result: Result<Response, KernelError>,
    ) -> Result<Response, KernelError> {
        result
    }
}
