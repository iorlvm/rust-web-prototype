use crate::middleware::req_body_extractors::Filed;

pub type JsonValue = serde_json::Value;
pub type Multipart = Vec<Filed>;
