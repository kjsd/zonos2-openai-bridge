use async_trait::async_trait;
use std::sync::Arc;

use crate::config::Config;
use crate::error::AppError;

pub mod irodori;
pub mod zonos2;

pub use irodori::IrodoriEngine;
pub use zonos2::ZonosEngine;

#[derive(Debug, Clone, Default)]
pub struct SynthesisRequest {
    /// Raw input text (may contain emotion/audio tags)
    pub text: String,
    /// Target voice name or speaker identifier
    pub voice: Option<String>,
    /// Optional raw custom speaker reference audio bytes (from Gradio upload or decoded base64)
    pub custom_speaker_audio_bytes: Option<Vec<u8>>,
    /// Optional base64 encoded speaker audio
    pub custom_speaker_audio_base64: Option<String>,
    /// User-specified base speed factor (defaults to 1.0)
    pub speed: Option<f32>,
    /// Requested response format (e.g. "wav")
    pub response_format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SynthesisResult {
    /// 16-bit PCM WAV audio bytes
    pub audio_bytes: Vec<u8>,
    /// MIME content-type (e.g. "audio/wav")
    pub content_type: String,
    /// Emotion/action tags detected during processing
    pub detected_tags: Vec<String>,
    /// Final text processed by the engine (for logging/debug)
    pub processed_text: String,
}

#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Engine identifier name ("zonos2", "irodori", etc.)
    fn name(&self) -> &'static str;

    /// Default model name for this engine
    fn default_model(&self) -> &str;

    /// Synthesize speech from request
    async fn synthesize(&self, req: SynthesisRequest) -> Result<SynthesisResult, AppError>;

    /// Health check for the backend engine
    async fn health_check(&self) -> Result<(), AppError>;
}

/// Factory function to instantiate the configured TTS engine
pub fn create_engine(config: &Config) -> Result<Arc<dyn TtsEngine>, AppError> {
    match config.tts_engine.to_lowercase().as_str() {
        "zonos2" | "zonos" => Ok(Arc::new(ZonosEngine::new(config)?)),
        "irodori" | "irodori-tts" => Ok(Arc::new(IrodoriEngine::new(config)?)),
        other => Err(AppError::BadRequest(format!(
            "Unsupported TTS engine: '{}'. Supported engines: 'zonos2', 'irodori'",
            other
        ))),
    }
}
