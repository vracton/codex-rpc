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

        let state = self.pet_state();
        let completed_preview = self.last_agent_markdown.as_deref().and_then(|markdown| {
            markdown
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(std::borrow::ToOwned::to_owned)
        });
        let subtitle = if state == PetState::Review {
            completed_preview.or(Some(self.current_status.header.clone()))
        } else {
            Some(self.current_status.header.clone())
        };
        let detail = if state == PetState::Review {
            None
        } else {
            self.current_status.details.clone().or(Some(model))
        };

        PetsSnapshot {
            state,
            title: self
                .last_rendered_user_message_event
                .as_ref()
                .map(|event| event.message.trim())
                .filter(|message| !message.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .unwrap_or(title),
            subtitle,
            detail,
            notification_count: self.pet_notification_count(),
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

    fn pet_notification_count(&self) -> u32 {
        let count = self.queued_user_messages.len();
        if self.task_complete_pending || self.last_agent_markdown.is_some() {
            count.saturating_add(1).try_into().unwrap_or(u32::MAX)
        } else {
            count.try_into().unwrap_or(u32::MAX)
        }
    }
}
