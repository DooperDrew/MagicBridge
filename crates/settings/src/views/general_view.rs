//! General application settings, autostart, and configuration import/export view.
//!
//! Complies with F7.3, F7.6, F8.1, F9.1, F9.2, and docs/04-ux.md.

use std::fs;
use std::path::PathBuf;

use egui::Ui;
use magicbridge_agent::autostart;
use magicbridge_core::{Config, NamedProfile};

use crate::i18n::I18n;
use crate::ipc::SettingsIpc;

/// State for the General tab.
#[derive(Debug, Clone, Default)]
pub struct GeneralViewState {
    pub pause_hotkey_text: String,
    pub game_processes_text: String,
    pub status_message: Option<String>,
}

/// Renders the General settings view.
pub fn show(
    ui: &mut Ui,
    state: &mut GeneralViewState,
    config: &mut Config,
    i18n: &mut I18n,
) -> bool {
    let mut modified = false;

    ui.group(|ui| {
        ui.heading(i18n.t("general.title"));
        ui.add_space(4.0);

        // 1. UI Language Selector (F7.3)
        ui.horizontal(|ui| {
            ui.label(i18n.t("general.language"));
            let current_lang = i18n.current_language();
            let mut selected_lang = current_lang.to_string();

            egui::ComboBox::from_id_salt("ui_language_combo")
                .selected_text(if current_lang == "it" {
                    "Italiano"
                } else {
                    "English"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_lang, "en".to_string(), "English");
                    ui.selectable_value(&mut selected_lang, "it".to_string(), "Italiano");
                });

            if selected_lang != config.ui_language {
                config.ui_language = selected_lang.clone();
                i18n.set_language(&selected_lang);
                modified = true;
            }
        });

        ui.separator();

        // 2. Autostart with Windows (F8.1)
        let autostart_active = autostart::is_autostart_enabled().unwrap_or(false);
        let mut autostart_toggle = autostart_active;
        if ui
            .checkbox(&mut autostart_toggle, i18n.t("general.autostart"))
            .changed()
        {
            if autostart_toggle {
                if let Some(agent_exe) = SettingsIpc::find_agent_exe() {
                    let _ = autostart::enable_autostart(&agent_exe);
                }
            } else {
                let _ = autostart::disable_autostart();
            }
            config.autostart = autostart_toggle;
            modified = true;
        }

        ui.separator();

        // 3. Global Pause Hotkey (F9.1)
        ui.horizontal(|ui| {
            ui.label(i18n.t("general.pause_hotkey"));
            let mut hotkey_str = config.pause_hotkey.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut hotkey_str).changed() {
                config.pause_hotkey = if hotkey_str.trim().is_empty() {
                    None
                } else {
                    Some(hotkey_str.trim().to_string())
                };
                modified = true;
            }
        });

        ui.separator();

        // 4. Game Mode & Monitored Executables (F9.2)
        ui.horizontal(|ui| {
            ui.label(i18n.t("general.game_processes"));
            ui.text_edit_singleline(&mut state.game_processes_text);
        });
        ui.indent("game_desc", |ui| {
            ui.label(i18n.t("general.game_mode_desc"));
        });

        ui.separator();

        // 5. Debug Logging (NFR-Q2)
        if ui
            .checkbox(&mut config.debug_logging, i18n.t("general.debug_logging"))
            .changed()
        {
            modified = true;
        }

        ui.separator();

        // 6. Configuration Import & Export (F7.6)
        ui.heading(i18n.t("general.config_management"));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Export Config
            if ui.button(i18n.t("general.export_config")).clicked() {
                let default_export_path = PathBuf::from("MagicBridge_Config_Export.toml");
                if let Ok(toml_str) = config.to_toml_string()
                    && let Ok(()) = fs::write(&default_export_path, toml_str)
                {
                    state.status_message = Some(i18n.t_format(
                        "general.config_exported",
                        &[("path", &default_export_path.display().to_string())],
                    ));
                }
            }

            // Import Config
            if ui.button(i18n.t("general.import_config")).clicked() {
                let default_import_path = PathBuf::from("MagicBridge_Config_Export.toml");
                if default_import_path.is_file()
                    && let Ok(contents) = fs::read_to_string(&default_import_path)
                    && let Ok(imported_config) = Config::from_toml_str(&contents)
                {
                    *config = imported_config;
                    modified = true;
                    state.status_message = Some(i18n.t("general.config_imported").to_string());
                }
            }

            // Export Active Profile
            if ui.button(i18n.t("general.export_profile")).clicked()
                && let Some(profile) = config.active_profile()
            {
                let filename = format!("Profile_{}.toml", profile.name.replace(' ', "_"));
                let path = PathBuf::from(&filename);
                if let Ok(toml_str) = toml::to_string_pretty(profile)
                    && let Ok(()) = fs::write(&path, toml_str)
                {
                    state.status_message = Some(i18n.t_format(
                        "general.config_exported",
                        &[("path", &path.display().to_string())],
                    ));
                }
            }

            // Import Profile
            if ui.button(i18n.t("general.import_profile")).clicked()
                && let Some(profile) = config.active_profile()
            {
                let filename = format!("Profile_{}.toml", profile.name.replace(' ', "_"));
                let path = PathBuf::from(&filename);
                if path.is_file()
                    && let Ok(contents) = fs::read_to_string(&path)
                    && let Ok(imported_prof) = toml::from_str::<NamedProfile>(&contents)
                {
                    config.add_profile(imported_prof);
                    modified = true;
                    state.status_message = Some(i18n.t("general.config_imported").to_string());
                }
            }
        });

        if let Some(ref msg) = state.status_message {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_rgb(40, 160, 60), msg);
        }
    });

    modified
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_view_state_defaults() {
        let state = GeneralViewState::default();
        assert!(state.pause_hotkey_text.is_empty());
        assert!(state.game_processes_text.is_empty());
        assert!(state.status_message.is_none());
    }
}
