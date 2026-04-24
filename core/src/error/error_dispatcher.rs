use crate::error::{KernelError, EXTERNAL_STATUS_CODE_KEY};
use crate::http::{Response, ResponseBuilder};
use crate::types::JsonValue;
use http::{Method, StatusCode, Uri};

pub struct ErrorDispatcher {
    framework_handler: Box<dyn FrameworkErrorHandler>,
    external_handlers: Vec<Box<dyn ExternalErrorHandler>>,
}

impl Default for ErrorDispatcher {
    fn default() -> Self {
        Self::new(DefaultFrameworkErrorHandler::default(), vec![])
    }
}

impl ErrorDispatcher {
    pub fn new(
        framework_handler: impl FrameworkErrorHandler + 'static,
        external_handlers: Vec<Box<dyn ExternalErrorHandler>>,
    ) -> Self {
        Self {
            framework_handler: Box::new(framework_handler),
            external_handlers,
        }
    }
    pub fn dispatch(&self, err: KernelError) -> Response {
        match err {
            KernelError::NotFound(method, uri) => self.framework_handler.on_not_found(method, uri),
            KernelError::BodyReadFailed(err) => self.framework_handler.on_body_read_failed(err),
            KernelError::External(err) => {
                let matched = self
                    .external_handlers
                    .iter()
                    .find(|handler| handler.matches(&&err));

                if let Some(handler) = matched {
                    return handler.handle(err);
                }

                let mut err = err;

                if let Some(obj) = err.as_object_mut() {
                    let status = obj
                        .get(EXTERNAL_STATUS_CODE_KEY)
                        .and_then(|v| v.as_u64())
                        .and_then(|c| StatusCode::from_u16(c as u16).ok())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                    obj.remove(EXTERNAL_STATUS_CODE_KEY);

                    return ResponseBuilder::new()
                        .status(status)
                        .json_str(err.to_string());
                }

                ResponseBuilder::new()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .json_str(err.to_string())
            }
        }
    }
}

#[derive(Default)]
pub struct DefaultFrameworkErrorHandler {}

impl FrameworkErrorHandler for DefaultFrameworkErrorHandler {
    fn on_not_found(&self, _: Method, _: Uri) -> Response {
        ResponseBuilder::new()
            .status(StatusCode::NOT_FOUND)
            .text("404 Not Found".to_string())
    }

    fn on_body_read_failed(&self, err: String) -> Response {
        ResponseBuilder::new()
            .status(StatusCode::BAD_REQUEST)
            .text(format!("Body read failed: {}", err))
    }
}

pub trait FrameworkErrorHandler: Send + Sync {
    fn on_not_found(&self, method: Method, uri: Uri) -> Response;

    fn on_body_read_failed(&self, err: String) -> Response;
}

pub trait ExternalErrorHandler: Send + Sync {
    fn matches(&self, err: &JsonValue) -> bool;
    fn handle(&self, err: JsonValue) -> Response;
}
