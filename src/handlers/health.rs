use crate::handlers::AppState;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let zonos_connected = state.zonos.health_check().await;

    Json(json!({
        "status": "ok",
        "zonos_connected": zonos_connected,
        "default_voice": &state.config.default_voice,
        "default_model": &state.config.default_model,
        "zonos_url": &state.config.zonos_url,
    }))
}
