use crate::handlers::AppState;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let engine_connected = state.engine.health_check().await.is_ok();

    Json(json!({
        "status": "ok",
        "engine": state.engine.name(),
        "engine_connected": engine_connected,
        "zonos_connected": engine_connected,
        "default_voice": &state.config.default_voice,
        "default_model": state.engine.default_model(),
    }))
}
