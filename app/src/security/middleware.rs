use crate::error::ErrorPayload;
use crate::security::jwt_provider::{Authentication, JwtProvider, Principal};
use async_trait::async_trait;
use ioc_lite::IoC;
use web_kernel::engine::Context;
use web_kernel::error::KernelError;
use web_kernel::http::{Request, Response};
use web_kernel::middleware::Middleware;

#[derive(Default)]
pub struct JwtAuthMiddleware;

#[async_trait]
impl Middleware for JwtAuthMiddleware {
    async fn before(
        &self,
        ctx: &mut Context,
        req: &mut Request,
    ) -> Result<Option<Response>, KernelError> {
        let token = req
            .header("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");

        let authentication = ctx
            .get_injected::<IoC>()
            .get::<JwtProvider>()
            .resolve_token(token)
            .await?;

        ctx.insert(authentication);

        Ok(None)
    }
}

pub struct AuthGuard {
    guard: Box<dyn Fn(&Context, &Authentication) -> bool + Send + Sync>,
}

impl AuthGuard {
    pub fn authed() -> Self {
        Self::authed_with(|_, _| true)
    }

    pub fn authed_with(guard: fn(ctx: &Context, principal: &Principal) -> bool) -> Self {
        Self {
            guard: Box::new(move |ctx, auth_info| match auth_info {
                Authentication::Anonymous => false,
                Authentication::User(p) => guard(ctx, p),
            }),
        }
    }
}

#[async_trait]
impl Middleware for AuthGuard {
    async fn before(
        &self,
        ctx: &mut Context,
        _: &mut Request,
    ) -> Result<Option<Response>, KernelError> {
        let auth_info = ctx
            .get::<Authentication>()
            .expect("Missing required authentication middleware: JwtAuthMiddleware");

        if (self.guard)(ctx, auth_info) {
            Ok(None)
        } else {
            Err(KernelError::External(
                ErrorPayload {
                    __ext_status: 403,
                    error_type: "auth-guard".to_string(),
                    message: "Unauthorized".to_string(),
                }
                .into(),
            ))
        }
    }
}
