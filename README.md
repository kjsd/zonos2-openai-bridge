# 🦀 zonos2-openai-bridge

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-performance, ultra-low latency **OpenAI-compatible & Gradio 4 (Chatterbox) TTS proxy bridge** for **Zyphra Zonos 2**, written in Rust.

It seamlessly bridges standard **OpenAI TTS clients** (Open WebUI, AnythingLLM, Elixir clients, etc.) and **SkyrimNet** (Chatterbox / Gradio client) with Zonos 2 with zero code changes, providing dynamic emotion parsing, speech pacing control, and high-speed audio format conversion.

---

## ⚡ Features

- **Dual API Support**:
  - **OpenAI Speech API**: Implements `POST /v1/audio/speech` and `GET /v1/models`.
  - **Gradio 4 (Chatterbox) API**: Emulates Gradio 4 endpoints used by SkyrimNet Chatterbox (`POST /gradio_api/upload`, `POST /gradio_api/call/generate_audio`, SSE event stream, and `/gradio_api/file={path}`).
  - **XTTS / SkyrimNet Routes**: Provides `/speakers`, `/speakers_list`, and `/languages` compatibility.
- **Full SkyrimNet & Chatterbox Tag Support**:
  - Automatically captures and strips all emotion and sound tags (including multi-word tags like `[clear throat]` and `[dramatic tone]`).
  - Converts emotion tags into Zonos 2 multidimensional emotion vectors and CFG scales.
  - Safely eliminates sound/action tags from spoken text so characters never speak tags aloud.
- **Dynamic Speaking-Rate Conditioning**:
  - Fixes Zonos 2's default sluggish Japanese cadence by enabling native speaking-rate conditioning (`DEFAULT_SPEED=1.15`).
  - Dynamically adjusts pacing per emotion (e.g. `[slowly]` for relaxed pacing, `[angry]` for fast delivery, `[whisper]` for deliberate soft whisper).
- **Zero-Loss Ultra-Fast Audio Conversion**:
  - Converts Zonos 2's raw 32-bit float PCM into standard **16-bit signed integer RIFF WAVE** (`Format 1`, 44.1kHz Mono).
  - Nanosecond-level RIFF header packing with zero re-allocations and audio clipping protection.
- **Concurrency & Resource Safety**:
  - Built-in async mutex queue prevents GPU VRAM spikes and inference collisions on the Zonos 2 backend.
  - Enforces request body size limits (`MAX_BODY_SIZE_MB`, default 100MB) for handling large Base64 speaker audio uploads.
- **Single Self-Contained Binary**: Zero runtime dependencies (no Python or Node.js required to run the bridge).

---

## 🎭 Supported Emotion & Sound Tags

The bridge parses inline tags in square brackets `[...]` or Japanese brackets `【...】`. Tags are mapped to Zonos 2 conditioning parameters, and stripped from the spoken text.

### 1. Emotion Tags (Expression & Pacing)

| Tag / Variants | Category | Emotion Sliders & Parameters | Pacing (`final_speed`) |
| :--- | :--- | :--- | :--- |
| `[happy]`, `[laughter]`, `[laugh]`, `【笑い】` | Joy / Happy | `happy: 0.7`, `cfg: 1.2` | 1.15x (Normal) |
| `[chuckle]`, `[giggle]`, `【くすくす】` | Chuckle | `happy: 0.5`, `cfg: 1.15` | 1.15x (Normal) |
| `[excited]`, `【興奮】`, `【わくわく】` | Excited | `happy: 0.8`, `cfg: 1.2` | 1.26x (Fast) |
| `[whisper]`, `[whispering]`, `[softly]`, `【囁き】` | Whisper | `happy: 0.2`, `cfg: 1.1` | 1.01x (Deliberate) |
| `[sigh]`, `[sad]`, `[crying]`, `【ため息】`, `【悲しい】` | Sad / Sorrow | `sad: 0.7`, `cfg: 1.2` | 1.03x (Heavy / Slow) |
| `[groan]`, `【うめき】` | Groan / Pain | `sad: 0.5`, `angry: 0.3`, `cfg: 1.15` | 1.03x (Heavy / Slow) |
| `[sniff]`, `【鼻をすする】` | Sniffle | `sad: 0.5`, `cfg: 1.15` | 1.05x |
| `[angry]`, `[shout]`, `[yell]`, `【怒り】`, `【叫び】` | Anger / Shout | `angry: 0.75`, `cfg: 1.2` | 1.32x (Intense / Fast) |
| `[surprised]`, `[shock]`, `【驚き】` | Surprise | `surprised: 0.7`, `cfg: 1.2` | 1.15x (Normal) |
| `[gasp]`, `【息をのむ】` | Gasp | `surprised: 0.6`, `fear: 0.3`, `cfg: 1.15` | 1.15x (Normal) |
| `[fear]`, `[scared]`, `【恐怖】` | Fear | `fear: 0.7`, `cfg: 1.2` | 1.15x (Normal) |
| `[disgust]`, `【嫌悪】` | Disgust | `disgust: 0.7`, `cfg: 1.2` | 1.15x (Normal) |
| `[sarcastic]`, `【皮肉】` | Sarcasm | `angry: 0.3`, `sad: 0.2`, `cfg: 1.15` | 1.09x |
| `[dramatic]`, `[dramatic tone]` | Dramatic | `surprised: 0.3`, `sad: 0.2`, `cfg: 1.2` | 1.03x (Ponderous) |
| `[advertisement]`, `[commercial]` | Upbeat | `happy: 0.4`, `cfg: 1.15` | 1.21x (Brisk) |
| `[narration]`, `[narrator]` | Narration | Neutral | 1.15x (Balanced) |
| **`[slowly]`** | **Slow Pacing** | Neutral | **0.86x (Relaxed / Deliberate)** |

### 2. Sound / Action Tags (Cleaned from Spoken Text)

- **`[clear throat]` / `[clears throat]`**: Throat clearing (safely removed without being spoken).
- **`[cough]` / `[coughs]`**: Coughing sounds.
- **`[shush]` / `[shh]`**: Shushing sounds (mapped to gentle whisper pacing).
- **`[pause]`**: Brief silence marker.

---

## ⚙️ Configuration

Options can be set via CLI flags or environment variables:

| Argument | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host` | `HOST` | `0.0.0.0` | Host address to bind bridge server to |
| `-p, --port` | `PORT` | `8080` | Port to listen on |
| `--zonos-url` | `ZONOS_URL` | `http://127.0.0.1:1919` | Base URL of Zonos 2 FastAPI server |
| `--default-voice` | `DEFAULT_VOICE` | `default` | Fallback speaker embedding voice name |
| `--default-model` | `DEFAULT_MODEL` | `zonos2` | Model name returned in `/v1/models` |
| `--default-speed` | `DEFAULT_SPEED` | `1.15` | Base speech speed multiplier for Zonos 2 |
| `--max-body-size-mb` | `MAX_BODY_SIZE_MB` | `100` | Max request body size in MB (for speaker audio) |
| `--log-level` | `LOG_LEVEL` | `info` | Log verbosity (`info`, `debug`, `trace`) |

---

## 🚀 Installation & Building

### 1. Build Release Binary

```bash
git clone https://github.com/kjsd/zonos2-openai-bridge.git
cd zonos2-openai-bridge
cargo build --release
```

The optimized binary will be produced at:
`target/release/zonos2-openai-bridge` (~8MB)

### 2. Run as a systemd Service (Linux)

Create `/etc/systemd/system/zonos2-openai-bridge.service`:

```ini
[Unit]
Description=Zonos 2 OpenAI TTS Bridge
After=network.target

[Service]
Type=simple
User=your_user
WorkingDirectory=/opt/zonos2-openai-bridge
ExecStart=/opt/zonos2-openai-bridge/target/release/zonos2-openai-bridge
Restart=always
RestartSec=3
Environment=HOST=0.0.0.0
Environment=PORT=8080
Environment=ZONOS_URL=http://127.0.0.1:1919
Environment=DEFAULT_VOICE=default
Environment=DEFAULT_SPEED=1.15
Environment=LOG_LEVEL=info

[Install]
WantedBy=multi-user.target
```

Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now zonos2-openai-bridge
```

---

## 📡 Client Setup & Integration

### 1. SkyrimNet (Chatterbox Client)

In your SkyrimNet Mod configuration:
- **TTS Engine**: `Chatterbox` (or `Gradio 4`)
- **Server Address**: `http://<your-server-ip>:8080` (e.g. `http://127.0.0.1:8080`)
- Character reference voices uploaded by SkyrimNet via Gradio will be automatically processed, calibrated, and synthesised.

### 2. Standard OpenAI TTS Client (e.g., Python / curl)

```bash
curl -X POST http://localhost:8080/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{
    "model": "zonos2",
    "input": "[happy] [clear throat] Welcome to Skyrim! [chuckle]",
    "voice": "default",
    "response_format": "wav"
  }' \
  --output speech.wav
```

### 3. Health & Diagnostics

```bash
# Check status and Zonos 2 connectivity
curl http://localhost:8080/health

# List supported models
curl http://localhost:8080/v1/models
```

---

## 🧪 Running Tests

```bash
cargo test
```

All 20 unit and integration tests (including tag stripping, Gradio upload/generate flow, and OpenAI endpoints) will be verified.

---

## 📜 License

MIT License.
