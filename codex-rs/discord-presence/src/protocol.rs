use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperCommand {
    SetPresence {
        details: String,
        state: Option<String>,
        small_image: Option<String>,
        small_text: Option<String>,
        start_timestamp_seconds: u64,
    },
    ClearPresence,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperEvent {
    Ready,
    Error { message: String },
}
