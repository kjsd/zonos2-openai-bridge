use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_tts_bridge::{create_engine, create_router, AppState, Config};

fn create_irodori_test_config(irodori_url: String) -> Config {
    Config {
        tts_engine: "irodori".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8000,
        zonos_url: "http://127.0.0.1:1919".to_string(),
        irodori_url,
        irodori_model: "irodori-tts".to_string(),
        default_voice: "default".to_string(),
        default_model: "irodori-tts".to_string(),
        log_level: "error".to_string(),
        max_body_size_mb: 100,
        default_speed: 1.0,
    }
}

// Generate dummy 16-bit PCM WAV bytes for mock responses
fn make_dummy_wav() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&36u32.to_le_bytes());
    header.extend_from_slice(b"WAVE");
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM
    header.extend_from_slice(&1u16.to_le_bytes()); // Mono
    header.extend_from_slice(&44100u32.to_le_bytes()); // Sample rate
    header.extend_from_slice(&88200u32.to_le_bytes()); // Byte rate
    header.extend_from_slice(&2u16.to_le_bytes()); // Block align
    header.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
    header.extend_from_slice(b"data");
    header.extend_from_slice(&0u32.to_le_bytes());
    header
}

#[tokio::test]
async fn test_irodori_models_endpoint() {
    let config = create_irodori_test_config("http://127.0.0.1:9999".to_string());
    let engine = create_engine(&config).unwrap();
    let state = Arc::new(AppState::with_engine(config, engine));
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let models = json["data"].as_array().unwrap();
    assert!(models.iter().any(|m| m["id"] == "irodori-tts"));
}

#[tokio::test]
async fn test_irodori_speech_tag_to_emoji_conversion() {
    let mock_server = MockServer::start().await;
    let dummy_wav = make_dummy_wav();

    // Verify that the mock server receives text with [whisper] converted to 👂 emoji
    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .respond_with(move |req: &wiremock::Request| {
            let body_json: Value = serde_json::from_slice(&req.body).unwrap();
            let input_text = body_json["input"].as_str().unwrap();
            assert!(input_text.contains('👂'), "Text must contain whisper emoji");
            assert!(input_text.contains("パパ、寒くない？"));
            assert!(!input_text.contains("[whisper]"));

            ResponseTemplate::new(200)
                .set_body_bytes(dummy_wav.clone())
                .insert_header("content-type", "audio/wav")
        })
        .mount(&mock_server)
        .await;

    let config = create_irodori_test_config(mock_server.uri());
    let engine = create_engine(&config).unwrap();
    let state = Arc::new(AppState::with_engine(config, engine));
    let app = create_router(state);

    let speech_req = json!({
        "model": "irodori-tts",
        "input": "[whisper] パパ、寒くない？",
        "voice": "default"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&speech_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/wav"
    );
    assert_eq!(
        response.headers().get("x-emotion-tags").unwrap(),
        "whisper"
    );
}

#[tokio::test]
async fn test_irodori_speech_missing_bracket_recovery() {
    let mock_server = MockServer::start().await;
    let dummy_wav = make_dummy_wav();

    // Verify that "chuckle] さあ..." is recovered to 🤭 emoji
    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .respond_with(move |req: &wiremock::Request| {
            let body_json: Value = serde_json::from_slice(&req.body).unwrap();
            let input_text = body_json["input"].as_str().unwrap();
            assert!(input_text.contains('🤭'), "Text must contain chuckle emoji");
            assert!(input_text.contains("さあケンジ君"));
            assert!(!input_text.contains("chuckle]"));

            ResponseTemplate::new(200)
                .set_body_bytes(dummy_wav.clone())
                .insert_header("content-type", "audio/wav")
        })
        .mount(&mock_server)
        .await;

    let config = create_irodori_test_config(mock_server.uri());
    let engine = create_engine(&config).unwrap();
    let state = Arc::new(AppState::with_engine(config, engine));
    let app = create_router(state);

    let speech_req = json!({
        "model": "irodori-tts",
        "input": "chuckle] さあケンジ君、やっとウィンターホールドに着いたわ。",
        "voice": "default"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&speech_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-emotion-tags").unwrap(),
        "chuckle"
    );
}
