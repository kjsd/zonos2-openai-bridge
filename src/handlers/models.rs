use crate::handlers::AppState;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_models(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": &state.config.default_model,
                "object": "model",
                "created": 1725400000,
                "owned_by": "zyphra"
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
