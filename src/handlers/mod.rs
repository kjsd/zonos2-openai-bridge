pub mod health;
pub mod models;
pub mod speech;

pub use health::handle_health;
pub use models::handle_models;
pub use speech::{handle_speech, AppState};
