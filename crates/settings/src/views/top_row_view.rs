//! Top row function keys (F1–F12) and Fn trigger key configuration view.
//!
//! Complies with F2.1, F2.2, F1.1, F1.4, and docs/04-ux.md.

use egui::Ui;
use magicbridge_core::{Config, FnKeyChoice, TopRowMode};

use crate::i18n::I18n;

/// State for the Top Row & Fn tab.
#[derive(Debug, Clone, Default)]
pub struct TopRowViewState {
    pub custom_hotkey_text: String,
}

/// Renders the Top Row & Fn settings view.
pub fn show(ui: &mut Ui, _state: &mut TopRowViewState, config: &mut Config, i18n: &I18n) -> bool {
    let mut modified = false;

    ui.group(|ui| {
        ui.heading(i18n.t("top_row.title"));
        ui.add_space(4.0);

        if let Some(profile) = config.active_profile_mut() {
            ui.label(i18n.t("top_row.mode_label"));

            // Media keys first radio button
            let mut mode = profile.top_row_mode;
            ui.vertical(|ui| {
                if ui
                    .radio_value(
                        &mut mode,
                        TopRowMode::MediaKeysFirst,
                        i18n.t("top_row.media_first"),
                    )
                    .changed()
                {
                    profile.top_row_mode = mode;
                    modified = true;
                }
                ui.indent("media_first_indent", |ui| {
                    ui.label(i18n.t("top_row.media_first_desc"));
                });

                ui.add_space(6.0);

                // F-keys first radio button
                if ui
                    .radio_value(
                        &mut mode,
                        TopRowMode::FKeysFirst,
                        i18n.t("top_row.fkeys_first"),
                    )
                    .changed()
                {
                    profile.top_row_mode = mode;
                    modified = true;
                }
                ui.indent("fkeys_first_indent", |ui| {
                    ui.label(i18n.t("top_row.fkeys_first_desc"));
                });
            });

            ui.separator();

            // Fn Trigger Key Selector (F1.1, F1.4)
            ui.heading(i18n.t("top_row.fn_key_title"));
            ui.label(i18n.t("top_row.fn_key_desc"));
            ui.add_space(4.0);

            let mut fn_key = profile.fn_key;
            let current_fn_label = match fn_key {
                FnKeyChoice::RightOption => i18n.t("top_row.fn_right_option"),
                FnKeyChoice::RightControl => i18n.t("top_row.fn_right_control"),
                FnKeyChoice::F19 => i18n.t("top_row.fn_f19"),
                FnKeyChoice::Disabled => i18n.t("top_row.fn_disabled"),
                FnKeyChoice::Other(_) => i18n.t("top_row.fn_other"),
            };

            let combo = egui::ComboBox::from_id_salt("fn_key_combo")
                .selected_text(current_fn_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut fn_key,
                        FnKeyChoice::RightOption,
                        i18n.t("top_row.fn_right_option"),
                    );
                    ui.selectable_value(
                        &mut fn_key,
                        FnKeyChoice::RightControl,
                        i18n.t("top_row.fn_right_control"),
                    );
                    ui.selectable_value(&mut fn_key, FnKeyChoice::F19, i18n.t("top_row.fn_f19"));
                    ui.selectable_value(
                        &mut fn_key,
                        FnKeyChoice::Disabled,
                        i18n.t("top_row.fn_disabled"),
                    );
                });
            let combo_id = combo.response.id;
            combo
                .response
                .on_hover_text(i18n.t("top_row.fn_assignment"));
            crate::set_accessible_name(ui, combo_id, i18n.t("top_row.fn_assignment"));

            if fn_key != profile.fn_key {
                profile.fn_key = fn_key;
                modified = true;
            }

            ui.add_space(6.0);
            ui.label(i18n.t("top_row.f3_f4_note"));
        }
    });

    modified
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_row_view_state_defaults() {
        let state = TopRowViewState::default();
        assert!(state.custom_hotkey_text.is_empty());
    }
}
