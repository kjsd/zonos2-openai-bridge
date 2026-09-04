# 🦀 zonos2-openai-bridge

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-performance, ultra-low latency **OpenAI-compatible Text-to-Speech (TTS) proxy bridge** for **Zyphra Zonos 2**, written in Rust.

It allows SkyrimNet (Chatterbox client), `nina_agent`, `kitt_agent`, and standard OpenAI TTS clients (Open WebUI, AnythingLLM, etc.) to seamlessly use Zonos 2 with zero code changes.

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

## 🚀 Quick Start

### 1. Build

```bash
cargo build --release
```

### 2. Run

```bash
# Default connects to Zonos 2 on http://127.0.0.1:1919 and listens on port 8000
./target/release/zonos2-openai-bridge
```

### Configuration Options

Options can be passed via CLI arguments or environment variables:

| Argument | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--host` | `HOST` | `0.0.0.0` | Host to bind bridge server to |
| `-p, --port` | `PORT` | `8000` | Port to listen on |
| `--zonos-url` | `ZONOS_URL` | `http://127.0.0.1:1919` | Base URL of Zonos 2 FastAPI server |
| `--default-voice` | `DEFAULT_VOICE` | `nina2` | Default speaker embedding name |
| `--default-model` | `DEFAULT_MODEL` | `zonos2` | Default model name |
| `--log-level` | `LOG_LEVEL` | `info` | Log verbosity (`info`, `debug`, `trace`) |

---

## 📡 API Usage

### Standard OpenAI Request (cURL)

```bash
curl -X POST http://localhost:8000/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{
    "model": "zonos2",
    "input": "[whisper] パパ、寒くない…？",
    "voice": "nina2",
    "response_format": "wav"
  }' \
  --output output.wav
```

### Python (OpenAI Official Client)

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="none"  # Not required for local bridge
)

response = client.audio.speech.create(
    model="zonos2",
    voice="nina2",
    input="[laughter] えへへ、パパ大好き！"
)

response.stream_to_file("output.wav")
```

### SkyrimNet (Chatterbox Integration)
In SkyrimNet's TTS configuration:
1. **TTS Provider**: `Chatterbox` (or `OpenAI Compatible`)
2. **Endpoint**: `http://nina.local:8000/v1/audio/speech`
3. **Voice**: `nina2`
4. **Format**: `wav`

---

## 🧪 Testing

Run all unit tests and mock integration tests:

```bash
cargo test
```

---

## 📜 License

MIT License. Authored by Kenji Minoura & Nina.
