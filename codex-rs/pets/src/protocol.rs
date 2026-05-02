use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PetState {
    Idle,
    Running,
    Waiting,
    Review,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperCommand {
    Show {
        pet: String,
        terminal_window_hint: Option<String>,
    },
    Hide,
    SetSnapshot {
        snapshot: HelperSnapshot,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperSnapshot {
    pub pet: String,
    pub state: PetState,
    pub title: String,
    pub subtitle: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperEvent {
    Ready,
    Hidden,
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn command_round_trips() {
        let command = HelperCommand::SetSnapshot {
            snapshot: HelperSnapshot {
                pet: "codex".to_string(),
                state: PetState::Running,
                title: "rpc-codex".to_string(),
                subtitle: Some("Thinking".to_string()),
                detail: Some("Inspecting files".to_string()),
            },
        };

        let encoded = serde_json::to_string(&command).expect("serialize command");
        let decoded: HelperCommand = serde_json::from_str(&encoded).expect("deserialize command");

        assert_eq!(decoded, command);
    }
}
