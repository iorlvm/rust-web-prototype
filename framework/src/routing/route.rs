use crate::contract::error::AppError;
use crate::infra::response::Response;
use crate::runtime::context::Context;
use crate::types::async_types::AsyncResult;
use http::{Method, StatusCode};
use std::collections::HashMap;

pub enum Order {
    Highest, // only for frameworks
    High,
    Normal,
    Low,
    Lowest, // only for frameworks
}
impl Order {
    fn weight(&self) -> u8 {
        match self {
            Order::Highest => 0,
            Order::High => 63,
            Order::Normal => 127,
            Order::Low => 191,
            Order::Lowest => 255,
        }
    }
}

pub trait HandleUnit: Send + Sync {
    fn order(&self) -> Order {
        Order::Normal
    }
    fn execute(&self, ctx: &mut Context) -> AsyncResult<(), AppError>;
}
pub struct Route {
    method: Method,
    path: String,
    pre_filters: Vec<Box<dyn HandleUnit>>,
    handler: Box<dyn HandleUnit>,
    post_filters: Vec<Box<dyn HandleUnit>>,
}

impl Route {
    pub fn method(&self) -> &Method {
        &self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn matches(&self, method: &Method, path: &str) -> bool {
        // TODO: 臨時實作, 待完成
        self.method == method && self.path == path
    }

    pub async fn execute(&self, mut ctx: Context) -> Result<Response, AppError> {
        for unit in &self.pre_filters {
            unit.execute(&mut ctx).await?;
            if ctx.res.is_some() {
                return Ok(ctx.res.unwrap());
            }
        }

        self.handler.execute(&mut ctx).await?;
        if ctx.res.is_none() {
            return Err(AppError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "handler is missing response".to_string(),
            });
        }

        for unit in &self.post_filters {
            unit.execute(&mut ctx).await?;
            if ctx.res.is_none() {
                return Err(AppError {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "after handler is missing response".to_string(),
                });
            }
        }

        Ok(ctx.res.unwrap())
    }

    pub fn builder() -> RouteBuilder {
        RouteBuilder {
            method: None,
            path: None,
            pre_filters: Vec::new(),
            handler: None,
            post_filters: Vec::new(),
        }
    }
}

pub struct RouteBuilder {
    method: Option<Method>,
    path: Option<String>,
    handler: Option<Box<dyn HandleUnit>>,
    pre_filters: Vec<Box<dyn HandleUnit>>,
    post_filters: Vec<Box<dyn HandleUnit>>,
}

impl RouteBuilder {
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }
    pub fn path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }
    pub fn add_pre_filter(mut self, unit: Box<dyn HandleUnit>) -> Self {
        self.pre_filters.push(unit);
        self
    }
    pub fn handler(mut self, unit: Box<dyn HandleUnit>) -> Self {
        self.handler = Some(unit);
        self
    }
    pub fn add_post_filter(mut self, unit: Box<dyn HandleUnit>) -> Self {
        self.post_filters.push(unit);
        self
    }

    pub fn build(mut self) -> Route {
        sort_units(&mut self.pre_filters);
        sort_units(&mut self.post_filters);

        Route {
            method: self.method.expect("method is required"),
            path: self.path.expect("path is required"),
            pre_filters: self.pre_filters,
            handler: self.handler.expect("handler is required"),
            post_filters: self.post_filters,
        }
    }
}

fn sort_units(v: &mut Vec<Box<dyn HandleUnit>>) {
    v.sort_by_key(|u| u.order().weight());
}
