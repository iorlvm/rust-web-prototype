use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Response as LibResponse, StatusCode};
use http_body_util::combinators::BoxBody;
use serde::Serialize;
use std::convert::Infallible;

pub type HttpResponse = LibResponse<BoxBody<Bytes, Infallible>>;

pub enum ResponseBody {
    Text(String),
    Bytes(Vec<u8>),
    None,
}

pub struct Response {
    status: StatusCode,
    header: HeaderMap,
    data: ResponseBody,
}

impl Response {
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    pub fn header(&self) -> &HeaderMap {
        &self.header
    }

    pub fn data(&self) -> &ResponseBody {
        &self.data
    }

    pub fn into_http_response(self) -> HttpResponse {
        let mut builder = LibResponse::builder().status(self.status);

        for (k, v) in &self.header {
            builder = builder.header(k, v);
        }

        builder
            .body(match self.data {
                ResponseBody::Text(s) => BoxBody::new(http_body_util::Full::new(Bytes::from(s))),
                ResponseBody::Bytes(b) => BoxBody::new(http_body_util::Full::new(Bytes::from(b))),
                ResponseBody::None => BoxBody::new(http_body_util::Empty::new()),
            })
            .unwrap()
    }
}

pub struct ResponseBuilder {
    status: StatusCode,
    header: HeaderMap,
    data: ResponseBody,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            status: StatusCode::OK,
            header: HeaderMap::new(),
            data: ResponseBody::None,
        }
    }
    pub fn from(r: Response) -> Self {
        ResponseBuilder {
            status: r.status,
            header: r.header,
            data: r.data,
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if let (Ok(name), Ok(value)) = (
            HeaderName::try_from(key.into()),
            HeaderValue::try_from(value.into()),
        ) {
            self.header.append(name, value);
        }
        self
    }

    pub fn text(mut self, text: String) -> Response {
        self.header("Content-Type", "text/plain; charset=utf-8")
            .body(ResponseBody::Text(text))
    }

    pub fn json<T: Serialize>(mut self, json: T) -> Result<Response, serde_json::Error> {
        let json_str = serde_json::to_string(&json)?;
        Ok(self
            .header("Content-Type", "application/json; charset=utf-8")
            .body(ResponseBody::Text(json_str)))
    }

    pub fn bytes(self, bytes: Vec<u8>) -> Response {
        self.body(ResponseBody::Bytes(bytes))
    }

    pub fn none(self) -> Response {
        self.body(ResponseBody::None)
    }

    pub fn build(self) -> Response {
        Response {
            status: self.status,
            header: self.header,
            data: self.data,
        }
    }

    fn body(mut self, data: ResponseBody) -> Response {
        self.data = data;
        self.build()
    }
}
