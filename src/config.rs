use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "zonos2-openai-bridge",
    about = "High-performance OpenAI-compatible TTS bridge for Zonos 2",
    version
)]
pub struct Config {
    /// Host to bind the bridge server to
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value_t = 8000)]
    pub port: u16,

    /// Base URL of the Zonos 2 FastAPI server
    #[arg(long, env = "ZONOS_URL", default_value = "http://127.0.0.1:1919")]
    pub zonos_url: String,

    /// Default speaker embedding voice name
    #[arg(long, env = "DEFAULT_VOICE", default_value = "default")]
    pub default_voice: String,

    /// Default model name returned in model list
    #[arg(long, env = "DEFAULT_MODEL", default_value = "zonos2")]
    pub default_model: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Maximum request body size in megabytes (for large base64 speaker audio)
    #[arg(long, env = "MAX_BODY_SIZE_MB", default_value_t = 100)]
    pub max_body_size_mb: usize,

    /// Default speech speed factor (1.15 is natural tempo for Japanese in Zonos 2)
    #[arg(long, env = "DEFAULT_SPEED", default_value_t = 1.15)]
    pub default_speed: f32,
}
