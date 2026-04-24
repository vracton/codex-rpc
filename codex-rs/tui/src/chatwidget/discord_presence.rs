use codex_discord_presence::DiscordPresenceSnapshot;

use crate::bottom_pane::StatusLineItem;
use crate::status::format_directory_display;
use crate::status::format_tokens_compact;

use super::ChatWidget;

const DEFAULT_MODEL_BADGE_IMAGE: &str = "gpt-54";
const GPT_55_MODEL_BADGE_IMAGE: &str = "gpt-55";

impl ChatWidget {
    pub(crate) fn discord_presence_snapshot(&mut self) -> Option<DiscordPresenceSnapshot> {
        self.thread_id?;

        let location = format_directory_display(self.status_line_cwd(), /*max_width*/ None);
        let small_text = self
            .status_line_value_for_item(&StatusLineItem::ModelWithReasoning)
            .unwrap_or_else(|| self.model_display_name().to_string());
        let details = format!("Working in {location}");

        let total_tokens = self.status_line_total_usage().tokens_in_context_window();
        let used_tokens = format!("{} tokens used", format_tokens_compact(total_tokens.max(0)));
        let context = self
            .status_line_context_remaining_percent()
            .map(|remaining| format!("{remaining}% context left"));
        let state = match (used_tokens, context) {
            (used_tokens, Some(context)) => Some(format!("{used_tokens} · {context}")),
            (used_tokens, None) => Some(used_tokens),
        };

        Some(DiscordPresenceSnapshot {
            details,
            state,
            small_image: Some(model_badge_image_for_text(&small_text).to_string()),
            small_text: Some(small_text),
        })
    }
}

fn model_badge_image_for_text(model_text: &str) -> &'static str {
    let model_text = model_text.to_ascii_lowercase();
    if model_text.contains("gpt-5.5") || model_text.contains("gpt 5.5") {
        GPT_55_MODEL_BADGE_IMAGE
    } else {
        DEFAULT_MODEL_BADGE_IMAGE
    }
}
