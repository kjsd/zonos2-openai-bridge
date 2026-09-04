use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::audio::float32_to_pcm16_wav;
use crate::config::Config;
use crate::error::AppError;
use crate::parser::EmotionParser;
use crate::zonos::{types::ZonosGenerateRequest, ZonosClient};

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

pub struct AppState {
    pub config: Config,
    pub zonos: ZonosClient,
}

pub async fn handle_speech(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SpeechRequest>,
) -> Result<Response, AppError> {
    if payload.input.trim().is_empty() {
        return Err(AppError::BadRequest("Input text cannot be empty".to_string()));
    }

    // 1. Resolve voice (fallback to config.default_voice)
    let voice = match payload.voice.as_deref() {
        Some(v) if !v.trim().is_empty() => v.trim(),
        _ => &state.config.default_voice,
    };

    // 2. Parse emotion tags from input text
    let parsed = EmotionParser::parse(&payload.input);

    if parsed.cleaned_text.is_empty() {
        return Err(AppError::BadRequest(
            "Input text contains only emotion tags without speakable content".to_string(),
        ));
    }

    // 3. Calculate effective speed
    let base_speed = payload.speed.unwrap_or(1.0).clamp(0.25, 4.0);
    let final_speed = base_speed * parsed.speed_factor;

    // 4. Resolve speaker reference audio (base64) vs registered embedding name
    let custom_audio_b64 = payload
        .speaker_audio_base64
        .or(payload.speaker_wav_base64)
        .filter(|s| !s.trim().is_empty());

    let (speaker_embedding_name, speaker_audio_base64) = match custom_audio_b64 {
        Some(b64) => (None, Some(b64)),
        None => (Some(voice.to_string()), None),
    };

    info!(
        original_input = %payload.input,
        cleaned_text = %parsed.cleaned_text,
        voice = %voice,
        has_custom_audio = speaker_audio_base64.is_some(),
        tags = ?parsed.detected_tags,
        final_speed = %final_speed,
        "Processing TTS request"
    );

    // 5. Build Zonos 2 request
    let zonos_req = ZonosGenerateRequest {
        text: parsed.cleaned_text,
        speaker_embedding_name,
        speaker_audio_base64,
        language: "ja".to_string(),
        emotion_sliders: parsed.emotion_sliders,
        emotion_cfg_scale: parsed.emotion_cfg_scale,
        speed: final_speed,
        speaking_rate: 15.0 * final_speed,
        accurate_mode: true,
        stream: false,
    };

    // 6. Call Zonos 2 backend (serialized by internal mutex)
    let raw_pcm = state.zonos.generate_pcm(&zonos_req).await?;

    // 7. Convert float32 PCM to standard 16-bit integer WAV (44.1kHz, Mono)
    let wav_data = float32_to_pcm16_wav(&raw_pcm, 44100, 1)
        .map_err(|e| AppError::AudioConversionError(e))?;

    // 8. Build HTTP response
    let tags_header = parsed.detected_tags.join(", ");
    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"speech.wav\""),
            (header::HeaderName::from_static("x-emotion-tags"), &tags_header),
        ],
        wav_data,
    )
        .into_response();

    Ok(response)
}
