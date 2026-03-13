use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"error":"serialization failed"}"#.to_string()
        });
        (
            self.status,
            [("content-type", "application/json")],
            body,
        )
            .into_response()
    }
}

impl From<pe_core::CoreError> for ApiError {
    fn from(e: pe_core::CoreError) -> Self {
        ApiError::bad_request(e.to_string())
    }
}
