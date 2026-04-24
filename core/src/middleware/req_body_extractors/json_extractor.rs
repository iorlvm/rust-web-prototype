use crate::error::AppError;
use crate::middleware::req_body_extractors::RequestBodyExtractor;
use bytes::Bytes;

pub type JsonValue = serde_json::Value;

#[derive(Default)]
pub struct JsonExtractor {}

impl RequestBodyExtractor for JsonExtractor {
    type Output = JsonValue;

    fn matches(&self, content_type: &str) -> bool {
        content_type.starts_with("application/json")
    }

    fn convert(&self, bytes: Bytes) -> Result<Self::Output, AppError> {
        serde_json::from_slice::<JsonValue>(&bytes)
            .map_err(|_| AppError::BodyExt(String::from("Invalid JSON")))
    }
}
