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
    #[serde(default)]
    pub notification_count: u32,
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
                notification_count: 0,
            },
        };

        let encoded = serde_json::to_string(&command).expect("serialize command");
        let decoded: HelperCommand = serde_json::from_str(&encoded).expect("deserialize command");

        assert_eq!(decoded, command);
    }

    #[test]
    fn snapshot_defaults_notification_count() {
        let decoded: HelperCommand = serde_json::from_str(
            r#"{"type":"set_snapshot","snapshot":{"pet":"codex","state":"idle","title":"Codex","subtitle":null,"detail":null}}"#,
        )
        .expect("deserialize command");

        assert_eq!(
            decoded,
            HelperCommand::SetSnapshot {
                snapshot: HelperSnapshot {
                    pet: "codex".to_string(),
                    state: PetState::Idle,
                    title: "Codex".to_string(),
                    subtitle: None,
                    detail: None,
                    notification_count: 0,
                },
            }
        );
    }
}
