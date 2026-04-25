use crate::error::KernelError;
use crate::middleware::req_body_extractors::RequestBodyExtractor;
use crate::types::JsonValue;
use bytes::Bytes;

#[derive(Default)]
pub struct JsonExtractor {}

impl RequestBodyExtractor for JsonExtractor {
    type Output = JsonValue;

    fn matches(&self, content_type: &str) -> bool {
        content_type.starts_with("application/json")
    }

    fn convert(&self, bytes: Bytes) -> Result<Self::Output, KernelError> {
        serde_json::from_slice::<JsonValue>(&bytes)
            .map_err(|_| KernelError::BodyReadFailed(String::from("Invalid JSON")))
    }
}
