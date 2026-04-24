use crate::error::AppError;
use crate::middleware::req_body_extractors::RequestBodyExtractor;
use bytes::Bytes;

pub type Multipart = Vec<Filed>;

pub struct Filed {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct MultipartExtractor {}

impl RequestBodyExtractor for MultipartExtractor {
    type Output = Multipart;

    fn matches(&self, content_type: &str) -> bool {
        false
    }

    fn convert(&self, bytes: Bytes) -> Result<Self::Output, AppError> {
        todo!()
    }
}
