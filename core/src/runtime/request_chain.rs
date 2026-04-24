use crate::error::KernelError;
use crate::http::{Request, Response};
use crate::middleware::Middleware;
use crate::runtime::Endpoint;

/// 使用線性 loop 模擬 stack（LIFO）執行模型。
///
/// 流程：before（正向）→ handler → after（反向）
///
/// - before：依序執行，可 short-circuit（回應或錯誤）
/// - handler：僅在未被 before 中斷時執行
/// - after：反向執行，用於後製處理或 before 階段資源釋放
///
/// 設計重點：
/// - `last`：標記實際進入的最後一個 builder，限定 after 回溯範圍
/// - `result_opt`：承接 before 的 short-circuit，避免進入 handler
/// - after 為單一收斂點（response transform + error propagation）
///
/// 實作上以 index + reverse iteration 模擬 stack unwind。
pub async fn request_chain(
    req: &mut Request,
    endpoint: &dyn Endpoint,
    middleware: &[Box<dyn Middleware>],
) -> Result<Response, KernelError> {
    let mut last: Option<usize> = None;
    let mut result_opt: Option<Result<Response, KernelError>> = None;

    // before phase
    for (i, cur) in middleware.iter().enumerate() {
        last = Some(i);

        match cur.before(req).await {
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
        None => endpoint.execute(req).await,
    };

    // after phase
    if let Some(last) = last {
        for i in (0..=last).rev() {
            let cur = &middleware[i];
            result = cur.after(req, result).await;
        }
    }

    result
}
