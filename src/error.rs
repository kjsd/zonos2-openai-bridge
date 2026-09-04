use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Zonos backend error: {0}")]
    ZonosError(String),

    #[error("Audio conversion error: {0}")]
    AudioConversionError(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "invalid_request_error", msg.clone()),
            AppError::ZonosError(msg) => (StatusCode::BAD_GATEWAY, "zonos_error", msg.clone()),
            AppError::AudioConversionError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "audio_conversion_error", msg.clone())
            }
            AppError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error",
                err.to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}
