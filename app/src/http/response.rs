use axum::{
    Json,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseFormat<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

pub struct HttpResponse<T> {
    status: StatusCode,
    body: HttpResponseFormat<T>,
    headers: HeaderMap,
}

impl<T> HttpResponse<T> {
    pub fn new(message: impl Into<String>, status: StatusCode, data: Option<T>) -> Self {
        Self {
            status,
            body: HttpResponseFormat {
                success: true,
                message: message.into(),
                data,
            },
            headers: HeaderMap::new(),
        }
    }

    pub fn ok(data: T, message: impl Into<String>) -> Self {
        Self::new(message, StatusCode::OK, Some(data))
    }

    pub fn created(data: T, message: impl Into<String>) -> Self {
        Self::new(message, StatusCode::CREATED, Some(data))
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.append(name, value);
        self
    }
}

impl HttpResponse<serde_json::Value> {
    pub fn done(message: impl Into<String>) -> Self {
        Self::new(message, StatusCode::OK, None)
    }
}

impl<T: Serialize> IntoResponse for HttpResponse<T> {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.body)).into_response();
        response.headers_mut().extend(self.headers);
        response
    }
}
