//! Main eframe application shell for MagicBridge settings GUI.
//!
//! Complies with F7.1, F7.5, NFR-P5, and docs/04-ux.md.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::App;
use egui::{Color32, Context, RichText, TopBottomPanel};
use magicbridge_agent::DaemonStatus;
use magicbridge_core::{Config, atomic_write, default_config_path};

use crate::i18n::I18n;
use crate::ipc::SettingsIpc;
use crate::views::architecture_view::ArchitectureViewState;
use crate::views::general_view::GeneralViewState;
use crate::views::keyboard_view::{self, ConfigChange, KeyboardViewState};
use crate::views::qa_view::{self, HardwareQaViewState};
use crate::views::top_row_view::{self, TopRowViewState};
use crate::views::wizard_view::{self, OnboardingWizardState};
use crate::views::{about_view, battery_view, general_view};

/// Compact mode breakpoint width in points.
pub const COMPACT_MODE_BREAKPOINT: f32 = 850.0;

/// Returns true if the available width triggers compact navigation mode (<850px).
#[must_use]
pub fn is_compact_mode(width: f32) -> bool {
    width < COMPACT_MODE_BREAKPOINT
}

/// Navigation tabs in the settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Keyboard,
    TopRow,
    Battery,
    General,
    Wizard,
    HardwareQa,
    Architecture,
    About,
}

/// The main settings application state.
pub struct MagicBridgeApp {
    pub current_tab: SettingsTab,
    pub config: Config,
    pub config_path: PathBuf,
    pub i18n: I18n,
    pub ipc: SettingsIpc,
    pub daemon_status: Option<DaemonStatus>,
    pub last_poll: Instant,
    pub banner_message: Option<(bool, String)>, // (is_error, message)
    pub keyboard_state: KeyboardViewState,
    pub top_row_state: TopRowViewState,
    pub general_state: GeneralViewState,
    pub architecture_state: ArchitectureViewState,
    pub wizard_state: OnboardingWizardState,
    pub qa_state: HardwareQaViewState,
    pub tab_pulse: Option<(SettingsTab, Instant)>,
}

impl MagicBridgeApp {
    /// Creates and initializes the settings application state.
    #[must_use]
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_path = default_config_path().unwrap_or_else(|_| PathBuf::from("config.toml"));
        let config = Config::load_from_path(&config_path).unwrap_or_default();
        let i18n = I18n::new(&config.ui_language);
        let ipc = SettingsIpc::new();
        let daemon_status = ipc.query_status();
        let general_state = GeneralViewState::new_from_config(&config);
        let wizard_state = OnboardingWizardState::new_from_config(&config);
        let qa_state = HardwareQaViewState::default();

        Self {
            current_tab: SettingsTab::Keyboard,
            config,
            config_path,
            i18n,
            ipc,
            daemon_status,
            last_poll: Instant::now(),
            banner_message: None,
            keyboard_state: KeyboardViewState::default(),
            top_row_state: TopRowViewState::default(),
            general_state,
            architecture_state: ArchitectureViewState::default(),
            wizard_state,
            qa_state,
            tab_pulse: None,
        }
    }

    /// Saves the current configuration to disk and notifies the background agent via IPC.
    pub fn persist_and_notify(&self) {
        if let Ok(toml_str) = self.config.to_toml_string() {
            let _ = atomic_write(&self.config_path, toml_str);
            let _ = self.ipc.reload_config();
        }
    }

    /// Switches the active profile and notifies the background agent.
    pub fn switch_profile_and_notify(&mut self, profile_name: &str) {
        if let Ok(()) = self.config.set_active_profile(profile_name) {
            self.persist_and_notify();
            let _ = self.ipc.switch_profile(profile_name);
        }
    }
}

impl App for MagicBridgeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Poll daemon status every 2 seconds
        if self.last_poll.elapsed() >= Duration::from_secs(2) {
            self.daemon_status = self.ipc.query_status();
            self.last_poll = Instant::now();
        }

        // 1. "Agent Not Running" Warning Banner (F7.1)
        if self.daemon_status.is_none() {
            TopBottomPanel::top("agent_warning_banner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("⚠ {}", self.i18n.t("banner.agent_not_running")))
                            .color(Color32::from_rgb(200, 140, 0))
                            .strong(),
                    );
                    ui.add_space(8.0);
                    if ui.button(self.i18n.t("banner.start_agent")).clicked() {
                        match SettingsIpc::launch_agent() {
                            Ok(()) => {
                                self.daemon_status = self.ipc.query_status();
                                self.banner_message =
                                    Some((false, self.i18n.t("banner.agent_started").to_string()));
                            }
                            Err(e) => {
                                self.banner_message = Some((
                                    true,
                                    self.i18n.t_format("banner.start_failed", &[("error", &e)]),
                                ));
                            }
                        }
                    }
                    if let Some((is_error, ref msg)) = self.banner_message {
                        let color = if is_error {
                            Color32::from_rgb(220, 50, 40)
                        } else {
                            Color32::from_rgb(40, 160, 60)
                        };
                        ui.colored_label(color, msg);
                    }
                });
            });
        }

        // 2. Top Navigation Tabs Bar (F7.1, Issue #14: Adaptive Responsive Tabs)
        TopBottomPanel::top("tabs_bar").show(ctx, |ui| {
            let available_width = ui.available_width();
            let is_compact = is_compact_mode(available_width);

            let tabs = [
                (SettingsTab::Keyboard, "⌨", "tab.keyboard"),
                (SettingsTab::TopRow, "🎛", "tab.top_row"),
                (SettingsTab::Battery, "🔋", "tab.battery"),
                (SettingsTab::General, "⚙", "tab.general"),
                (SettingsTab::Wizard, "🪄", "tab.wizard"),
                (SettingsTab::HardwareQa, "🧪", "tab.qa"),
                (SettingsTab::Architecture, "🤖", "tab.architecture"),
                (SettingsTab::About, "ℹ", "tab.about"),
            ];

            if is_compact {
                // Compact Mode (<850px): icon-only buttons with tooltips in a horizontal scrollable rail
                egui::ScrollArea::horizontal()
                    .id_salt("tabs_scroll_area")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            for (tab, icon, key) in tabs {
                                let is_active = self.current_tab == tab;
                                let is_pulsing =
                                    self.tab_pulse.as_ref().is_some_and(|(t, inst)| {
                                        *t == tab && inst.elapsed().as_millis() < 300
                                    });
                                let tooltip = self.i18n.t(key);
                                let text = if is_pulsing {
                                    RichText::new(icon)
                                        .size(16.0)
                                        .strong()
                                        .color(Color32::from_rgb(0, 120, 215))
                                } else if is_active {
                                    RichText::new(icon).size(16.0).strong()
                                } else {
                                    RichText::new(icon).size(16.0)
                                };

                                let btn = egui::Button::new(text).selected(is_active);
                                let res = ui.add_enabled(!is_active, btn).on_hover_text(tooltip);
                                crate::set_accessible_name(ui, res.id, tooltip);
                                if res.clicked() {
                                    self.tab_pulse = Some((tab, Instant::now()));
                                    self.current_tab = tab;
                                    println!("[UI] Tab switched ok: {:?}", tab);
                                    let _ = std::io::stdout().flush();
                                    ctx.request_repaint();
                                }
                            }

                            // Show active tab title at the end of the compact rail
                            ui.separator();
                            let active_title = match self.current_tab {
                                SettingsTab::Keyboard => self.i18n.t("tab.keyboard"),
                                SettingsTab::TopRow => self.i18n.t("tab.top_row"),
                                SettingsTab::Battery => self.i18n.t("tab.battery"),
                                SettingsTab::General => self.i18n.t("tab.general"),
                                SettingsTab::Wizard => self.i18n.t("tab.wizard"),
                                SettingsTab::HardwareQa => self.i18n.t("tab.qa"),
                                SettingsTab::Architecture => self.i18n.t("tab.architecture"),
                                SettingsTab::About => self.i18n.t("tab.about"),
                            };
                            ui.label(RichText::new(active_title).strong());
                        });
                    });
            } else {
                // Full Mode (>=850px): full icon + label tabs
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    for (tab, icon, key) in tabs {
                        let label = format!("{} {}", icon, self.i18n.t(key));
                        let is_active = self.current_tab == tab;
                        let is_pulsing = self
                            .tab_pulse
                            .as_ref()
                            .is_some_and(|(t, inst)| *t == tab && inst.elapsed().as_millis() < 300);
                        let text = if is_pulsing {
                            RichText::new(&label)
                                .strong()
                                .color(Color32::from_rgb(0, 120, 215))
                        } else if is_active {
                            RichText::new(&label).strong()
                        } else {
                            RichText::new(&label)
                        };

                        let tooltip = self.i18n.t(key);
                        let btn = egui::Button::new(text).selected(is_active);
                        let res = ui.add_enabled(!is_active, btn).on_hover_text(tooltip);
                        crate::set_accessible_name(ui, res.id, tooltip);
                        if res.clicked() {
                            self.tab_pulse = Some((tab, Instant::now()));
                            self.current_tab = tab;
                            println!("[UI] Tab switched ok: {:?}", tab);
                            let _ = std::io::stdout().flush();
                            ctx.request_repaint();
                        }
                    }
                });
            }
        });

        // 3. Central Content Panel
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.current_tab {
                    SettingsTab::Keyboard => {
                        if let Some(change) = keyboard_view::show(
                            ui,
                            &mut self.keyboard_state,
                            &mut self.config,
                            &self.i18n,
                        ) {
                            match change {
                                ConfigChange::Modified => {
                                    self.persist_and_notify();
                                }
                                ConfigChange::ActiveProfileChanged(name) => {
                                    self.switch_profile_and_notify(&name);
                                }
                                ConfigChange::ProfileCreated(name) => {
                                    self.switch_profile_and_notify(&name);
                                }
                                ConfigChange::ProfileDeleted(_) => {
                                    self.persist_and_notify();
                                }
                            }
                        }
                    }
                    SettingsTab::TopRow => {
                        if top_row_view::show(
                            ui,
                            &mut self.top_row_state,
                            &mut self.config,
                            &self.i18n,
                        ) {
                            self.persist_and_notify();
                        }
                    }
                    SettingsTab::Battery => {
                        if battery_view::show(
                            ui,
                            self.daemon_status.as_ref(),
                            &mut self.config,
                            &self.i18n,
                        ) {
                            self.persist_and_notify();
                        }
                    }
                    SettingsTab::General => {
                        if general_view::show(
                            ui,
                            &mut self.general_state,
                            &mut self.config,
                            &mut self.i18n,
                        ) {
                            self.persist_and_notify();
                        }
                    }
                    SettingsTab::Wizard => {
                        let daemon_status = self.daemon_status.as_ref();
                        let action = wizard_view::show(
                            ui,
                            &mut self.wizard_state,
                            &mut self.config,
                            daemon_status,
                            &self.i18n,
                        );
                        if action == wizard_view::WizardAction::Finished {
                            self.persist_and_notify();
                            self.current_tab = SettingsTab::Keyboard;
                        }
                    }
                    SettingsTab::HardwareQa => {
                        qa_view::show(ui, &mut self.qa_state, &self.i18n);
                    }
                    SettingsTab::Architecture => {
                        self.architecture_state.ui(ui, &self.i18n);
                    }
                    SettingsTab::About => {
                        about_view::show(ui, self.daemon_status.as_ref(), &self.i18n);
                    }
                });
        });

        // Request a repaint periodically to update battery/connection status
        ctx.request_repaint_after(Duration::from_millis(1500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_mode_threshold() {
        assert!(is_compact_mode(849.0));
        assert!(is_compact_mode(500.0));
        assert!(is_compact_mode(0.0));
        assert!(!is_compact_mode(850.0));
        assert!(!is_compact_mode(1200.0));
        assert!(!is_compact_mode(3840.0));
    }
}
