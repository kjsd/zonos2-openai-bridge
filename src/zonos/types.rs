use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ZonosGenerateRequest {
    pub text: String,
    pub speaker_embedding_name: String,
    pub language: String,
    pub emotion_sliders: HashMap<String, f32>,
    pub emotion_cfg_scale: f32,
    pub speed: f32,
    pub speaking_rate: f32,
    pub accurate_mode: bool,
    pub stream: bool,
}
