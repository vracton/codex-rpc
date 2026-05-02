use codex_pets::PetsSnapshot;
use codex_pets::protocol::PetState;

use crate::bottom_pane::StatusLineItem;
use crate::status::format_directory_display;

use super::ChatWidget;

impl ChatWidget {
    pub(crate) fn pets_snapshot(&mut self) -> PetsSnapshot {
        let location = format_directory_display(self.status_line_cwd(), /*max_width*/ None);
        let title = self
            .thread_name
            .as_ref()
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Working in {location}"));
        let model = self
            .status_line_value_for_item(&StatusLineItem::ModelWithReasoning)
            .unwrap_or_else(|| self.model_display_name().to_string());

        PetsSnapshot {
            state: self.pet_state(),
            title,
            subtitle: Some(self.current_status.header.clone()),
            detail: self.current_status.details.clone().or(Some(model)),
        }
    }

    fn pet_state(&self) -> PetState {
        if self.current_status.is_guardian_review()
            || self.bottom_pane.terminal_title_requires_action()
        {
            PetState::Waiting
        } else if self.current_status.header.eq_ignore_ascii_case("failed")
            || self
                .current_status
                .header
                .to_ascii_lowercase()
                .contains("error")
        {
            PetState::Failed
        } else if self.bottom_pane.is_task_running() || self.user_turn_pending_start {
            PetState::Running
        } else if self.task_complete_pending || self.last_agent_markdown.is_some() {
            PetState::Review
        } else {
            PetState::Idle
        }
    }
}
