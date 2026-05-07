use crate::error::ErrorPayload;
use crate::model::User;
use crate::repository::UserRepository;
use crate::security::jwt_provider::JwtProvider;
use crate::security::middleware::AuthGuard;
use http::StatusCode;
use ioc_lite::IoC;
use serde::Serialize;
use web_kernel::engine::Context;
use web_kernel::error::KernelError;
use web_kernel::handler;
use web_kernel::handler::PathVariables;
use web_kernel::http::{Request, Response, ResponseBuilder};
use web_kernel::types::JsonValue;

#[handler(method = "POST", route = "/api/users")]
pub async fn user_register(ctx: &mut Context, _: &mut Request) -> Result<Response, KernelError> {
    let json = ctx.get::<JsonValue>();
    let json = unwrap_json_value(json)?;

    let user = User::new(
        None,
        get_json_str(json, "email"),
        get_json_str(json, "name"),
        get_json_str(json, "password"),
    )
    .map_err(|e| {
        KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserRegistrationFailed".to_string(),
                message: format!("Failed to create user: {}", e),
            }
            .into(),
        )
    })?;

    let ioc = ctx.get_injected::<IoC>();
    let repo = ioc.get::<UserRepository>(ctx.trace_id()).await;
    {
        let user = repo.write().await.save(user).await.map_err(|e| {
            KernelError::External(
                ErrorPayload {
                    __ext_status: 500,
                    error_type: "UserRegistrationFailed".to_string(),
                    message: format!("Failed to save user: {}", e),
                }
                .into(),
            )
        })?;

        build_json_response(UserDto::from_user(user))
    }
}

#[handler(method = "POST", route = "/api/users/login")]
pub async fn user_login(ctx: &mut Context, _: &mut Request) -> Result<Response, KernelError> {
    let json = ctx.get::<JsonValue>();
    let json = unwrap_json_value(json)?;

    let ioc = ctx.get_injected::<IoC>();
    let repo = ioc.get::<UserRepository>(ctx.trace_id()).await;
    let user = {
        repo.read()
            .await
            .find_by_email_and_password(
                &get_json_str(json, "email"),
                &get_json_str(json, "password"),
            )
            .await
    };

    if user.is_none() {
        return Err(KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserLoginFailed".to_string(),
                message: "Invalid email or password".to_string(),
            }
            .into(),
        ));
    }

    let user = user.unwrap();
    let jwt_provider = ioc.get::<JwtProvider>(ctx.trace_id()).await;
    let token = { jwt_provider.read().await.generate_token(&user) };

    build_json_response(UserDto::from_user_with_token(user, token))
}

#[handler(method = "GET", route = "/api/users", middleware(AuthGuard::authed()))]
pub async fn user_query(ctx: &mut Context, req: &mut Request) -> Result<Response, KernelError> {
    let ioc = ctx.get_injected::<IoC>();

    let keyword = req
        .uri()
        .query()
        .and_then(|q| {
            form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "keyword")
                .map(|(_, v)| v.into_owned())
        })
        .unwrap_or_else(|| "".to_string());

    let repo = ioc.get::<UserRepository>(ctx.trace_id()).await;
    let users = { repo.read().await.query_by_name_like(&keyword).await };

    let users: Vec<UserDto> = users.into_iter().map(|u| UserDto::from_user(u)).collect();
    build_json_response(users)
}

#[handler(
    method = "PATCH",
    route = "/api/users/{user_id}",
    middleware(auth_guard_with_user_id_same())
)]
pub async fn user_rename(ctx: &mut Context, _: &mut Request) -> Result<Response, KernelError> {
    let user_id = ctx
        .get::<PathVariables>()
        .unwrap()
        .map()
        .get("user_id")
        .and_then(|v| v.parse::<u64>().ok());
    if user_id.is_none() {
        return Err(KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserRenameFailed".to_string(),
                message: "Missing user id".to_string(),
            }
            .into(),
        ));
    }
    let user_id = user_id.unwrap();

    let ioc = ctx.get_injected::<IoC>();
    let repo = ioc.get::<UserRepository>(ctx.trace_id()).await;
    let user = { repo.read().await.find_by_id(user_id).await };
    if user.is_none() {
        return Err(KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserRenameFailed".to_string(),
                message: "User not found".to_string(),
            }
            .into(),
        ));
    }
    let mut user = user.unwrap();

    let json = ctx.get::<JsonValue>();
    let json = unwrap_json_value(json)?;

    user.set_name(get_json_str(json, "newName")).map_err(|e| {
        KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserRenameFailed".to_string(),
                message: format!("Failed to rename user: {}", e),
            }
            .into(),
        )
    })?;

    match { repo.write().await.save(user).await } {
        Ok(_) => Ok(ResponseBuilder::new()
            .status(StatusCode::from_u16(204).unwrap())
            .build()),
        Err(e) => Err(KernelError::External(
            ErrorPayload {
                __ext_status: 500,
                error_type: "UserRenameFailed".to_string(),
                message: format!("Failed to save user: {}", e),
            }
            .into(),
        )),
    }
}

fn auth_guard_with_user_id_same() -> AuthGuard {
    AuthGuard::authed_with(|ctx, principal| {
        let path_variables = ctx.get::<PathVariables>().unwrap();

        let user_id = path_variables
            .map()
            .get("user_id")
            .and_then(|v| v.parse::<u64>().ok());

        if user_id.is_none() {
            return false;
        }

        principal.id() == user_id.unwrap()
    })
}

fn get_json_str(json: &JsonValue, key: &str) -> String {
    json.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn unwrap_json_value(json: Option<&JsonValue>) -> Result<&JsonValue, KernelError> {
    match json {
        Some(json) => Ok(json),
        None => Err(KernelError::External(
            ErrorPayload {
                __ext_status: 400,
                error_type: "UserRegistrationFailed".to_string(),
                message: "Missing JSON body".to_string(),
            }
            .into(),
        )),
    }
}

fn build_json_response<T: Serialize>(data: T) -> Result<Response, KernelError> {
    ResponseBuilder::new().json(data).map_err(|_| {
        KernelError::External(
            ErrorPayload {
                __ext_status: 500,
                error_type: "InternalError".to_string(),
                message: "Failed to serialize data".to_string(),
            }
            .into(),
        )
    })
}

#[derive(Serialize)]
struct UserDto {
    id: u64,
    email: String,
    name: String,
    token: Option<String>,
}
impl UserDto {
    fn from_user(user: User) -> Self {
        Self {
            id: user.id().unwrap(),
            email: user.email().to_string(),
            name: user.name().to_string(),
            token: None,
        }
    }

    fn from_user_with_token(user: User, token: String) -> Self {
        Self {
            id: user.id().unwrap(),
            email: user.email().to_string(),
            name: user.name().to_string(),
            token: Some(token),
        }
    }
}
