use crate::error::{AppError, ErrorResponder};
use crate::filter::Filter;
use crate::handler::{Handler, HandlerRegistry};
use crate::http::{HttpRequest, HttpResponse, Request, Response};
use http::StatusCode;
use std::convert::Infallible;
use std::sync::Arc;

pub struct Kernel<T: Send + Sync + 'static> {
    injected: Arc<T>,
    registry: HandlerRegistry,
    filters: Vec<Box<dyn Filter>>,
    error_responder: ErrorResponder,
}

impl<T: Send + Sync + 'static> Kernel<T> {
    pub fn new(
        injected: T,
        registry: HandlerRegistry,
        filters: Vec<Box<dyn Filter>>,
        error_responder: ErrorResponder,
    ) -> Self {
        Self {
            injected: Arc::new(injected),
            registry,
            filters,
            error_responder,
        }
    }
    pub async fn handle(&self, req: HttpRequest) -> Result<HttpResponse, Infallible> {
        let handler = self.find_handler(req.method(), req.uri().path());

        let resp = match handler {
            Some(handler) => self
                .request_chain(handler, Request::from(req))
                .await
                .unwrap_or_else(|err| self.error_responder.handle(err)),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .text("Not Found".to_string()),
        };

        Ok(resp.into_http_response())
    }

    fn find_handler(&self, method: &http::Method, path: &str) -> Option<&dyn Handler> {
        self.registry.get_handlers(method).and_then(|handlers| {
            handlers
                .iter()
                .find(|handler| handler.matches(method, path))
                .map(|handler| handler.as_ref())
        })
    }

    /// 使用線性 loop 模擬 stack（LIFO）執行模型。
    ///
    /// 流程：before（正向）→ handler → after（反向）
    ///
    /// - before：依序執行，可 short-circuit（回應或錯誤）
    /// - handler：僅在未被 before 中斷時執行
    /// - after：反向執行，用於後製處理或 before 階段資源釋放
    ///
    /// 設計重點：
    /// - `last`：標記實際進入的最後一個 filter，限定 after 回溯範圍
    /// - `result_opt`：承接 before 的 short-circuit，避免進入 handler
    /// - after 為單一收斂點（response transform + error propagation）
    ///
    /// 實作上以 index + reverse iteration 模擬 stack unwind。
    async fn request_chain(
        &self,
        handler: &dyn Handler,
        mut req: Request,
    ) -> Result<Response, AppError> {
        req.insert(self.injected.clone());

        let mut last: Option<usize> = None;
        let mut result_opt: Option<Result<Response, AppError>> = None;

        // before phase
        for (i, filter) in self.filters.iter().enumerate() {
            last = Some(i);

            match filter.before(&mut req).await {
                Ok(Some(res)) => {
                    result_opt = Some(Ok(res));
                    break;
                }
                Err(err) => {
                    result_opt = Some(Err(err));
                    break;
                }
                Ok(None) => continue,
            }
        }

        // handler phase
        let mut result = match result_opt {
            Some(res) => res,
            None => handler.execute(&mut req).await,
        };

        // after phase
        if let Some(last) = last {
            for i in (0..=last).rev() {
                let filter = &self.filters[i];
                result = filter.after(&mut req, result).await;
            }
        }

        result
    }
}
