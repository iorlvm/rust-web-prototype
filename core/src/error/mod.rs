mod error_dispatcher;

use crate::types::JsonValue;
use http::{Method, Uri};

pub use error_dispatcher::*;

pub const EXTERNAL_STATUS_CODE_KEY: &str = "__ext_status";

pub enum KernelError {
    NotFound(Method, Uri),
    BodyReadFailed(String),
    External(JsonValue),
}
