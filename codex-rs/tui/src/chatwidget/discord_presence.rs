use codex_discord_presence::DiscordPresenceSnapshot;

use crate::bottom_pane::StatusLineItem;
use crate::status::format_directory_display;

use super::ChatWidget;

impl ChatWidget {
    pub(crate) fn discord_presence_snapshot(&mut self) -> Option<DiscordPresenceSnapshot> {
        self.thread_id?;

        let location = self.status_line_project_root_name().unwrap_or_else(|| {
            format_directory_display(self.status_line_cwd(), /*max_width*/ None)
        });
        let small_text = self
            .status_line_value_for_item(&StatusLineItem::ModelWithReasoning)
            .unwrap_or_else(|| self.model_display_name().to_string());
        let details = location;

        let used_tokens = self.status_line_value_for_item(&StatusLineItem::UsedTokens);
        let context = self
            .status_line_value_for_item(&StatusLineItem::ContextRemaining)
            .or_else(|| self.status_line_value_for_item(&StatusLineItem::ContextUsed));
        let state = match (used_tokens, context) {
            (Some(used_tokens), Some(context)) => Some(format!("{used_tokens} · {context}")),
            (Some(used_tokens), None) => Some(used_tokens),
            (None, Some(context)) => Some(context),
            (None, None) => None,
        };

        Some(DiscordPresenceSnapshot {
            details,
            state,
            small_text: Some(small_text),
        })
    }
}
