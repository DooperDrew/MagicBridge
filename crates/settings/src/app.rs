//! Main eframe application shell for MagicBridge settings GUI.
//!
//! Complies with F7.1, F7.5, NFR-P5, and docs/04-ux.md.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::App;
use egui::{Color32, Context, RichText, TopBottomPanel};
use magicbridge_agent::DaemonStatus;
use magicbridge_core::{Config, atomic_write, default_config_path};

use crate::i18n::I18n;
use crate::ipc::SettingsIpc;
use crate::views::general_view::GeneralViewState;
use crate::views::keyboard_view::{self, ConfigChange, KeyboardViewState};
use crate::views::top_row_view::{self, TopRowViewState};
use crate::views::{about_view, battery_view, general_view};

/// Navigation tabs in the settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Keyboard,
    TopRow,
    Battery,
    General,
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
            general_state: GeneralViewState::default(),
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

    /// Polls the daemon status if the poll interval has elapsed.
    fn check_daemon_status(&mut self) {
        if self.last_poll.elapsed() >= Duration::from_millis(1500) {
            self.daemon_status = self.ipc.query_status();
            self.last_poll = Instant::now();
        }
    }
}

impl App for MagicBridgeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.check_daemon_status();

        // 1. "Agent Not Running" Top Banner (F7.1)
        let agent_running = self.daemon_status.as_ref().is_some_and(|s| s.agent_running);

        if !agent_running {
            TopBottomPanel::top("offline_banner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(220, 60, 40),
                        format!("⚠ {}", self.i18n.t("banner.agent_not_running")),
                    );
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

        // 2. Top Navigation Tabs Bar
        TopBottomPanel::top("tabs_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);

                let tabs = [
                    (SettingsTab::Keyboard, "⌨ ", "tab.keyboard"),
                    (SettingsTab::TopRow, "🎛 ", "tab.top_row"),
                    (SettingsTab::Battery, "🔋 ", "tab.battery"),
                    (SettingsTab::General, "⚙ ", "tab.general"),
                    (SettingsTab::About, "ℹ ", "tab.about"),
                ];

                for (tab, icon, key) in tabs {
                    let label = format!("{}{}", icon, self.i18n.t(key));
                    let is_active = self.current_tab == tab;
                    let text = if is_active {
                        RichText::new(label).strong()
                    } else {
                        RichText::new(label)
                    };

                    if ui.selectable_label(is_active, text).clicked() {
                        self.current_tab = tab;
                    }
                }
            });
        });

        // 3. Central Content Panel
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            SettingsTab::Keyboard => {
                if let Some(change) =
                    keyboard_view::show(ui, &mut self.keyboard_state, &mut self.config, &self.i18n)
                {
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
                if top_row_view::show(ui, &mut self.top_row_state, &mut self.config, &self.i18n) {
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
            SettingsTab::About => {
                about_view::show(ui, self.daemon_status.as_ref(), &self.i18n);
            }
        });

        // Request a repaint periodically to update battery/connection status
        ctx.request_repaint_after(Duration::from_millis(1500));
    }
}
