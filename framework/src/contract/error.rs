use http::StatusCode;
use crate::infra::response::Response;

// TODO: 目前顆粒度較粗, 待拓展
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl From<AppError> for Response {
    fn from(value: AppError) -> Self {
        Response::builder()
            .status(value.status)
            .text(value.message.to_string())
            .build()
    }
}