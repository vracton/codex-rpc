//! Chat widget helpers for ambient terminal pets and the pets picker.

use super::*;
use codex_config::types::TuiPetAnchor;
use codex_pets::PetsSnapshot;
use codex_pets::protocol::PetState;

use crate::bottom_pane::StatusLineItem;
use crate::status::format_directory_display;

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
            .status_line_value_for_item(StatusLineItem::ModelWithReasoning)
            .unwrap_or_else(|| self.model_display_name().to_string());

        let state = self.desktop_pet_state();
        let completed_preview = self
            .transcript
            .last_agent_markdown
            .as_deref()
            .and_then(first_non_empty_line);
        let live_preview = self
            .transcript
            .live_agent_markdown
            .lines()
            .map(str::trim)
            .rev()
            .find(|line| !line.is_empty())
            .map(std::borrow::ToOwned::to_owned);
        let current_status = &self.status_state.current_status;
        let current_status_header = if state != PetState::Running
            && current_status.header.eq_ignore_ascii_case("working")
        {
            None
        } else {
            Some(current_status.header.clone())
        };
        let subtitle = if state == PetState::Running {
            live_preview.or(current_status_header)
        } else if state == PetState::Review {
            completed_preview
                .or(current_status_header)
                .or_else(|| Some("Ready".to_string()))
        } else if state == PetState::Idle {
            None
        } else {
            current_status_header
        };
        let detail = if matches!(
            state,
            PetState::Running | PetState::Waiting | PetState::Failed
        ) {
            current_status.details.clone().or(Some(model))
        } else {
            None
        };

        PetsSnapshot {
            state,
            title: self
                .last_rendered_user_message_display
                .as_ref()
                .map(|display| display.message.trim())
                .filter(|message| !message.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .unwrap_or(title),
            subtitle,
            detail,
            notification_count: self.desktop_pet_notification_count(),
        }
    }

    fn desktop_pet_state(&self) -> PetState {
        let current_status = &self.status_state.current_status;
        if current_status.is_guardian_review() || self.bottom_pane.terminal_title_requires_action()
        {
            PetState::Waiting
        } else if current_status.header.eq_ignore_ascii_case("failed")
            || current_status.header.to_ascii_lowercase().contains("error")
        {
            PetState::Failed
        } else if self.bottom_pane.is_task_running() || self.input_queue.user_turn_pending_start {
            PetState::Running
        } else if self.task_complete_pending || self.transcript.last_agent_markdown.is_some() {
            PetState::Review
        } else {
            PetState::Idle
        }
    }

    fn desktop_pet_notification_count(&self) -> u32 {
        let count = self.input_queue.queued_user_messages.len();
        if self.task_complete_pending || self.transcript.last_agent_markdown.is_some() {
            count.saturating_add(1).try_into().unwrap_or(u32::MAX)
        } else {
            count.try_into().unwrap_or(u32::MAX)
        }
    }
}

fn first_non_empty_line(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(std::borrow::ToOwned::to_owned)
}

pub(super) fn load_ambient_pet(
    config: &Config,
    frame_requester: FrameRequester,
) -> Option<crate::pets::AmbientPet> {
    let selected_pet = config.tui_pet.as_deref()?;
    if selected_pet == crate::pets::DISABLED_PET_ID {
        return None;
    }

    crate::pets::AmbientPet::load(
        Some(selected_pet),
        &config.codex_home,
        frame_requester,
        config.animations,
    )
    .ok()
}

pub(super) fn start_configured_pet_load_if_needed(
    config: &Config,
    ambient_pet_missing: bool,
    frame_requester: FrameRequester,
    app_event_tx: AppEventSender,
) {
    let Some(pet_id) = config.tui_pet.clone() else {
        return;
    };
    if pet_id == crate::pets::DISABLED_PET_ID || !ambient_pet_missing {
        return;
    }

    let codex_home = config.codex_home.clone();
    let animations_enabled = config.animations;
    spawn_pet_load(move || {
        let result = crate::pets::ensure_builtin_pack_for_pet(&pet_id, &codex_home)
            .and_then(|()| {
                crate::pets::AmbientPet::load(
                    Some(&pet_id),
                    &codex_home,
                    frame_requester,
                    animations_enabled,
                )
            })
            .map(Some)
            .map_err(|err| err.to_string());
        app_event_tx.send(AppEvent::ConfiguredPetLoaded { pet_id, result });
    });
}

impl ChatWidget {
    pub(super) fn set_ambient_pet_notification(
        &mut self,
        kind: crate::pets::PetNotificationKind,
        body: Option<String>,
    ) {
        if let Some(pet) = self.ambient_pet.as_mut() {
            pet.set_notification(kind, body);
        }
    }

    pub(crate) fn ambient_pet_image_enabled(&self) -> bool {
        self.ambient_pet
            .as_ref()
            .is_some_and(crate::pets::AmbientPet::image_enabled)
    }

    pub(crate) fn disable_ambient_pet_for_session(&mut self) {
        self.ambient_pet = None;
        self.request_redraw();
    }

    pub(crate) fn ambient_pet_draw(
        &self,
        area: Rect,
        composer_bottom_y: u16,
    ) -> Option<crate::pets::AmbientPetDraw> {
        if !self.bottom_pane.no_modal_or_popup_active() {
            return None;
        }

        let anchor_bottom_y = match self.config.tui_pet_anchor {
            TuiPetAnchor::Composer => composer_bottom_y,
            TuiPetAnchor::ScreenBottom => area.bottom(),
        };
        self.ambient_pet
            .as_ref()?
            .draw_request(area, anchor_bottom_y)
    }

    pub(super) fn ambient_pet_wrap_reserved_cols(&self) -> u16 {
        self.ambient_pet
            .as_ref()
            .filter(|pet| pet.image_enabled())
            .map(|pet| {
                pet.image_columns()
                    .saturating_add(AMBIENT_PET_WRAP_GAP_COLUMNS)
            })
            .unwrap_or(0)
    }

    pub(crate) fn history_wrap_width(&self, width: u16) -> u16 {
        width
            .saturating_sub(self.ambient_pet_wrap_reserved_cols())
            .max(1)
    }

    pub(crate) fn pet_picker_preview_draw(&self) -> Option<crate::pets::AmbientPetDraw> {
        self.bottom_pane
            .selected_index_for_active_view(crate::pets::PET_PICKER_VIEW_ID)?;
        let area = self.pet_picker_preview_state.area()?;
        let request = self
            .pet_picker_preview_pet
            .as_ref()?
            .preview_draw_request(area)?;
        self.pet_picker_preview_image_visible.set(true);
        Some(request)
    }

    pub(crate) fn should_clear_pet_picker_preview_image(&self) -> bool {
        self.pet_picker_preview_image_visible.replace(false)
    }

    pub(crate) fn fail_pet_picker_preview_render(&mut self, message: String) {
        self.pet_picker_preview_state.set_error(message);
        self.pet_picker_preview_pet = None;
        self.request_redraw();
    }

    pub(crate) fn open_pets_picker(&mut self) {
        if self.warn_if_pets_unsupported() {
            return;
        }

        self.pet_picker_preview_state.clear();
        self.pet_picker_preview_pet = None;
        let params = crate::pets::build_pet_picker_params(
            self.config.tui_pet.as_deref(),
            &self.config.codex_home,
            self.pet_picker_preview_state.clone(),
        );
        self.bottom_pane.show_selection_view(params);
        let initial_pet_id = self
            .config
            .tui_pet
            .as_deref()
            .unwrap_or(crate::pets::DEFAULT_PET_ID)
            .to_string();
        self.start_pet_picker_preview(initial_pet_id);
    }

    pub(crate) fn select_pet_by_id(&mut self, pet_id: String) {
        if self.warn_if_pets_unsupported() {
            return;
        }

        self.app_event_tx.send(AppEvent::PetSelected { pet_id });
    }

    fn warn_if_pets_unsupported(&mut self) -> bool {
        let support = self.pet_image_support();
        let Some(message) = support.unsupported_message() else {
            return false;
        };

        self.add_warning_message(message.to_string());
        true
    }

    fn pet_image_support(&self) -> crate::pets::PetImageSupport {
        #[cfg(test)]
        if let Some(support) = self.pet_image_support_override {
            return support;
        }

        #[cfg(test)]
        return crate::pets::PetImageSupport::Unsupported(
            crate::pets::PetImageUnsupportedReason::Terminal,
        );

        #[cfg(not(test))]
        crate::pets::detect_pet_image_support()
    }

    /// Set the pet preselected by the TUI picker in the widget's config copy.
    pub(crate) fn set_tui_pet(&mut self, pet: Option<String>) {
        self.config.tui_pet = pet;
        self.ambient_pet = load_ambient_pet(&self.config, self.frame_requester.clone());
        self.apply_ambient_pet_image_support_override_for_tests();
        self.request_redraw();
    }

    pub(crate) fn set_tui_pet_loaded(
        &mut self,
        pet: Option<String>,
        ambient_pet: Option<crate::pets::AmbientPet>,
    ) {
        self.config.tui_pet = pet;
        self.ambient_pet = ambient_pet;
        self.apply_ambient_pet_image_support_override_for_tests();
        self.request_redraw();
    }

    #[cfg(test)]
    fn apply_ambient_pet_image_support_override_for_tests(&mut self) {
        if let Some(support) = self.pet_image_support_override
            && let Some(pet) = self.ambient_pet.as_mut()
        {
            pet.set_image_support_for_tests(support);
        }
    }

    #[cfg(not(test))]
    fn apply_ambient_pet_image_support_override_for_tests(&mut self) {}

    pub(crate) fn start_pet_picker_preview(&mut self, pet_id: String) {
        self.pet_picker_preview_request_id =
            self.pet_picker_preview_request_id.wrapping_add(/*rhs*/ 1);
        let request_id = self.pet_picker_preview_request_id;
        self.pet_picker_preview_pet = None;
        if pet_id == crate::pets::DISABLED_PET_ID {
            self.pet_picker_preview_state.set_disabled();
            self.request_redraw();
            return;
        }

        self.pet_picker_preview_state.set_loading();
        self.request_redraw();

        let codex_home = self.config.codex_home.clone();
        let frame_requester = self.frame_requester.clone();
        let tx = self.app_event_tx.clone();
        spawn_pet_load(move || {
            let result = crate::pets::ensure_builtin_pack_for_pet(&pet_id, &codex_home)
                .and_then(|()| {
                    crate::pets::AmbientPet::load(
                        Some(&pet_id),
                        &codex_home,
                        frame_requester,
                        /*animations_enabled*/ false,
                    )
                })
                .map_err(|err| err.to_string());
            tx.send(AppEvent::PetPreviewLoaded { request_id, result });
        });
    }

    pub(crate) fn finish_pet_picker_preview_load(
        &mut self,
        request_id: u64,
        result: Result<crate::pets::AmbientPet, String>,
    ) {
        if request_id != self.pet_picker_preview_request_id {
            return;
        }

        match result {
            Ok(pet) => {
                self.pet_picker_preview_state.set_ready();
                self.pet_picker_preview_pet = Some(pet);
                #[cfg(test)]
                if let Some(support) = self.pet_image_support_override
                    && let Some(pet) = self.pet_picker_preview_pet.as_mut()
                {
                    pet.set_image_support_for_tests(support);
                }
            }
            Err(message) => {
                self.pet_picker_preview_state.set_error(message);
                self.pet_picker_preview_pet = None;
            }
        }
        self.request_redraw();
    }

    pub(crate) fn show_pet_selection_loading_popup(&mut self) -> u64 {
        self.pet_selection_load_request_id =
            self.pet_selection_load_request_id.wrapping_add(/*rhs*/ 1);
        self.pet_picker_preview_state.clear();
        self.pet_picker_preview_pet = None;
        self.bottom_pane.show_selection_view(SelectionViewParams {
            view_id: Some(PET_SELECTION_LOADING_VIEW_ID),
            title: Some("Loading Pet".to_string()),
            subtitle: Some("Preparing the terminal pet.".to_string()),
            items: vec![SelectionItem {
                name: "Loading selected pet...".to_string(),
                is_disabled: true,
                ..Default::default()
            }],
            ..Default::default()
        });
        self.pet_selection_load_request_id
    }

    pub(crate) fn finish_pet_selection_loading_popup(&mut self, request_id: u64) -> bool {
        if request_id != self.pet_selection_load_request_id {
            return false;
        }
        self.bottom_pane
            .dismiss_active_view_if_id(PET_SELECTION_LOADING_VIEW_ID);
        true
    }

    #[cfg(test)]
    pub(crate) fn set_pet_image_support_for_tests(
        &mut self,
        support: crate::pets::PetImageSupport,
    ) {
        self.pet_image_support_override = Some(support);
        self.apply_ambient_pet_image_support_override_for_tests();
    }

    #[cfg(test)]
    pub(crate) fn install_test_ambient_pet_for_tests(&mut self, animations_enabled: bool) {
        self.set_tui_pet_loaded(
            Some("test".to_string()),
            Some(crate::pets::test_ambient_pet(
                self.frame_requester.clone(),
                animations_enabled,
            )),
        );
    }
}

fn spawn_pet_load(f: impl FnOnce() + Send + 'static) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        std::mem::drop(handle.spawn_blocking(f));
    } else {
        let _ = std::thread::spawn(f);
    }
}
