use crate::error::AppError;
use crate::filter::Filter;
use crate::http::{Request, Response};
use async_trait::async_trait;

pub type Multipart = Vec<Filed>;

pub struct Filed {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

pub struct MultipartResolver {}

impl MultipartResolver {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Filter for MultipartResolver {
    async fn before(&self, req: &mut Request) -> Result<Option<Response>, AppError> {
        Ok(None)
    }

    async fn after(
        &self,
        req: &mut Request,
        result: Result<Response, AppError>,
    ) -> Result<Response, AppError> {
        result
    }
}
