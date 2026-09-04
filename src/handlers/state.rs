use crate::config::Config;
use crate::zonos::ZonosClient;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum GradioEventStatus {
    Generating,
    Done(Value),
    Error(String),
}

pub struct AppState {
    pub config: Config,
    pub zonos: ZonosClient,
    pub gradio_events: Mutex<HashMap<String, GradioEventStatus>>,
}

impl AppState {
    pub fn new(config: Config, zonos: ZonosClient) -> Self {
        Self {
            config,
            zonos,
            gradio_events: Mutex::new(HashMap::new()),
        }
    }
}
