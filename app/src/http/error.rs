use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::constants::errors::INTERNAL_ERROR;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub success: bool,
    pub error: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub errors: BTreeMap<String, String>,
}

impl HttpError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            errors: BTreeMap::from([(code.into(), message.into())]),
        }
    }

    pub fn validation(errors: BTreeMap<String, String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            errors,
        }
    }

    pub fn bad_request(code: &str, message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }
    pub fn unauthorized(code: &str, message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }
    pub fn forbidden(code: &str, message: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }
    pub fn not_found(code: &str, message: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }
    pub fn conflict(code: &str, message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            INTERNAL_ERROR,
            "An internal error occurred.",
        )
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                success: false,
                error: self.errors,
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for HttpError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database operation failed");
        Self::internal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::errors::{EMAIL_INVAILD, EMAIL_INVAILD_MESSAGE};

    #[test]
    fn serializes_required_shape() {
        let body = ErrorBody {
            success: false,
            error: BTreeMap::from([(EMAIL_INVAILD.into(), EMAIL_INVAILD_MESSAGE.into())]),
        };
        assert_eq!(
            serde_json::to_value(body).unwrap(),
            serde_json::json!({"success":false,"error":{"EMAIL_INVAILD":"Please use proper email"}})
        );
    }
}
