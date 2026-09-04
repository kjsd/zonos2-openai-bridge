use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use byteorder::{LittleEndian, WriteBytesExt};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zonos2_openai_bridge::{create_router, AppState, Config, ZonosClient};

fn create_test_config(zonos_url: String) -> Config {
    Config {
        host: "127.0.0.1".to_string(),
        port: 8000,
        zonos_url,
        default_voice: "nina2".to_string(),
        default_model: "zonos2".to_string(),
        log_level: "error".to_string(),
        max_body_size_mb: 100,
    }
}

#[tokio::test]
async fn test_models_endpoint() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
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
    assert_eq!(json["object"], "list");
    let models = json["data"].as_array().unwrap();
    assert!(models.iter().any(|m| m["id"] == "zonos2"));
    assert!(models.iter().any(|m| m["id"] == "tts-1"));
}

#[tokio::test]
async fn test_speech_empty_input_bad_request() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let req_body = json!({
        "input": "   ",
        "voice": "nina2"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_speech_e2e_with_emotion_and_wav_conversion() {
    // 1. Start mock Zonos 2 server
    let mock_zonos = MockServer::start().await;

    // Prepare 4 samples of float32 PCM (0.0, 0.5, -0.5, 1.0)
    let mut mock_float_pcm = Vec::new();
    mock_float_pcm.write_f32::<LittleEndian>(0.0).unwrap();
    mock_float_pcm.write_f32::<LittleEndian>(0.5).unwrap();
    mock_float_pcm.write_f32::<LittleEndian>(-0.5).unwrap();
    mock_float_pcm.write_f32::<LittleEndian>(1.0).unwrap();

    // Mock /tts/generate endpoint
    Mock::given(method("POST"))
        .and(path("/tts/generate"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "audio/pcm")
                .set_body_bytes(mock_float_pcm),
        )
        .mount(&mock_zonos)
        .await;

    let config = create_test_config(mock_zonos.uri());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    // 2. Send OpenAI speech request with [whisper] emotion tag
    let req_body = json!({
        "model": "zonos2",
        "input": "[whisper] パパ、寒くない？",
        "voice": "nina2",
        "response_format": "wav",
        "speed": 1.0
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
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

    // 3. Verify returned WAV binary
    let body = response.into_body().collect().await.unwrap().to_bytes();
    // 44 bytes header + 4 samples * 2 bytes = 52 bytes
    assert_eq!(body.len(), 52);

    assert_eq!(&body[0..4], b"RIFF");
    assert_eq!(&body[8..12], b"WAVE");
    assert_eq!(&body[12..16], b"fmt ");
    assert_eq!(&body[36..40], b"data");

    // Format code: 1 (PCM Integer)
    let format_code = u16::from_le_bytes([body[20], body[21]]);
    assert_eq!(format_code, 1);

    // Sample rate: 44100
    let sample_rate = u32::from_le_bytes([body[24], body[25], body[26], body[27]]);
    assert_eq!(sample_rate, 44100);

    // Verify converted sample values:
    let s0 = i16::from_le_bytes([body[44], body[45]]);
    let s1 = i16::from_le_bytes([body[46], body[47]]);
    let s2 = i16::from_le_bytes([body[48], body[49]]);
    let s3 = i16::from_le_bytes([body[50], body[51]]);

    assert_eq!(s0, 0);
    assert!((s1 - 16384).abs() <= 1);
    assert!((s2 - -16384).abs() <= 1);
    assert_eq!(s3, 32767);
}

#[tokio::test]
async fn test_speech_with_custom_speaker_audio_base64() {
    let mock_server = MockServer::start().await;

    // Verify mock server receives speaker_audio_base64 and no speaker_embedding_name
    let mut fake_pcm = Vec::new();
    fake_pcm.write_f32::<LittleEndian>(0.5).unwrap();

    Mock::given(method("POST"))
        .and(path("/tts/generate"))
        .and(wiremock::matchers::body_partial_json(serde_json::json!({
            "speaker_audio_base64": "UklGRiQAAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAAAAA="
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_pcm))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let req_body = json!({
        "model": "zonos2",
        "input": "カスタム話者で喋るよ",
        "speaker_audio_base64": "UklGRiQAAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAAAAA="
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_speech_input_too_long() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let long_text = "あ".repeat(4097);
    let req_body = json!({
        "model": "zonos2",
        "input": long_text
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audio/speech")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_speakers_endpoint() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/speakers")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let speakers: Vec<String> = serde_json::from_slice(&body).unwrap();
    assert!(speakers.contains(&"nina2".to_string()));
}

#[tokio::test]
async fn test_languages_endpoint() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/languages")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let langs: Vec<String> = serde_json::from_slice(&body).unwrap();
    assert!(langs.contains(&"ja".to_string()));
    assert!(langs.contains(&"en".to_string()));
}

#[tokio::test]
async fn test_unmapped_fallback_404() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/some/random/missing/route")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_gradio_upload_endpoint() {
    let config = create_test_config("http://127.0.0.1:9999".to_string());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    // Build multipart body
    let boundary = "------------------------boundary123";
    let body_bytes = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"test_voice.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFFfakebytes\r\n--{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/gradio_api/upload")
                .method("POST")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body_bytes))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let paths: Vec<String> = serde_json::from_slice(&body).unwrap();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with(".wav"));
    assert!(std::path::Path::new(&paths[0]).exists());
}

#[tokio::test]
async fn test_gradio_generate_and_file_flow() {
    let mock_server = MockServer::start().await;

    let mut fake_pcm = Vec::new();
    fake_pcm.write_f32::<LittleEndian>(0.2).unwrap();

    Mock::given(method("POST"))
        .and(path("/tts/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_pcm))
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let zonos = ZonosClient::new(config.zonos_url.clone());
    let state = Arc::new(AppState::new(config, zonos));
    let app = create_router(state);

    // 1. Call generate_audio
    let req_body = json!({
        "data": [
            null,
            "[whisper] こんにちは、ドヴァキン",
            null,
            null
        ]
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/gradio_api/call/generate_audio")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res_json: Value = serde_json::from_slice(&body).unwrap();
    let event_id = res_json["event_id"].as_str().unwrap();
    assert!(event_id.starts_with("event_"));

    // Wait a little for background generation to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Check generated file path
    let out_file_path = format!("/tmp/zonos_gradio_voices/out_{event_id}.wav");
    assert!(std::path::Path::new(&out_file_path).exists());

    // 2. Fetch the file via /gradio_api/file=...
    let file_uri = format!("/gradio_api/file={out_file_path}");
    let response = app
        .oneshot(
            Request::builder()
                .uri(&file_uri)
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "audio/wav"
    );
}


