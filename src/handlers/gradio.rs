use crate::audio::float32_to_pcm16_wav;
use crate::error::AppError;
use crate::handlers::state::{AppState, GradioEventStatus};
use crate::parser::EmotionParser;
use crate::zonos::types::ZonosGenerateRequest;
use axum::{
    extract::{Multipart, Path, State},
    http::{header, Method, StatusCode, Uri},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::Stream;
use tracing::{error, info, warn};

const GRADIO_VOICES_DIR: &str = "/tmp/zonos_gradio_voices";

fn ensure_voices_dir() -> PathBuf {
    let p = PathBuf::from(GRADIO_VOICES_DIR);
    if !p.exists() {
        let _ = std::fs::create_dir_all(&p);
    }
    p
}

/// POST /gradio_api/upload
/// Accepts multipart form-data upload of reference audio samples and returns their saved paths.
pub async fn handle_gradio_upload(
    mut multipart: Multipart,
) -> Result<Json<Vec<String>>, AppError> {
    let voices_dir = ensure_voices_dir();
    let mut uploaded_paths = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read multipart field: {e}")))?
    {
        let original_name = field.file_name().unwrap_or("sample.wav").to_string();
        let ext = StdPath::new(&original_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("wav");

        let file_id = uuid::Uuid::new_v4().to_string();
        let save_path = voices_dir.join(format!("gradio_ref_{file_id}.{ext}"));

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read file bytes: {e}")))?;

        tokio::fs::write(&save_path, &data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save uploaded voice: {e}"))?;

        info!(path = ?save_path, size = data.len(), "Gradio voice sample uploaded");
        uploaded_paths.push(save_path.to_string_lossy().to_string());
    }

    Ok(Json(uploaded_paths))
}

#[derive(Debug, Deserialize)]
pub struct GradioGenerateRequest {
    pub data: Vec<Value>,
}

/// POST /gradio_api/call/generate_audio
/// Initiates asynchronous TTS generation from Gradio data payload.
pub async fn handle_gradio_call_generate_audio(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GradioGenerateRequest>,
) -> Result<Json<Value>, AppError> {
    let data = payload.data;

    // Gradio Zonos format:
    // data[1] -> text (e.g. "[whisper] Hello Dovahkiin")
    // data[3] -> speaker (file object {"path": "..."} or raw path string)
    let text = data
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let speaker_path = data
        .get(3)
        .and_then(|v| {
            if let Some(path_str) = v.as_str() {
                Some(path_str.to_string())
            } else if let Some(path_val) = v.get("path").and_then(|p| p.as_str()) {
                Some(path_val.to_string())
            } else {
                None
            }
        });

    let event_id = format!("event_{}", uuid::Uuid::new_v4().simple());

    // Register event state as Generating
    {
        let mut lock = state.gradio_events.lock().await;
        lock.insert(event_id.clone(), GradioEventStatus::Generating);
    }

    let state_clone = Arc::clone(&state);
    let event_id_clone = event_id.clone();

    // Spawn async worker task to run TTS on Zonos 2 backend
    tokio::spawn(async move {
        let result = run_gradio_generation(state_clone.clone(), text, speaker_path, &event_id_clone).await;

        let mut lock = state_clone.gradio_events.lock().await;
        match result {
            Ok(file_info) => {
                lock.insert(event_id_clone, GradioEventStatus::Done(file_info));
            }
            Err(e) => {
                error!(error = %e, "Gradio TTS generation failed");
                lock.insert(event_id_clone, GradioEventStatus::Error(e.to_string()));
            }
        }
    });

    Ok(Json(json!({ "event_id": event_id })))
}

async fn run_gradio_generation(
    state: Arc<AppState>,
    raw_text: String,
    speaker_path: Option<String>,
    event_id: &str,
) -> Result<Value, AppError> {
    // 1. Emotion parsing
    let parsed = EmotionParser::parse(&raw_text);

    // 2. Resolve speaker reference audio
    let mut speaker_audio_base64 = None;
    let mut speaker_embedding_name = None;

    if let Some(ref path_str) = speaker_path {
        let p = StdPath::new(path_str);
        if p.is_file() {
            if let Ok(bytes) = tokio::fs::read(p).await {
                speaker_audio_base64 = Some(BASE64_STANDARD.encode(&bytes));
            }
        }
    }

    if speaker_audio_base64.is_none() {
        speaker_embedding_name = Some(state.config.default_voice.clone());
    }

    info!(
        raw_text = %raw_text,
        cleaned_text = %parsed.cleaned_text,
        tags = ?parsed.detected_tags,
        has_custom_audio = speaker_audio_base64.is_some(),
        "Gradio: processing TTS generation"
    );

    let zonos_req = ZonosGenerateRequest {
        text: parsed.cleaned_text,
        speaker_embedding_name,
        speaker_audio_base64,
        language: "ja".to_string(),
        emotion_sliders: parsed.emotion_sliders,
        emotion_cfg_scale: parsed.emotion_cfg_scale,
        speed: parsed.speed_factor,
        speaking_rate: 15.0 * parsed.speed_factor,
        accurate_mode: true,
        stream: false,
    };

    let raw_pcm = state.zonos.generate_pcm(&zonos_req).await?;
    let wav_bytes = float32_to_pcm16_wav(&raw_pcm, 44100, 1)
        .map_err(AppError::AudioConversionError)?;

    let voices_dir = ensure_voices_dir();
    let out_filename = format!("out_{event_id}.wav");
    let out_path = voices_dir.join(&out_filename);

    tokio::fs::write(&out_path, &wav_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save output WAV: {e}"))?;

    let out_path_str = out_path.to_string_lossy().to_string();
    let file_info = json!({
        "name": out_filename,
        "path": out_path_str,
        "url": format!("/gradio_api/file={out_path_str}"),
        "size": wav_bytes.len(),
        "mime_type": "audio/wav",
        "meta": { "_type": "gradio.FileData" }
    });

    Ok(file_info)
}

/// GET /gradio_api/call/generate_audio/{event_id}
/// Server-Sent Events (SSE) stream waiting for audio completion.
pub async fn handle_gradio_sse(
    Path(event_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("heartbeat").data("null"));

        for _ in 0..1200 { // Up to 120 seconds (100ms * 1200)
            tokio::time::sleep(Duration::from_millis(100)).await;

            let status = {
                let lock = state.gradio_events.lock().await;
                lock.get(&event_id).cloned()
            };

            match status {
                Some(GradioEventStatus::Done(payload)) => {
                    let data_str = json!([payload]).to_string();
                    yield Ok(Event::default().event("complete").data(data_str));
                    break;
                }
                Some(GradioEventStatus::Error(err)) => {
                    let data_str = json!(err).to_string();
                    yield Ok(Event::default().event("error").data(data_str));
                    break;
                }
                Some(GradioEventStatus::Generating) | None => {
                    yield Ok(Event::default().event("generating").data("null"));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn url_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1 as char, h2 as char), 16) {
                    bytes.push(byte);
                    continue;
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// GET /gradio_api/file={file_path}
/// HEAD /gradio_api/file={file_path}
/// Serves generated audio files to Gradio clients.
pub async fn handle_gradio_file(
    uri: Uri,
) -> Result<Response, AppError> {
    let raw_path = uri.path();
    let file_path = if let Some(stripped) = raw_path.strip_prefix("/gradio_api/file=") {
        stripped
    } else if let Some(stripped) = raw_path.strip_prefix("/gradio_api/file/") {
        stripped
    } else if let Some(stripped) = raw_path.strip_prefix("/file=") {
        stripped
    } else {
        return Err(AppError::NotFound(format!("Invalid file path URI: {raw_path}")));
    };

    let decoded = url_decode(file_path);
    let p = StdPath::new(&decoded);
    if !p.is_file() {
        return Err(AppError::NotFound(format!("File not found: {decoded}")));
    }

    let bytes = tokio::fs::read(p)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read file: {e}"))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "audio/wav")],
        bytes,
    )
        .into_response())
}

/// GET /speakers, GET /speakers_list, GET /get_speakers
/// Returns available speaker list for SkyrimNet / XTTS compatibility.
pub async fn handle_speakers(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    Json(vec![
        state.config.default_voice.clone(),
        "nina2".to_string(),
        "AmericanFemale".to_string(),
        "AmericanMale".to_string(),
    ])
}

/// GET /languages
/// Returns supported languages for SkyrimNet compatibility.
pub async fn handle_languages() -> Json<Vec<String>> {
    Json(vec![
        "ja".to_string(),
        "en".to_string(),
        "zh".to_string(),
        "de".to_string(),
        "fr".to_string(),
        "es".to_string(),
        "it".to_string(),
        "ko".to_string(),
        "ru".to_string(),
        "pt".to_string(),
    ])
}

/// Fallback handler for unmapped routes, printing a visible warning with method and URI
pub async fn handle_fallback(
    method: Method,
    uri: Uri,
) -> impl IntoResponse {
    warn!(%method, %uri, "Received request to unmapped route (404)");
    (
        StatusCode::NOT_FOUND,
        format!("Not Found: {method} {uri}"),
    )
}
