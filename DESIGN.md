# 🦀 zonos2-openai-bridge: 要件定義 & アーキテクチャ設計書

## 1. 概要 (Overview)
**`zonos2-openai-bridge`** は、RTX 5090 上で稼働する次世代音声合成エンジン **Zonos 2**（FastAPI: `http://localhost:1919`）の前段に配置される、**Rust 製の超軽量・超低レイテンシな OpenAI 互換 Text-to-Speech (TTS) プロキシサーバー** です。

SkyrimNet（Chatterbox クライアント）、`nina_agent`、`kitt_agent`、その他サードパーティ製ツール（Open WebUI 等）に対し、標準の `POST /v1/audio/speech` API を提供します。
さらに、テキスト内に埋め込まれた各種感情タグ（`[whisper]`, `[laughter]`, `[sigh]` 等）をリアルタイムに自動抽出し、Zonos 2 の多次元感情パラメータ（スライダー / CFG）へ動的にマッピングします。

```mermaid
flowchart LR
    subgraph Clients [Clients (Standard OpenAI TTS)]
        Skyrim[SkyrimNet<br/>Chatterbox]
        NinaAgent[nina_agent<br/>Web / Pocket]
        Kitt[kitt_agent<br/>mBot2 / App]
        Other[Open WebUI / etc]
    end

    subgraph Bridge [zonos2-openai-bridge (Rust)]
        Router[Axum Router<br/>POST /v1/audio/speech]
        Parser[Emotion Parser<br/>Tag extraction & Mapping]
        Lock[Concurrency Guard<br/>GPU Lock / Queue]
        Converter[Audio Converter<br/>Float32 -> Int16 WAV]
    end

    subgraph Backend [Zonos 2 Engine (RTX 5090)]
        Zonos[FastAPI Server<br/>Port 1919<br/>MoE 16-Experts]
    end

    Skyrim -->|POST /v1/audio/speech| Router
    NinaAgent -->|POST /v1/audio/speech| Router
    Kitt -->|POST /v1/audio/speech| Router
    Other -->|POST /v1/audio/speech| Router

    Router --> Parser
    Parser -->|Cleaned Text + Sliders| Lock
    Lock -->|POST /tts/generate| Zonos
    Zonos -->|Float32 PCM (44.1kHz)| Converter
    Converter -->|16-bit PCM WAV / MP3| Router
    Router -->|Binary Audio| Clients
```

---

## 2. 要件定義 (Requirements)

### 機能要件 (Functional Requirements)

1. **REQ-1: OpenAI 互換 TTS エンドポイントの実装**
   - **URI**: `POST /v1/audio/speech`
   - **Request Payload (JSON)**:
     ```json
     {
       "model": "zonos2",
       "input": "[whisper] パパ、寒くない…？",
       "voice": "nina2",
       "response_format": "wav",
       "speed": 1.0
     }
     ```
   - **Response**: 音声バイナリ（デフォルト: `audio/wav`、フォーマット: Format 1 PCM 16-bit 44.1kHz Mono）
   - **REQ-1.1**: `GET /v1/models` でモデル一覧（`zonos2` 等）を返却。
   - **REQ-1.2**: `GET /health` でブリッジおよび Zonos 2 バックエンドの生存確認。

2. **REQ-2: 感情タグ自動パース & Zonos 2 パラメータ変換**
   - 入力テキストから `[...]` 形式の感情タグを正規表現で検出・抽出。
   - 音声合成用テキストからタグを除去（クリーン化）。
   - タグに基づき、Zonos 2 の以下のパラメータを動的生成：
     - `emotion_sliders`: `happy`, `sad`, `angry`, `surprised`, `disgust`, `fear` 等
     - `emotion_cfg_scale`: 感情タグが存在する場合は強調（例: 1.5 〜 2.0）
     - `speed` / `speaking_rate`: `[whisper]`（0.85）や `[fast]`（1.2）など速度への影響
     - `accurate_mode`: 音質優先フラグ
   - **対応タグ例**:
     - 喜び/笑い: `[laughter]`, `[giggle]`, `[laugh]`, `[happy]`, `[joy]` -> `happy: 0.7`, `cfg: 1.5`
     - 囁き/甘え: `[whisper]`, `[gentle]`, `[sweet]` -> `speed: 0.85`, `happy: 0.2`, `cfg: 1.2`
     - 溜息/悲哀: `[sigh]`, `[sad]`, `[sorrow]` -> `sad: 0.7`, `speed: 0.9`, `cfg: 1.5`
     - 怒り/叫び: `[angry]`, `[shout]`, `[mad]` -> `angry: 0.7`, `cfg: 1.6`
     - 驚き: `[surprise]`, `[shock]` -> `surprised: 0.7`, `cfg: 1.5`

3. **REQ-3: 超高速・ゼロロスレス音声変換 (Float32 -> Int16 WAV)**
   - Zonos 2 の出力（Format 3, Float32 LE, 44.1kHz）をバイトストリームで受信。
   - IEEE 754 Float32 (-1.0 〜 1.0) を Int16 (-32768 〜 32767) へクランプ・スケーリング変換。
   - 標準 RIFF WAVE (Format 1, 16-bit, 44.1kHz, Mono) ヘッダ（44バイト）を先頭に付与。
   - 組み込み開発の流儀に則り、SIMD / ゼロアロケーションを意識した極小レイテンシ（< 2ms）処理。

4. **REQ-4: 単一推論ガード (Concurrency Guard)**
   - Zonos 2 の同時実行による VRAM 溢れや競合を防ぐため、`tokio::sync::Mutex` または `Semaphore` による 1 リクエスト単位の直列化キューイング。

5. **REQ-5: 設定の柔軟性 (Configurable)**
   - 環境変数または CLI オプションで設定可能：
     - バインドポート（デフォルト: `8000`）
     - Zonos 2 API URL（デフォルト: `http://127.0.0.1:1919`）
     - デフォルト話者（デフォルト: `nina2`）
     - ログレベル（デフォルト: `info`）

---

## 3. 非機能要件 (Non-Functional Requirements)

1. **極小オーバーヘッド**:
   - リクエスト受信から Zonos 2 送信、および音声変換からレスポンス返却までのブリッジオーバーヘッドを **5ms 未満** とする。
2. **完全ロスレス音質**:
   - 16-bit 整数変換においてダイナミックレンジを損なわない線形スケーリングを実施（クリッピング保護付き）。
3. **シングルバイナリ・フットプリント**:
   - 外部ランタイム（Python, Node 等）不要。単一のネイティブバイナリ（Linux x86_64 / macOS aarch64）として即時稼働可能。

---

## 4. 技術選定 (Rust Crates)

| クレート | 用途 / 選定理由 |
| :--- | :--- |
| **`axum`** (v0.8) | 高速・非同期 Web フレームワーク。Tokio 陣営の公式標準。 |
| **`tokio`** (v1) | マルチスレッド非同期ランタイム（macros, rt-multi-thread）。 |
| **`reqwest`** (v0.12) | 非同期 HTTP クライアント。接続プール再利用で高速通信。 |
| **`serde` / `serde_json`** | 型安全な JSON シリアライズ / デシリアライズ。 |
| **`clap`** (v4, derive) | 堅牢で美しい CLI 引数パーサー。 |
| **`tracing` / `tracing-subscriber`** | 構造化ロギング。 |
| **`regex`** | 高速な感情タグ抽出・テキストクリーニング。 |
| **`byteorder`** | RIFF WAV ヘッダおよび PCM バイト操作（リトルエンディアン）。 |

---

## 5. ディレクトリ・モジュール構成 (Project Layout)

```text
zonos2-openai-bridge/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              # エントリポイント (CLI パース、Tokio 起動)
│   ├── config.rs            # 設定管理 (ポート、Zonos URL、デフォルトボイス)
│   ├── server.rs            # Axum Router & サーバー起動ロジック
│   ├── error.rs             # 統合エラー型 (AppError -> HTTP レスポンス変換)
│   ├── parser/
│   │   ├── mod.rs
│   │   └── emotion.rs       # 感情タグ正規表現パース & Zonos2 パラメータ生成
│   ├── zonos/
│   │   ├── mod.rs
│   │   ├── client.rs        # Zonos2 /tts/generate 呼び出しクライアント
│   │   └── types.rs         # Zonos2 API の Request/Response 構造体
│   ├── audio/
│   │   ├── mod.rs
│   │   └── wav.rs           # Float32 -> Int16 PCM 変換 & RIFF ヘッダ構築
│   └── handlers/
│       ├── mod.rs
│       ├── speech.rs        # POST /v1/audio/speech
│       ├── models.rs        # GET /v1/models
│       └── health.rs        # GET /health
└── tests/
    ├── emotion_test.rs      # タグパースの網羅テスト
    ├── audio_test.rs        # PCM変換・WAVヘッダ検証テスト
    └── integration_test.rs  # モックを使ったエンドツーエンドテスト
```

---

## 6. テスト・検証方針 (Testing Strategy)

1. **単体テスト (Unit Tests on darkstar)**:
   - **タグ解析テスト**: `[whisper]`, `[laughter]`, 複合タグ、全角括弧（`【笑い】`等）のフォールバック動作、テキストクリーニングの完全性。
   - **PCM/WAV 変換テスト**: Float32 配列を与えて Int16 WAV が生成され、サンプリングレート 44100Hz、1ch、16-bit の正しい RIFF ヘッダ（44バイト）が含まれるかをバイナリ検査。
2. **統合テスト (Integration Tests on darkstar)**:
   - WireMock 等、またはローカルのダミーサーバーを用意し、`POST /v1/audio/speech` の送受信を E2E でテスト。
3. **実機デプロイ検証 (on nina.local)**:
   - 単体テスト合格後、`nina.local` へ転送またはビルドし、実稼働中の Zonos 2 (`:1919`) と結合テスト。
   - SkyrimNet (Chatterbox) および `nina_agent` からのリクエスト受送信テスト。
