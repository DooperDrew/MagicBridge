//! First-Run Initial Configuration Onboarding Wizard View.
//!
//! Provides a step-by-step interactive onboarding flow:
//! - Step 0: Welcome & Apple keyboard hardware auto-detection (USB / Bluetooth, battery level).
//! - Step 1: Visual card picker for physical and language keyboard layout (ISO Italian, ANSI US, ISO UK, ISO German, ISO French).
//! - Step 2: Modifier key preset selection (Mac Layout vs Windows Standard).
//! - Step 3: Fn layer trigger key choice (Right Option, Right Control, Caps Lock, F19).
//! - Step 4: System preferences (Autostart with Windows, Top row media keys first).
//! - Step 5: Interactive test text area with real-time modifier tracking, configuration summary, and "Save & Finish Setup".
//!
//! Complies with Issue #13, docs/07-ux-and-improvements.md, docs/04-ux.md, and NFR-P5.

use eframe::egui::{self, Color32, CornerRadius, ProgressBar, RichText, ScrollArea, Stroke, Ui};
use magicbridge_agent::DaemonStatus;
use magicbridge_agent::autostart;
use magicbridge_core::{Config, FnKeyChoice, KeyboardLayout, ModifierPreset, TopRowMode};

use crate::i18n::I18n;
use crate::ipc::SettingsIpc;

/// Virtual key code for Caps Lock (`VK_CAPITAL`).
pub const VK_CAPITAL_CODE: u16 = 0x14;

/// Action emitted by the onboarding wizard during UI updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardAction {
    /// No change in wizard state.
    None,
    /// Switched to the specified step index.
    StepChanged(usize),
    /// Setup completed and changes applied to configuration.
    Finished,
    /// Wizard was cancelled or exited.
    Cancelled,
}

/// State for the first-run onboarding configuration wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingWizardState {
    /// Current step index (0 to 5).
    pub current_step: usize,
    /// Selected keyboard physical and language layout.
    pub selected_layout: KeyboardLayout,
    /// Selected modifier preset.
    pub selected_modifier: ModifierPreset,
    /// Selected Fn layer trigger key.
    pub selected_fn_key: FnKeyChoice,
    /// Whether autostart with Windows is enabled.
    pub autostart: bool,
    /// Whether top-row keys default to media controls (true) or F-keys (false).
    pub media_keys_first: bool,
    /// Test text area input buffer for live verification.
    pub test_input_text: String,
    /// Whether the configuration has been saved.
    pub is_completed: bool,
}

impl Default for OnboardingWizardState {
    fn default() -> Self {
        Self {
            current_step: 0,
            selected_layout: KeyboardLayout::IsoItalian,
            selected_modifier: ModifierPreset::WindowsLayout,
            selected_fn_key: FnKeyChoice::RightOption,
            autostart: true,
            media_keys_first: true,
            test_input_text: String::new(),
            is_completed: false,
        }
    }
}

impl OnboardingWizardState {
    /// Creates a new wizard state initialized with standard defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new wizard state initialized from an existing [`Config`].
    #[must_use]
    pub fn new_from_config(config: &Config) -> Self {
        let profile = config.active_profile();
        let selected_layout = profile
            .map(|p| p.layout.clone())
            .unwrap_or(KeyboardLayout::IsoItalian);
        let selected_modifier = profile
            .map(|p| p.modifier_preset)
            .unwrap_or(ModifierPreset::WindowsLayout);
        let selected_fn_key = profile
            .map(|p| p.fn_key)
            .unwrap_or(FnKeyChoice::RightOption);
        let media_keys_first = profile
            .map(|p| p.top_row_mode == TopRowMode::MediaKeysFirst)
            .unwrap_or(true);
        let autostart = config.autostart;

        Self {
            current_step: 0,
            selected_layout,
            selected_modifier,
            selected_fn_key,
            autostart,
            media_keys_first,
            test_input_text: String::new(),
            is_completed: false,
        }
    }

    /// Advances to the next step (up to step 5).
    pub fn next_step(&mut self) {
        if self.current_step < 5 {
            self.current_step += 1;
        }
    }

    /// Returns to the previous step (down to step 0).
    pub fn prev_step(&mut self) {
        if self.current_step > 0 {
            self.current_step -= 1;
        }
    }

    /// Resets the wizard back to step 0.
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.is_completed = false;
        self.test_input_text.clear();
    }

    /// Applies the wizard choices to the given configuration and registers autostart if enabled.
    pub fn apply_to_config(&mut self, config: &mut Config) {
        if let Some(profile) = config.active_profile_mut() {
            profile.layout = self.selected_layout.clone();
            profile.modifier_preset = self.selected_modifier;
            profile.fn_key = self.selected_fn_key;
            profile.top_row_mode = if self.media_keys_first {
                TopRowMode::MediaKeysFirst
            } else {
                TopRowMode::FKeysFirst
            };
        }
        config.autostart = self.autostart;

        if self.autostart {
            if let Some(agent_exe) = SettingsIpc::find_agent_exe() {
                let _ = autostart::enable_autostart(&agent_exe);
            }
        } else {
            let _ = autostart::disable_autostart();
        }

        self.is_completed = true;
    }
}

/// Renders the Onboarding Configuration Wizard view.
pub fn show(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    config: &mut Config,
    daemon_status: Option<&DaemonStatus>,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading(i18n.t("wizard.title"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(i18n.t_format(
                            "wizard.step_counter",
                            &[
                                ("current", &(state.current_step + 1).to_string()),
                                ("total", "6"),
                            ],
                        ))
                        .strong()
                        .color(Color32::from_rgb(0, 120, 215)),
                    );
                });
            });
            ui.label(
                RichText::new(i18n.t("wizard.subtitle"))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(8.0);

            // Step Progress Bar / Tabs Indicator
            render_step_indicator(ui, state, i18n);
            ui.separator();
            ui.add_space(6.0);

            // Step Content
            let step_action = match state.current_step {
                0 => render_step_0_welcome(ui, state, daemon_status, i18n),
                1 => render_step_1_layout(ui, state, i18n),
                2 => render_step_2_modifiers(ui, state, i18n),
                3 => render_step_3_fn_key(ui, state, i18n),
                4 => render_step_4_preferences(ui, state, i18n),
                5 => render_step_5_test_finish(ui, state, config, i18n),
                _ => WizardAction::None,
            };

            if step_action != WizardAction::None {
                action = step_action;
            }
        });
    });

    action
}

/// Renders the horizontal step progress indicator across the top of the wizard.
fn render_step_indicator(ui: &mut Ui, state: &mut OnboardingWizardState, i18n: &I18n) {
    let steps = [
        (0, "wizard.step_0_name"),
        (1, "wizard.step_1_name"),
        (2, "wizard.step_2_name"),
        (3, "wizard.step_3_name"),
        (4, "wizard.step_4_name"),
        (5, "wizard.step_5_name"),
    ];

    ui.horizontal_wrapped(|ui| {
        for (idx, key) in steps {
            let is_current = state.current_step == idx;
            let is_past = state.current_step > idx;
            let label = format!("{}. {}", idx + 1, i18n.t(key));

            let text = if is_current {
                RichText::new(label)
                    .strong()
                    .color(Color32::from_rgb(0, 120, 215))
            } else if is_past {
                RichText::new(label).color(Color32::from_rgb(40, 160, 60))
            } else {
                RichText::new(label).color(ui.visuals().weak_text_color())
            };

            if ui.selectable_label(is_current, text).clicked() {
                state.current_step = idx;
            }

            if idx < 5 {
                ui.label(
                    RichText::new("›")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        }
    });
}

/// Step 0: Welcome and real-time hardware detection.
fn render_step_0_welcome(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    daemon_status: Option<&DaemonStatus>,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.welcome_title"));
        ui.label(i18n.t("wizard.welcome_desc"));
        ui.add_space(10.0);

        // Hardware detection card
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.strong(format!("🔍 {}", i18n.t("wizard.hw_detection_title")));
            });
            ui.add_space(4.0);

            let is_connected = daemon_status.is_some_and(|s| s.is_connected);
            let agent_running = daemon_status.is_some_and(|s| s.agent_running);

            if is_connected {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(40, 180, 80),
                        format!("✔ {}", i18n.t("wizard.hw_detected")),
                    );
                });

                if let Some(status) = daemon_status {
                    ui.horizontal(|ui| {
                        ui.label(i18n.t("wizard.hw_transport"));
                        let trans = status.transport.as_deref().unwrap_or("USB / Bluetooth");
                        ui.strong(trans);
                    });

                    if let Some(battery_pct) = status.battery_percent {
                        ui.horizontal(|ui| {
                            ui.label(i18n.t("wizard.hw_battery"));
                            let fraction = (battery_pct as f32) / 100.0;
                            let bar_color = if battery_pct <= 15 {
                                Color32::from_rgb(220, 50, 40)
                            } else {
                                Color32::from_rgb(40, 180, 80)
                            };
                            ui.add(
                                ProgressBar::new(fraction)
                                    .show_percentage()
                                    .desired_width(140.0)
                                    .fill(bar_color),
                            );
                            ui.label(format!("{battery_pct}%"));
                        });
                    }

                    ui.label(
                        RichText::new(i18n.t("wizard.hw_model"))
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            } else {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(230, 150, 20),
                        format!("⚡ {}", i18n.t("wizard.hw_not_detected")),
                    );
                });
                ui.label(i18n.t("wizard.hw_waiting"));

                if !agent_running {
                    ui.add_space(4.0);
                    ui.colored_label(
                        Color32::from_rgb(220, 60, 40),
                        format!("⚠ {}", i18n.t("wizard.agent_offline_warning")),
                    );
                    if ui.button(i18n.t("banner.start_agent")).clicked() {
                        let _ = SettingsIpc::launch_agent();
                    }
                }
            }
        });

        ui.add_space(16.0);

        // Action Navigation
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(i18n.t("wizard.btn_next")).strong())
                    .clicked()
                {
                    state.next_step();
                    action = WizardAction::StepChanged(state.current_step);
                }
            });
        });
    });

    action
}

/// Step 1: Visual layout selection cards.
fn render_step_1_layout(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.layout_title"));
        ui.label(i18n.t("wizard.layout_desc"));
        ui.add_space(8.0);

        let layouts = [
            (
                KeyboardLayout::IsoItalian,
                "IT",
                "wizard.layout_iso_it",
                "wizard.layout_iso_it_desc",
            ),
            (
                KeyboardLayout::AnsiUs,
                "US",
                "wizard.layout_ansi_us",
                "wizard.layout_ansi_us_desc",
            ),
            (
                KeyboardLayout::IsoUk,
                "UK",
                "wizard.layout_iso_uk",
                "wizard.layout_iso_uk_desc",
            ),
            (
                KeyboardLayout::IsoGerman,
                "DE",
                "wizard.layout_iso_de",
                "wizard.layout_iso_de_desc",
            ),
            (
                KeyboardLayout::IsoFrench,
                "FR",
                "wizard.layout_iso_fr",
                "wizard.layout_iso_fr_desc",
            ),
        ];

        for (layout, badge, title_key, desc_key) in layouts {
            let is_selected = state.selected_layout == layout;

            let stroke = if is_selected {
                Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 215))
            } else {
                Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
            };

            let bg = if is_selected {
                Color32::from_rgba_premultiplied(0, 120, 215, 20)
            } else {
                ui.visuals().window_fill()
            };

            egui::Frame::group(ui.style())
                .stroke(stroke)
                .fill(bg)
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.radio(is_selected, "").clicked() {
                            state.selected_layout = layout.clone();
                        }

                        ui.colored_label(
                            if is_selected {
                                Color32::from_rgb(0, 120, 215)
                            } else {
                                ui.visuals().text_color()
                            },
                            RichText::new(format!("[{badge}]")).strong(),
                        );

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(i18n.t(title_key));
                                if is_selected {
                                    ui.colored_label(Color32::from_rgb(40, 180, 80), "✔");
                                }
                            });
                            ui.label(
                                RichText::new(i18n.t(desc_key))
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
                });

            ui.add_space(4.0);
        }

        ui.add_space(12.0);

        // Action Navigation
        ui.horizontal(|ui| {
            if ui.button(i18n.t("wizard.btn_back")).clicked() {
                state.prev_step();
                action = WizardAction::StepChanged(state.current_step);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(i18n.t("wizard.btn_next")).strong())
                    .clicked()
                {
                    state.next_step();
                    action = WizardAction::StepChanged(state.current_step);
                }
            });
        });
    });

    action
}

/// Step 2: Modifier preset selection (Mac vs Windows Standard).
fn render_step_2_modifiers(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.modifier_title"));
        ui.label(i18n.t("wizard.modifier_desc"));
        ui.add_space(8.0);

        let presets = [
            (
                ModifierPreset::MacLayout,
                "wizard.modifier_mac",
                "wizard.modifier_mac_desc",
                "Command ➔ Windows Key  |  Option ➔ Alt",
            ),
            (
                ModifierPreset::WindowsLayout,
                "wizard.modifier_windows",
                "wizard.modifier_windows_desc",
                "Command ➔ Alt  |  Option ➔ Windows Key",
            ),
        ];

        for (preset, title_key, desc_key, mapping_hint) in presets {
            let is_selected = state.selected_modifier == preset;

            let stroke = if is_selected {
                Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 215))
            } else {
                Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
            };

            let bg = if is_selected {
                Color32::from_rgba_premultiplied(0, 120, 215, 20)
            } else {
                ui.visuals().window_fill()
            };

            egui::Frame::group(ui.style())
                .stroke(stroke)
                .fill(bg)
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.radio(is_selected, "").clicked() {
                            state.selected_modifier = preset;
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(i18n.t(title_key));
                                if is_selected {
                                    ui.colored_label(Color32::from_rgb(40, 180, 80), "✔");
                                }
                            });
                            ui.colored_label(
                                Color32::from_rgb(0, 120, 215),
                                RichText::new(mapping_hint).small().strong(),
                            );
                            ui.label(
                                RichText::new(i18n.t(desc_key))
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
                });

            ui.add_space(6.0);
        }

        ui.add_space(12.0);

        // Action Navigation
        ui.horizontal(|ui| {
            if ui.button(i18n.t("wizard.btn_back")).clicked() {
                state.prev_step();
                action = WizardAction::StepChanged(state.current_step);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(i18n.t("wizard.btn_next")).strong())
                    .clicked()
                {
                    state.next_step();
                    action = WizardAction::StepChanged(state.current_step);
                }
            });
        });
    });

    action
}

/// Step 3: Fn layer trigger key selection.
fn render_step_3_fn_key(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.fn_title"));
        ui.label(i18n.t("wizard.fn_desc"));
        ui.add_space(8.0);

        let choices = [
            (
                FnKeyChoice::RightOption,
                "wizard.fn_right_option",
                "wizard.fn_right_option_desc",
            ),
            (
                FnKeyChoice::RightControl,
                "wizard.fn_right_control",
                "wizard.fn_right_control_desc",
            ),
            (
                FnKeyChoice::Other(VK_CAPITAL_CODE),
                "wizard.fn_caps_lock",
                "wizard.fn_caps_lock_desc",
            ),
            (FnKeyChoice::F19, "wizard.fn_f19", "wizard.fn_f19_desc"),
        ];

        for (choice, title_key, desc_key) in choices {
            let is_selected = state.selected_fn_key == choice;

            let stroke = if is_selected {
                Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 215))
            } else {
                Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
            };

            let bg = if is_selected {
                Color32::from_rgba_premultiplied(0, 120, 215, 20)
            } else {
                ui.visuals().window_fill()
            };

            egui::Frame::group(ui.style())
                .stroke(stroke)
                .fill(bg)
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.radio(is_selected, "").clicked() {
                            state.selected_fn_key = choice;
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(i18n.t(title_key));
                                if is_selected {
                                    ui.colored_label(Color32::from_rgb(40, 180, 80), "✔");
                                }
                            });
                            ui.label(
                                RichText::new(i18n.t(desc_key))
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
                });

            ui.add_space(4.0);
        }

        ui.add_space(12.0);

        // Action Navigation
        ui.horizontal(|ui| {
            if ui.button(i18n.t("wizard.btn_back")).clicked() {
                state.prev_step();
                action = WizardAction::StepChanged(state.current_step);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(i18n.t("wizard.btn_next")).strong())
                    .clicked()
                {
                    state.next_step();
                    action = WizardAction::StepChanged(state.current_step);
                }
            });
        });
    });

    action
}

/// Step 4: System preferences and autostart toggles.
fn render_step_4_preferences(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.pref_title"));
        ui.label(i18n.t("wizard.pref_desc"));
        ui.add_space(8.0);

        // Autostart card
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.autostart, i18n.t("wizard.pref_autostart"));
            });
            ui.indent("pref_autostart_indent", |ui| {
                ui.label(
                    RichText::new(i18n.t("wizard.pref_autostart_desc"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            });
        });

        ui.add_space(6.0);

        // Top Row Media Keys First card
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut state.media_keys_first,
                    i18n.t("wizard.pref_media_first"),
                );
            });
            ui.indent("pref_media_indent", |ui| {
                ui.label(
                    RichText::new(i18n.t("wizard.pref_media_first_desc"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            });
        });

        ui.add_space(12.0);

        // Action Navigation
        ui.horizontal(|ui| {
            if ui.button(i18n.t("wizard.btn_back")).clicked() {
                state.prev_step();
                action = WizardAction::StepChanged(state.current_step);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(i18n.t("wizard.btn_next")).strong())
                    .clicked()
                {
                    state.next_step();
                    action = WizardAction::StepChanged(state.current_step);
                }
            });
        });
    });

    action
}

/// Step 5: Interactive test text area, summary card, and "Save & Finish Setup".
fn render_step_5_test_finish(
    ui: &mut Ui,
    state: &mut OnboardingWizardState,
    config: &mut Config,
    i18n: &I18n,
) -> WizardAction {
    let mut action = WizardAction::None;

    ui.vertical(|ui| {
        ui.heading(i18n.t("wizard.test_title"));
        ui.label(i18n.t("wizard.test_desc"));
        ui.add_space(8.0);

        // Interactive Text Input Area
        ui.group(|ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.test_input_text)
                    .hint_text(i18n.t("wizard.test_hint"))
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(4.0);

            // Active Modifier Indicator
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(i18n.t("wizard.test_active_modifiers"))
                        .small()
                        .strong(),
                );
                let modifiers = ui.input(|i| i.modifiers);

                let render_modifier_badge = |ui: &mut Ui, name: &str, active: bool| {
                    let stroke = if active {
                        Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 215))
                    } else {
                        Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
                    };
                    let bg = if active {
                        Color32::from_rgb(0, 120, 215)
                    } else {
                        ui.visuals().window_fill()
                    };
                    let fg = if active {
                        Color32::WHITE
                    } else {
                        ui.visuals().weak_text_color()
                    };

                    egui::Frame::group(ui.style())
                        .stroke(stroke)
                        .fill(bg)
                        .corner_radius(CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.colored_label(fg, RichText::new(name).small().strong());
                        });
                };

                render_modifier_badge(ui, "Ctrl", modifiers.ctrl);
                render_modifier_badge(ui, "Alt", modifiers.alt);
                render_modifier_badge(ui, "Shift", modifiers.shift);
                render_modifier_badge(ui, "Win/Cmd", modifiers.command || modifiers.mac_cmd);
            });
        });

        ui.add_space(8.0);

        // Configuration Summary Card
        ui.group(|ui| {
            ui.strong(format!("📋 {}", i18n.t("wizard.test_summary_title")));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(i18n.t("wizard.test_summary_layout"));
                ui.strong(state.selected_layout.to_string());
            });

            ui.horizontal(|ui| {
                ui.label(i18n.t("wizard.test_summary_modifier"));
                ui.strong(state.selected_modifier.to_string());
            });

            ui.horizontal(|ui| {
                ui.label(i18n.t("wizard.test_summary_fn"));
                let fn_str = match state.selected_fn_key {
                    FnKeyChoice::RightOption => i18n.t("wizard.fn_right_option"),
                    FnKeyChoice::RightControl => i18n.t("wizard.fn_right_control"),
                    FnKeyChoice::Other(VK_CAPITAL_CODE) => i18n.t("wizard.fn_caps_lock"),
                    FnKeyChoice::F19 => i18n.t("wizard.fn_f19"),
                    _ => i18n.t("top_row.fn_other"),
                };
                ui.strong(fn_str);
            });

            ui.horizontal(|ui| {
                ui.label(i18n.t("wizard.test_summary_autostart"));
                ui.strong(if state.autostart {
                    i18n.t("about.status_yes")
                } else {
                    i18n.t("about.status_no")
                });
            });

            ui.horizontal(|ui| {
                ui.label(i18n.t("wizard.test_summary_top_row"));
                ui.strong(if state.media_keys_first {
                    i18n.t("top_row.media_first")
                } else {
                    i18n.t("top_row.fkeys_first")
                });
            });
        });

        if state.is_completed {
            ui.add_space(4.0);
            ui.colored_label(
                Color32::from_rgb(40, 180, 80),
                format!("✔ {}", i18n.t("wizard.test_saved")),
            );
        }

        ui.add_space(12.0);

        // Action Navigation & Big Finish Button
        ui.horizontal(|ui| {
            if ui.button(i18n.t("wizard.btn_back")).clicked() {
                state.prev_step();
                action = WizardAction::StepChanged(state.current_step);
            }

            if ui.button(i18n.t("wizard.btn_restart")).clicked() {
                state.reset();
                action = WizardAction::StepChanged(state.current_step);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let finish_btn = egui::Button::new(
                    RichText::new(i18n.t("wizard.btn_finish"))
                        .strong()
                        .size(14.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(0, 120, 215))
                .corner_radius(CornerRadius::same(6));

                if ui.add(finish_btn).clicked() {
                    state.apply_to_config(config);
                    action = WizardAction::Finished;
                }
            });
        });
    });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_state_defaults() {
        let state = OnboardingWizardState::default();
        assert_eq!(state.current_step, 0);
        assert_eq!(state.selected_layout, KeyboardLayout::IsoItalian);
        assert_eq!(state.selected_modifier, ModifierPreset::WindowsLayout);
        assert_eq!(state.selected_fn_key, FnKeyChoice::RightOption);
        assert!(state.autostart);
        assert!(state.media_keys_first);
        assert!(!state.is_completed);
    }

    #[test]
    fn test_wizard_state_new_from_config() {
        let mut config = Config::default();
        if let Some(profile) = config.active_profile_mut() {
            profile.layout = KeyboardLayout::AnsiUs;
            profile.modifier_preset = ModifierPreset::MacLayout;
            profile.fn_key = FnKeyChoice::F19;
            profile.top_row_mode = TopRowMode::FKeysFirst;
        }
        config.autostart = false;

        let state = OnboardingWizardState::new_from_config(&config);
        assert_eq!(state.selected_layout, KeyboardLayout::AnsiUs);
        assert_eq!(state.selected_modifier, ModifierPreset::MacLayout);
        assert_eq!(state.selected_fn_key, FnKeyChoice::F19);
        assert!(!state.media_keys_first);
        assert!(!state.autostart);
    }

    #[test]
    fn test_wizard_step_transitions() {
        let mut state = OnboardingWizardState::new();
        assert_eq!(state.current_step, 0);

        state.next_step();
        assert_eq!(state.current_step, 1);

        state.next_step();
        assert_eq!(state.current_step, 2);

        state.prev_step();
        assert_eq!(state.current_step, 1);

        // Advance to max step 5
        for _ in 0..10 {
            state.next_step();
        }
        assert_eq!(state.current_step, 5);

        // Reset
        state.reset();
        assert_eq!(state.current_step, 0);
        assert!(!state.is_completed);
    }

    #[test]
    fn test_wizard_apply_to_config() {
        let mut config = Config::default();
        let mut state = OnboardingWizardState::new();

        state.selected_layout = KeyboardLayout::IsoGerman;
        state.selected_modifier = ModifierPreset::MacLayout;
        state.selected_fn_key = FnKeyChoice::Other(VK_CAPITAL_CODE);
        state.autostart = false;
        state.media_keys_first = false;

        state.apply_to_config(&mut config);

        assert!(state.is_completed);
        let profile = config.active_profile().expect("active profile exists");
        assert_eq!(profile.layout, KeyboardLayout::IsoGerman);
        assert_eq!(profile.modifier_preset, ModifierPreset::MacLayout);
        assert_eq!(profile.fn_key, FnKeyChoice::Other(VK_CAPITAL_CODE));
        assert_eq!(profile.top_row_mode, TopRowMode::FKeysFirst);
        assert!(!config.autostart);
    }

    #[test]
    fn test_all_wizard_translation_keys_exist() {
        let mut i18n = I18n::default();

        let keys = [
            "wizard.title",
            "wizard.subtitle",
            "wizard.step_counter",
            "wizard.step_0_name",
            "wizard.step_1_name",
            "wizard.step_2_name",
            "wizard.step_3_name",
            "wizard.step_4_name",
            "wizard.step_5_name",
            "wizard.btn_next",
            "wizard.btn_back",
            "wizard.btn_finish",
            "wizard.btn_restart",
            "wizard.welcome_title",
            "wizard.welcome_desc",
            "wizard.hw_detection_title",
            "wizard.hw_detected",
            "wizard.hw_not_detected",
            "wizard.hw_waiting",
            "wizard.hw_transport",
            "wizard.hw_battery",
            "wizard.hw_model",
            "wizard.agent_offline_warning",
            "wizard.layout_title",
            "wizard.layout_desc",
            "wizard.layout_iso_it",
            "wizard.layout_iso_it_desc",
            "wizard.layout_ansi_us",
            "wizard.layout_ansi_us_desc",
            "wizard.layout_iso_uk",
            "wizard.layout_iso_uk_desc",
            "wizard.layout_iso_de",
            "wizard.layout_iso_de_desc",
            "wizard.layout_iso_fr",
            "wizard.layout_iso_fr_desc",
            "wizard.modifier_title",
            "wizard.modifier_desc",
            "wizard.modifier_mac",
            "wizard.modifier_mac_desc",
            "wizard.modifier_windows",
            "wizard.modifier_windows_desc",
            "wizard.fn_title",
            "wizard.fn_desc",
            "wizard.fn_right_option",
            "wizard.fn_right_option_desc",
            "wizard.fn_right_control",
            "wizard.fn_right_control_desc",
            "wizard.fn_caps_lock",
            "wizard.fn_caps_lock_desc",
            "wizard.fn_f19",
            "wizard.fn_f19_desc",
            "wizard.pref_title",
            "wizard.pref_desc",
            "wizard.pref_autostart",
            "wizard.pref_autostart_desc",
            "wizard.pref_media_first",
            "wizard.pref_media_first_desc",
            "wizard.test_title",
            "wizard.test_desc",
            "wizard.test_hint",
            "wizard.test_active_modifiers",
            "wizard.test_summary_title",
            "wizard.test_summary_layout",
            "wizard.test_summary_modifier",
            "wizard.test_summary_fn",
            "wizard.test_summary_autostart",
            "wizard.test_summary_top_row",
            "wizard.test_saved",
        ];

        for lang in ["en", "it"] {
            i18n.set_language(lang);
            for key in &keys {
                let trans = i18n.t(key);
                assert_ne!(
                    trans, *key,
                    "Translation key '{key}' missing in language '{lang}'"
                );
            }
        }
    }
}
