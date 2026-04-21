use crate::infra::response::{Response, ResponseBuilder};
use crate::routing::register::{RouteRegister, RouteRegisterBuilder};
use crate::runtime::context::Context;
use crate::types::http_types::{HttpRequest, HttpResponse};
use http::StatusCode;
use std::convert::Infallible;

pub struct Dispatcher {
    register: RouteRegister,
}

impl Dispatcher {
    pub fn build() -> Self {
        Self {
            // TODO: 後續實現
            register: RouteRegisterBuilder::new().build(),
        }
    }

    pub async fn dispatch(&self, req: HttpRequest) -> Result<HttpResponse, Infallible> {
        let ctx = Context::from(req);
        let routes = &self.register.get_routes(ctx.req.method());

        if routes.is_some() {
            for route in routes.unwrap() {
                if !route.matches(ctx.req.method(), ctx.req.uri().path()) {
                    continue;
                }

                let resp = route.execute(ctx).await.unwrap_or_else(|err| err.into());

                // TODO: finalize process
                let resp = ResponseBuilder::from(resp)
                    .header("Server", "Rust-Framework")
                    .build();

                return Ok(resp.into_http_response());
            }
        }

        println!("Not Found: {}", ctx.req.uri());
        let resp = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .text("Not Found".to_string())
            .build();

        Ok(resp.into_http_response())
    }
}
