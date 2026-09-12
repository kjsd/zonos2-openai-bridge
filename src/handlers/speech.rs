use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::engine::SynthesisRequest;
use crate::error::AppError;
use crate::handlers::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct SpeechRequest {
    pub model: Option<String>,
    pub input: String,
    pub voice: Option<String>,
    pub response_format: Option<String>,
    pub speed: Option<f32>,
    pub speaker_audio_base64: Option<String>,
    pub speaker_wav_base64: Option<String>,
}

pub async fn handle_speech(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SpeechRequest>,
) -> Result<Response, AppError> {
    if payload.input.trim().is_empty() {
        return Err(AppError::BadRequest("Input text cannot be empty".to_string()));
    }

    if payload.input.chars().count() > 4096 {
        return Err(AppError::BadRequest(
            "Input text exceeds maximum limit of 4096 characters (OpenAI specification)".to_string(),
        ));
    }

    let custom_b64 = payload
        .speaker_audio_base64
        .or(payload.speaker_wav_base64)
        .filter(|s| !s.trim().is_empty());

    let req = SynthesisRequest {
        text: payload.input,
        voice: payload.voice,
        custom_speaker_audio_bytes: None,
        custom_speaker_audio_base64: custom_b64,
        speed: payload.speed,
        response_format: payload.response_format,
    };

    let result = state.engine.synthesize(req).await?;

    let tags_header = result.detected_tags.join(", ");
    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, result.content_type.as_str()),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"speech.wav\""),
            (header::HeaderName::from_static("x-emotion-tags"), &tags_header),
        ],
        result.audio_bytes,
    )
        .into_response();

    Ok(response)
}
