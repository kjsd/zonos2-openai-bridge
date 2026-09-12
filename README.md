# 🦀 openai-tts-bridge

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-performance, modular **OpenAI-compatible & Gradio 4 (Chatterbox) TTS proxy bridge** written in Rust.

It seamlessly bridges standard **OpenAI TTS clients** (Open WebUI, AnythingLLM, Elixir clients, etc.) and **SkyrimNet** (Chatterbox / Gradio client) with state-of-the-art TTS engines using a plugin-based architecture.

Supported engines:
- **Zyphra Zonos 2**: High-dimensional emotion vectors, speaking-rate conditioning, and ultra-fast 32-bit float to 16-bit PCM WAV conversion.
- **Aratako Irodori-TTS**: Expressive Japanese TTS controlled via inline Unicode style emojis (`👂`, `😮‍💨`, `🤭`, `😊`, `😡`, `😭`, `😱`, etc.).

---

## ⚡ Features

- **Modular Plugin Engine Architecture**:
  - Switch TTS backends seamlessly via environment variable (`TTS_ENGINE=zonos2` or `TTS_ENGINE=irodori`).
  - Extensible `TtsEngine` trait allows adding new speech synthesis engines easily.
- **Dual API Compatibility**:
  - **OpenAI Speech API**: Implements `POST /v1/audio/speech` and `GET /v1/models`.
  - **Gradio 4 (Chatterbox) API**: Emulates Gradio 4 endpoints used by SkyrimNet Chatterbox (`POST /gradio_api/upload`, `POST /gradio_api/call/generate_audio`, SSE event stream, and `/gradio_api/file={path}`).
  - **XTTS / SkyrimNet Routes**: Provides `/speakers`, `/speakers_list`, and `/languages` endpoints.
- **Smart Emotion & Audio Tag Transformation**:
  - Automatically captures SkyrimNet / Chatterbox tags (e.g. `[happy]`, `[whisper]`, `[clear throat]`, `(sigh)`, `*giggle*`).
  - Resilient parser rescues SkyrimNet sentence-boundary anomalies (e.g. `chuckle] Hello` or `[whisper Hello`).
  - **Engine-specific conversion**:
    - **Zonos 2**: Converts tags into multidimensional emotion slider vectors & CFG scales; safely strips tags from speakable text.
    - **Irodori-TTS**: Transforms tags directly into native Unicode style control emojis inline; cleans non-voiced sound tags.
- **Dynamic Speaking-Rate Conditioning**:
  - Calibrates pacing dynamically based on emotion and configuration (`DEFAULT_SPEED=1.15` for Zonos2, customizable per request).
- **Zero-Loss Ultra-Fast Audio Conversion**:
  - Converts raw 32-bit float PCM into standard **16-bit signed integer RIFF WAVE** (`Format 1`, 44.1kHz Mono) with clipping protection.
- **Concurrency & Resource Safety**:
  - Built-in async mutex queue serializes backend inference requests, preventing GPU VRAM spikes and collisions.
  - Enforces body size limits (`MAX_BODY_SIZE_MB`, default 100MB) for reference audio uploads.
- **Self-Contained Binary**: Zero runtime dependencies (no Python or Node.js required to run the bridge).

---

## 🎭 Emotion & Audio Tag Mapping Matrix

The bridge parses inline tags in brackets `[...]`, `【...】`, `(...)`, or `*...*`.

| Tag / Syntax | Expression | Zonos 2 Mapping | Irodori-TTS Emoji |
| :--- | :--- | :--- | :--- |
| `[whisper]`, `[softly]`, `【囁き】` | Whisper / Gentle | `happy: 0.2`, `cfg: 1.1`, speed 1.01x | `👂` |
| `[sigh]`, `[sad]`, `【ため息】` | Sigh / Sorrow | `sad: 0.7`, `cfg: 1.2`, speed 1.03x | `😮‍💨` |
| `[chuckle]`, `[giggle]`, `【くすくす】` | Chuckle / Laugh | `happy: 0.5`, `cfg: 1.15`, speed 1.15x | `🤭` |
| `[happy]`, `[laughter]`, `【笑い】` | Joy / Happy | `happy: 0.7`, `cfg: 1.2`, speed 1.15x | `😊` |
| `[angry]`, `[shout]`, `【怒り】` | Anger / Shout | `angry: 0.75`, `cfg: 1.2`, speed 1.32x | `😡` |
| `[crying]`, `[weep]`, `【泣き】` | Crying / Weeping | `sad: 0.8`, `cfg: 1.25`, speed 1.00x | `😭` |
| `[fear]`, `[scared]`, `【恐怖】` | Fear | `fear: 0.7`, `cfg: 1.2`, speed 1.15x | `😱` |
| `[surprised]`, `[shock]`, `【驚き】` | Surprise | `surprised: 0.7`, `cfg: 1.2`, speed 1.15x | `😲` |
| `[gasp]`, `【息をのむ】` | Gasp | `surprised: 0.6`, `fear: 0.3`, speed 1.15x | `😲` |
| `[sniff]`, `【鼻をすする】` | Sniffle | `sad: 0.5`, `cfg: 1.15`, speed 1.05x | `🤧` |
| `[groan]`, `【うめき】` | Groan / Pain | `sad: 0.5`, `angry: 0.3`, speed 1.03x | `😩` |
| `[yawn]`, `【あくび】` | Yawn | Neutral | `🥱` |
| `[pause]` | Silence marker | Strip tag | `⏸️` |
| `[slowly]`, `【ゆっくり】` | Slow pacing | Speed 0.86x | `🐢` |
| `[fast]`, `【早口】` | Fast pacing | Speed 1.26x | `⏩` |
| `[excited]`, `【興奮】` | Excited | `happy: 0.8`, `cfg: 1.2`, speed 1.26x | `✨` |
| `[sarcastic]`, `【皮肉】` | Sarcasm | `angry: 0.3`, `sad: 0.2`, speed 1.09x | `😏` |
| `[dramatic]`, `[dramatic tone]` | Dramatic | `surprised: 0.3`, `sad: 0.2`, speed 1.03x | `🎭` |
| `[clear throat]`, `[cough]` | Sound tags | Stripped from spoken audio | Stripped |

---

## ⚙️ Configuration

Options can be configured via command-line arguments or environment variables:

| Argument | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--tts-engine` | `TTS_ENGINE` | `zonos2` | Engine to use: `zonos2` or `irodori` |
| `--host` | `HOST` | `0.0.0.0` | Host address to bind bridge server to |
| `-p, --port` | `PORT` | `8080` | Port to listen on |
| `--zonos-url` | `ZONOS_URL` | `http://127.0.0.1:1919` | Base URL of Zonos 2 FastAPI server |
| `--irodori-url` | `IRODORI_URL` | `http://127.0.0.1:8088` | Base URL of Irodori-TTS server |
| `--irodori-model` | `IRODORI_MODEL` | `irodori-tts` | Model identifier for Irodori-TTS |
| `--default-voice` | `DEFAULT_VOICE` | `default` | Fallback speaker identifier |
| `--default-model` | `DEFAULT_MODEL` | `zonos2` | Fallback model name |
| `--default-speed` | `DEFAULT_SPEED` | `1.15` | Base speech speed multiplier for Zonos 2 |
| `--max-body-size-mb` | `MAX_BODY_SIZE_MB` | `100` | Max request body size in MB |
| `--log-level` | `LOG_LEVEL` | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |

---

## 🚀 Quick Start

### 1. Build from Source

```bash
git clone git@github.com:kjsd/openai-tts-bridge.git
cd openai-tts-bridge
cargo build --release
```

The optimized binary will be produced at `target/release/openai-tts-bridge` (~8MB).

### 2. Run with Zonos 2 Backend

```bash
TTS_ENGINE=zonos2 \
ZONOS_URL=http://127.0.0.1:1919 \
DEFAULT_VOICE=default \
./target/release/openai-tts-bridge
```

### 3. Run with Irodori-TTS Backend

```bash
TTS_ENGINE=irodori \
IRODORI_URL=http://127.0.0.1:8088 \
DEFAULT_VOICE=default \
./target/release/openai-tts-bridge
```

---

## 🖥️ Running as a systemd Service (Linux)

To run `openai-tts-bridge` as a background daemon on a Linux server:

1. Create a service file at `/etc/systemd/system/openai-tts-bridge.service`:

```ini
[Unit]
Description=OpenAI & Gradio TTS Bridge Server
After=network.target

[Service]
Type=simple
User=minoru
WorkingDirectory=/home/minoru/work/openai-tts-bridge
ExecStart=/home/minoru/work/openai-tts-bridge/target/release/openai-tts-bridge
Restart=always
RestartSec=3
Environment=HOST=0.0.0.0
Environment=PORT=8080
Environment=TTS_ENGINE=zonos2
Environment=ZONOS_URL=http://127.0.0.1:1919
Environment=IRODORI_URL=http://127.0.0.1:8088
Environment=DEFAULT_VOICE=default
Environment=DEFAULT_SPEED=1.15
Environment=LOG_LEVEL=info

[Install]
WantedBy=multi-user.target
```

2. Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now openai-tts-bridge
```

3. Check logs:

```bash
journalctl -u openai-tts-bridge -f
```

---

## 📡 Client Setup & Integration

### 1. SkyrimNet (Chatterbox Client)

In SkyrimNet configuration:
- **TTS Engine**: `Chatterbox` (or `Gradio 4`)
- **Server Address**: `http://<server-ip>:8080`
- Character reference voices uploaded by SkyrimNet via Gradio will be automatically processed, converted, and synthesised.

### 2. Standard OpenAI Client (cURL)

```bash
curl -X POST http://localhost:8080/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{
    "model": "zonos2",
    "input": "[whisper] パパ、寒くない？ [sigh]",
    "voice": "default",
    "response_format": "wav"
  }' \
  --output speech.wav
```

### 3. Health & Models API

```bash
# Health check (includes backend engine status)
curl http://localhost:8080/health

# List available models
curl http://localhost:8080/v1/models
```

---

## 🧪 Testing

Run all unit tests and mock integration tests:

```bash
cargo test
```

Includes comprehensive tests for:
- Float32 to PCM16 WAV audio conversion and header validation.
- Emotion parser and malformed tag recovery (missing brackets, punctuation split anomalies).
- Irodori emoji mapper conversions and tag sanitization.
- Full end-to-end OpenAI API speech synthesis with mocked backends.
- Gradio 4 upload, background audio generation, and file streaming flows.

---

## 📄 License

MIT License.
