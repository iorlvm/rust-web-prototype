use bytes::Bytes;
use http::{Request, Response};
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use std::convert::Infallible;

pub type HttpBody = BoxBody<Bytes, Infallible>;

pub type HttpRequest = Request<Incoming>;
pub type HttpResponse = Response<HttpBody>;
