use serde::Serialize;
use web_kernel::types::JsonValue;

#[derive(Serialize)]
pub struct ErrorPayload {
    pub __ext_status: u16,
    pub error_type: String,
    pub message: String,
}

impl Into<JsonValue> for ErrorPayload {
    fn into(self) -> JsonValue {
        serde_json::to_value(self).expect("Failed to serialize error")
    }
}
