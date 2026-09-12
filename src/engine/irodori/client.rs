use crate::engine::irodori::types::IrodoriSpeechRequest;
use crate::error::AppError;
use bytes::Bytes;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Clone)]
pub struct IrodoriClient {
    client: Client,
    base_url: String,
    // Concurrency guard to serialize GPU execution if needed
    lock: Arc<Mutex<()>>,
}

impl IrodoriClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client for Irodori");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Calls Irodori-TTS-Server /v1/audio/speech and returns audio bytes (WAV).
    pub async fn generate_wav(&self, req: &IrodoriSpeechRequest) -> Result<Bytes, AppError> {
        let _guard = self.lock.lock().await;

        let url = format!("{}/v1/audio/speech", self.base_url);
        debug!(
            url = %url,
            text = %req.input,
            voice = ?req.voice,
            model = %req.model,
            "Calling Irodori-TTS backend"
        );

        let response = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| AppError::EngineError(format!("Failed to connect to Irodori-TTS: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::EngineError(format!(
                "Irodori-TTS returned error (HTTP {status}): {error_body}"
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::EngineError(format!("Failed to read audio response bytes: {e}")))?;

        info!(
            text_len = req.input.len(),
            bytes_received = bytes.len(),
            "Successfully received audio from Irodori-TTS"
        );

        Ok(bytes)
    }

    /// Health check for Irodori-TTS backend
    pub async fn health_check(&self) -> bool {
        let health_url = format!("{}/health", self.base_url);
        if let Ok(resp) = self.client.get(&health_url).send().await {
            if resp.status().is_success() {
                return true;
            }
        }

        let models_url = format!("{}/v1/models", self.base_url);
        match self.client.get(&models_url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
