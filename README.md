# 🦀 zonos2-openai-bridge

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-performance, ultra-low latency **OpenAI-compatible Text-to-Speech (TTS) proxy bridge** for **Zyphra Zonos 2**, written in Rust.

It enables SkyrimNet (Chatterbox client), `nina_agent`, `kitt_agent`, and standard OpenAI TTS clients (Open WebUI, AnythingLLM, etc.) to seamlessly use Zonos 2 with zero code changes.

---

## ⚡ Features

- **OpenAI Speech API Compatible**: Implements `POST /v1/audio/speech` and `GET /v1/models`.
- **Automatic Emotion Tag Parsing**: Detects tags like `[whisper]`, `[laughter]`, `[sigh]`, `[angry]`, `[happy]` (and Japanese brackets `【笑い】`) in input text, dynamically mapping them to Zonos 2's multi-dimensional emotion sliders and CFG scale.
- **Zero-Loss Ultra-Fast Audio Conversion**:
  - Converts Zonos 2's raw Float32 PCM into standard **16-bit signed integer RIFF WAVE** (`Format 1`, 44.1kHz Mono).
  - Nanosecond-level RIFF header packing with zero re-allocations and audio clipping protection.
- **GPU Concurrency Guard**: Built-in async mutex queue prevents GPU VRAM spikes or inference race conditions.
- **Single Self-Contained Binary**: Zero runtime dependencies (no Python/Node.js required to run).

---

## 🎭 Emotion Tag Mapping

Input text can include inline emotion tags. The bridge automatically extracts them, configures Zonos 2, cleans the spoken text, and attaches an `X-Emotion-Tags` header to the response:

| Tag | Detected Category | Sliders & Parameters |
| :--- | :--- | :--- |
| `[laughter]`, `[giggle]`, `[happy]`, `【笑い】` | Happy / Joy | `happy: 0.7`, `cfg: 1.5`, `speed: 1.0` |
| `[whisper]`, `[gentle]`, `[sweet]`, `【囁き】` | Whisper / Soft | `happy: 0.25`, `cfg: 1.3`, `speed: 0.88` |
| `[sigh]`, `[sad]`, `[sorrow]`, `【ため息】` | Sad / Melancholy | `sad: 0.7`, `cfg: 1.5`, `speed: 0.90` |
| `[angry]`, `[mad]`, `[shout]`, `【怒り】` | Anger / Irritated | `angry: 0.75`, `cfg: 1.6`, `speed: 1.05` |
| `[surprise]`, `[shock]`, `【驚き】` | Surprise | `surprised: 0.7`, `cfg: 1.5`, `speed: 1.0` |

---

## 🚀 Installation & Building (Linux / macOS)

### 1. Prerequisites (Rust Toolchain)

If Rust is not yet installed on your machine (e.g., on a fresh Linux server):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### 2. Clone & Build Release Binary

```bash
git clone https://github.com/kjsd/zonos2-openai-bridge.git
cd zonos2-openai-bridge
cargo build --release
```

The optimized binary will be produced at:
`target/release/zonos2-openai-bridge` (~8MB)

---

## ⚙️ Configuration

Options can be set via CLI flags or environment variables:

| Argument | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host` | `HOST` | `0.0.0.0` | Host address to bind bridge server to |
| `-p, --port` | `PORT` | `8000` | Port to listen on |
| `--zonos-url` | `ZONOS_URL` | `http://127.0.0.1:1919` | Base URL of Zonos 2 FastAPI server |
| `--default-voice` | `DEFAULT_VOICE` | `nina2` | Default speaker embedding name |
| `--default-model` | `DEFAULT_MODEL` | `zonos2` | Default model name |
| `--max-body-size-mb` | `MAX_BODY_SIZE_MB` | `100` | Max request body size in MB (for large base64 reference audio) |
| `--log-level` | `LOG_LEVEL` | `info` | Log verbosity (`info`, `debug`, `trace`) |

You can also create a `.env` file in the working directory:

```env
HOST=0.0.0.0
PORT=8000
ZONOS_URL=http://127.0.0.1:1919
DEFAULT_VOICE=nina2
MAX_BODY_SIZE_MB=100
LOG_LEVEL=info
```

---

## 🖥️ Running as a systemd Service (Production on Linux)

To run `zonos2-openai-bridge` as a persistent background daemon on Linux:

1. Create a systemd unit file at `/etc/systemd/system/zonos2-openai-bridge.service`:

```ini
[Unit]
Description=Zonos 2 OpenAI TTS Bridge
After=network.target

[Service]
Type=simple
User=xxx
WorkingDirectory=/xxx/zonos2-openai-bridge
ExecStart=/xxx/zonos2-openai-bridge/target/release/zonos2-openai-bridge
Restart=always
RestartSec=3
Environment=HOST=0.0.0.0
Environment=PORT=8080
Environment=ZONOS_URL=http://127.0.0.1:1919
Environment=DEFAULT_VOICE=nina2
Environment=LOG_LEVEL=info

[Install]
WantedBy=multi-user.target
```

2. Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now zonos2-openai-bridge
```

3. Check logs:

```bash
journalctl -u zonos2-openai-bridge -f
```

---

## 📡 API Usage & Client Integration

### 1. Standard cURL Request

```bash
curl -X POST http://localhost:8000/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{
    "model": "zonos2",
    "input": "[whisper] パパ、寒くない…？",
    "voice": "nina2",
    "response_format": "wav"
  }' \
  --output speech.wav
```

### 2. Python (Official OpenAI Client)

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://nina.local:8000/v1",
    api_key="none"
)

response = client.audio.speech.create(
    model="zonos2",
    voice="nina2",
    input="[laughter] えへへ、パパ大好き！"
)

response.stream_to_file("output.wav")
```

### 3. SkyrimNet (Chatterbox Client)

In SkyrimNet configuration:
- **TTS Engine**: `Chatterbox` (or `OpenAI Compatible`)
- **Server URL**: `http://nina.local:8000/v1/audio/speech`
- **Voice**: `nina2`
- **Format**: `wav`

### 4. Health Check & Models

```bash
# Check status and Zonos 2 connectivity
curl http://localhost:8000/health

# List supported models
curl http://localhost:8000/v1/models
```

---

## 🧪 Testing

Run all unit and integration tests:

```bash
cargo test
```

---

## 📜 License

MIT License. Authored by Kenji Minoura & Nina.
