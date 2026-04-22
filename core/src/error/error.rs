use crate::http::{Response, ResponseBuilder};
use http::StatusCode;

// TODO: 目前顆粒度較粗, 待拓展
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

pub struct ErrorResponder {
    handlers: Vec<Box<dyn ErrorHandler>>,
}
impl ErrorResponder {
    pub fn new() -> Self {
        Self { handlers: vec![] }
    }
    pub fn handle(&self, err: AppError) -> Response {
        for handler in &self.handlers {
            if handler.matches(&err) {
                return handler.handle(err);
            }
        }

        ResponseBuilder::new()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .text("Internal Server Error".to_string())
    }
}

pub trait ErrorHandler: Send + Sync {
    fn matches(&self, err: &AppError) -> bool;
    fn handle(&self, err: AppError) -> Response;
}
