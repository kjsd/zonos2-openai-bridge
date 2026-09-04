use crate::config::Config;
use crate::handlers::{handle_health, handle_models, handle_speech, AppState};
use crate::zonos::ZonosClient;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/v1/audio/speech", post(handle_speech))
        .route("/v1/models", get(handle_models))
        .route("/health", get(handle_health))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run_server(config: Config) -> anyhow::Result<()> {
    let zonos_client = ZonosClient::new(config.zonos_url.clone());
    let bind_addr = format!("{}:{}", config.host, config.port);

    let state = Arc::new(AppState {
        config: config.clone(),
        zonos: zonos_client,
    });

    let app = create_router(state);

    info!(
        addr = %bind_addr,
        zonos_url = %config.zonos_url,
        default_voice = %config.default_voice,
        "Starting zonos2-openai-bridge server"
    );

    let listener = TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
