use crate::handlers::AppState;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_models(State(state): State<Arc<AppState>>) -> Json<Value> {
    let engine_model = state.engine.default_model();
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": engine_model,
                "object": "model",
                "created": 1725400000,
                "owned_by": state.engine.name()
            },
            {
                "id": "tts-1",
                "object": "model",
                "created": 1725400000,
                "owned_by": "openai"
            },
            {
                "id": "tts-1-hd",
                "object": "model",
                "created": 1725400000,
                "owned_by": "openai"
            }
        ]
    }))
}
