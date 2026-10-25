//! Visual A1843 keyboard view, profile management, and key editor side panel.
//!
//! Complies with F7.2, F7.4, F7.7, and docs/04-ux.md.

use egui::{Color32, CornerRadius, Rect, Sense, Stroke, Ui, Vec2};
use magicbridge_core::{
    Config, CustomAction, FnKeyChoice, KeyAction, KeyboardLayout, ModifierPreset, NamedProfile,
};

use crate::i18n::I18n;
use crate::keyboard_model::{A1843Model, KeyCluster, PhysicalKey};

/// Active layer being inspected or edited in the visual keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveLayer {
    #[default]
    Normal,
    Fn,
}

/// Modal dialog state for profile operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDialog {
    None,
    New {
        name: String,
        layout: KeyboardLayout,
    },
    Duplicate {
        name: String,
    },
    Rename {
        name: String,
    },
    DeleteConfirm {
        name: String,
    },
}

/// State for the Key Editor side panel.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyEditorState {
    pub selected_key_id: Option<String>,
    pub action_type_idx: usize,
    pub target_key_name: String,
    pub media_command: String,
    pub launch_command: String,
    pub url_target: String,
    pub shortcut_keys: String,
}

impl Default for KeyEditorState {
    fn default() -> Self {
        Self {
            selected_key_id: None,
            action_type_idx: 0, // 0 = Key, 1 = Media, 2 = Launch, 3 = URL, 4 = Shortcut
            target_key_name: "PrintScreen".to_string(),
            media_command: "volume_up".to_string(),
            launch_command: String::new(),
            url_target: String::new(),
            shortcut_keys: String::new(),
        }
    }
}

/// State for the Keyboard tab.
#[derive(Debug, Clone)]
pub struct KeyboardViewState {
    pub active_layer: ActiveLayer,
    pub listen_key_active: bool,
    pub model: A1843Model,
    pub editor: KeyEditorState,
    pub dialog: ProfileDialog,
    pub dialog_error: Option<String>,
}

impl Default for KeyboardViewState {
    fn default() -> Self {
        Self {
            active_layer: ActiveLayer::Normal,
            listen_key_active: false,
            model: A1843Model::new(),
            editor: KeyEditorState::default(),
            dialog: ProfileDialog::None,
            dialog_error: None,
        }
    }
}

/// Renders the complete Keyboard view tab including profile bar, visual keyboard, and key editor.
pub fn show(
    ui: &mut Ui,
    state: &mut KeyboardViewState,
    config: &mut Config,
    i18n: &I18n,
) -> Option<ConfigChange> {
    let mut change = None;

    // 1. Profile Bar at the top (F7.7, F7.4, Issue #14: horizontal wrapping)
    ui.horizontal_wrapped(|ui| {
        ui.label(i18n.t("profile.label"));

        // Active profile dropdown
        let current_profile_name = config.active_profile.clone();
        let mut selected_name = current_profile_name.clone();

        egui::ComboBox::from_id_salt("profile_selector")
            .selected_text(&selected_name)
            .show_ui(ui, |ui| {
                for name in config.profiles.keys() {
                    ui.selectable_value(&mut selected_name, name.clone(), name);
                }
            });

        if selected_name != current_profile_name
            && let Ok(()) = config.set_active_profile(&selected_name)
        {
            change = Some(ConfigChange::ActiveProfileChanged(selected_name));
        }

        // Profile action buttons: New, Duplicate, Rename, Delete
        if ui.button(i18n.t("profile.new")).clicked() {
            state.dialog = ProfileDialog::New {
                name: format!("Profile {}", config.profiles.len() + 1),
                layout: KeyboardLayout::IsoItalian,
            };
            state.dialog_error = None;
        }

        if ui.button(i18n.t("profile.duplicate")).clicked() {
            state.dialog = ProfileDialog::Duplicate {
                name: format!("{} (Copy)", config.active_profile),
            };
            state.dialog_error = None;
        }

        if ui.button(i18n.t("profile.rename")).clicked() {
            state.dialog = ProfileDialog::Rename {
                name: config.active_profile.clone(),
            };
            state.dialog_error = None;
        }

        let can_delete = config.profiles.len() > 1;
        if ui
            .add_enabled(can_delete, egui::Button::new(i18n.t("profile.delete")))
            .clicked()
        {
            state.dialog = ProfileDialog::DeleteConfirm {
                name: config.active_profile.clone(),
            };
            state.dialog_error = None;
        }

        ui.separator();

        // Physical & Language Layout Selector (F7.4)
        if let Some(profile) = config.active_profile_mut() {
            ui.label(i18n.t("profile.layout"));
            let current_layout_str = format!("{}", profile.layout);
            let mut selected_layout = profile.layout.clone();

            egui::ComboBox::from_id_salt("layout_selector")
                .selected_text(current_layout_str)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected_layout,
                        KeyboardLayout::IsoItalian,
                        "ISO Italian",
                    );
                    ui.selectable_value(&mut selected_layout, KeyboardLayout::AnsiUs, "ANSI US");
                    ui.selectable_value(&mut selected_layout, KeyboardLayout::IsoUk, "ISO UK");
                    ui.selectable_value(
                        &mut selected_layout,
                        KeyboardLayout::IsoGerman,
                        "ISO German",
                    );
                    ui.selectable_value(
                        &mut selected_layout,
                        KeyboardLayout::IsoFrench,
                        "ISO French",
                    );
                });

            if selected_layout != profile.layout {
                profile.layout = selected_layout;
                change = Some(ConfigChange::Modified);
            }
        }
    });

    // Profile Dialogs (New / Duplicate / Rename / Delete)
    if let Some(dialog_change) = render_profile_dialogs(ui, state, config, i18n) {
        change = Some(dialog_change);
    }

    ui.separator();

    // 2. Toolbar: Layer Switch, Presets, "Press a key to select" (Issue #14: horizontal wrapping)
    ui.horizontal_wrapped(|ui| {
        ui.label(i18n.t("keyboard.layer"));
        ui.selectable_value(
            &mut state.active_layer,
            ActiveLayer::Normal,
            i18n.t("keyboard.layer_normal"),
        );
        ui.selectable_value(
            &mut state.active_layer,
            ActiveLayer::Fn,
            i18n.t("keyboard.layer_fn"),
        );

        ui.separator();

        // Presets dropdown (F3.1, F3.2)
        egui::ComboBox::from_id_salt("presets_dropdown")
            .selected_text(i18n.t("keyboard.presets"))
            .show_ui(ui, |ui| {
                if ui.button(i18n.t("keyboard.preset_windows")).clicked()
                    && let Some(p) = config.active_profile_mut()
                {
                    p.modifier_preset = ModifierPreset::WindowsLayout;
                    change = Some(ConfigChange::Modified);
                }
                if ui.button(i18n.t("keyboard.preset_mac")).clicked()
                    && let Some(p) = config.active_profile_mut()
                {
                    p.modifier_preset = ModifierPreset::MacLayout;
                    change = Some(ConfigChange::Modified);
                }
                if ui.button(i18n.t("keyboard.preset_reset_all")).clicked()
                    && let Some(p) = config.active_profile_mut()
                {
                    let default = NamedProfile::default_it();
                    p.mappings = default.mappings;
                    p.fn_mappings = default.fn_mappings;
                    p.modifier_preset = ModifierPreset::WindowsLayout;
                    change = Some(ConfigChange::Modified);
                }
            });

        ui.separator();

        // "Press a key to select" toggle
        let listen_label = if state.listen_key_active {
            i18n.t("keyboard.listening")
        } else {
            i18n.t("keyboard.listen_key")
        };
        if ui
            .toggle_value(&mut state.listen_key_active, listen_label)
            .changed()
            && state.listen_key_active
        {
            state.editor.selected_key_id = None;
        }

        ui.separator();

        // Color legend indicators
        ui.colored_label(Color32::from_rgb(45, 125, 210), "■");
        ui.label(i18n.t("keyboard.legend_remapped"));
        ui.colored_label(Color32::from_rgb(180, 100, 40), "■");
        ui.label(i18n.t("keyboard.legend_fn_trigger"));
    });

    // Handle "Press a key to select" keyboard listener
    if state.listen_key_active {
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Key {
                    key, pressed: true, ..
                } = event
                    && let Some(matched_key) = find_matching_physical_key(*key, &state.model)
                {
                    state.editor.selected_key_id = Some(matched_key.id.to_string());
                    state.listen_key_active = false;
                    sync_editor_with_selection(&mut state.editor, config, state.active_layer);
                    break;
                }
            }
        });
    }

    ui.add_space(4.0);

    // 3. Visual A1843 Keyboard Canvas
    let active_profile = config.active_profile().cloned();
    let current_layout = active_profile
        .as_ref()
        .map(|p| p.layout.clone())
        .unwrap_or(KeyboardLayout::IsoItalian);

    let keyboard_response = render_visual_keyboard(
        ui,
        state,
        active_profile.as_ref(),
        &current_layout,
        state.active_layer,
    );

    if let Some(clicked_id) = keyboard_response {
        state.editor.selected_key_id = Some(clicked_id);
        sync_editor_with_selection(&mut state.editor, config, state.active_layer);
    }

    ui.separator();

    // 4. Key Editor Side/Bottom Panel (F5.1, F7.2)
    if let Some(editor_change) = render_key_editor(ui, state, config, i18n) {
        change = Some(editor_change);
    }

    change
}

/// Changes made to the configuration during UI interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChange {
    Modified,
    ActiveProfileChanged(String),
    ProfileCreated(String),
    ProfileDeleted(String),
}

/// Dynamically calculates keyboard unit size in points based on available width.
///
/// Follows Issue #14 responsive layout: `(available_width / 24.5).clamp(20.0, 42.0)`.
#[must_use]
pub fn calculate_unit_size(available_width: f32) -> f32 {
    (available_width / 24.5).clamp(20.0, 42.0)
}

/// Dynamically calculates key font size in points based on unit size, key width, cluster, and label length.
#[must_use]
pub fn calculate_key_font_size(
    unit_size: f32,
    key_width: f32,
    cluster: KeyCluster,
    label_len: usize,
) -> f32 {
    if key_width >= 1.5 {
        if label_len > 6 {
            (unit_size * 0.28).clamp(7.0, 12.5)
        } else {
            (unit_size * 0.34).clamp(8.0, 14.5)
        }
    } else if cluster == KeyCluster::Main && label_len <= 2 {
        (unit_size * 0.38).clamp(8.5, 16.0)
    } else if label_len <= 3 {
        (unit_size * 0.32).clamp(7.5, 13.5)
    } else if label_len <= 5 {
        (unit_size * 0.26).clamp(6.5, 11.5)
    } else {
        (unit_size * 0.22).clamp(6.0, 10.0)
    }
}

/// Renders the visual A1843 keyboard grid to scale.
fn render_visual_keyboard(
    ui: &mut Ui,
    state: &KeyboardViewState,
    profile: Option<&NamedProfile>,
    layout: &KeyboardLayout,
    layer: ActiveLayer,
) -> Option<String> {
    let mut clicked_key_id = None;

    // Dynamically calculate unit_size based on available width (Issue #14)
    let available_width = ui.available_width();
    let unit_size = calculate_unit_size(available_width);

    // Dynamically scale key height, spacing, and font sizes
    let key_height = (unit_size * 0.95).round();
    let key_spacing = (unit_size * 0.055).clamp(1.0, 2.5);

    let keyboard_total_width = (23.0 * (unit_size + key_spacing)) + unit_size;
    let total_height = (6.0 * (key_height + key_spacing)) + 8.0;

    // Center keyboard horizontally if available width exceeds total keyboard width
    let margin_x = ((available_width - keyboard_total_width) / 2.0).max(4.0);
    let margin_y = 4.0;

    let (rect, _response) = ui.allocate_exact_size(
        Vec2::new(available_width, total_height),
        Sense::click_and_drag(),
    );

    let painter = ui.painter_at(rect);

    // Draw keyboard aluminum base plate (centered with bezel)
    let plate_rect = Rect::from_min_size(
        egui::pos2(rect.left() + margin_x - 4.0, rect.top()),
        Vec2::new(keyboard_total_width + 8.0, total_height),
    );
    painter.rect_filled(
        plate_rect,
        CornerRadius::same(6),
        Color32::from_rgb(235, 237, 240),
    );
    painter.rect_stroke(
        plate_rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_rgb(215, 218, 222)),
        egui::StrokeKind::Inside,
    );

    // Dynamic corner radius for keys based on unit size
    let key_corner_radius = ((unit_size * 0.1).round() as u8).clamp(2, 4);

    // Draw each physical key
    for key in &state.model.keys {
        let key_x = rect.left() + margin_x + (key.x * (unit_size + key_spacing));
        let key_y = rect.top() + margin_y + (key.row as f32 * (key_height + key_spacing));
        let key_w = (key.width * unit_size) + ((key.width - 1.0) * key_spacing);
        let key_h = (key.height * key_height) + ((key.height - 1.0) * key_spacing);

        let key_rect = Rect::from_min_size(egui::pos2(key_x, key_y), Vec2::new(key_w, key_h));

        let id = ui.make_persistent_id(format!("key_{}", key.id));
        let response = ui.interact(key_rect, id, Sense::click());

        if response.clicked() {
            clicked_key_id = Some(key.id.to_string());
        }

        // Determine key highlighting
        let is_selected = state
            .editor
            .selected_key_id
            .as_deref()
            .is_some_and(|sel| sel.eq_ignore_ascii_case(key.id));

        let is_fn_trigger = profile.is_some_and(|p| match p.fn_key {
            FnKeyChoice::RightOption => key.id == "RightOption",
            FnKeyChoice::RightControl => key.id == "RightControl",
            FnKeyChoice::F19 => key.id == "F19",
            FnKeyChoice::Disabled => false,
            FnKeyChoice::Other(vk) => key.vk == vk,
        });

        let is_remapped = profile.is_some_and(|p| match layer {
            ActiveLayer::Normal => p.mappings.contains_key(key.id),
            ActiveLayer::Fn => p.fn_mappings.contains_key(key.id),
        });

        let is_modifier_swapped = profile.is_some_and(|p| {
            p.modifier_preset == ModifierPreset::WindowsLayout
                && matches!(
                    key.id,
                    "LeftCommand" | "RightCommand" | "LeftOption" | "RightOption"
                )
        });

        // Determine colors
        let fill_color = if is_selected {
            Color32::from_rgb(255, 235, 180)
        } else if is_fn_trigger {
            Color32::from_rgb(250, 220, 190)
        } else if is_remapped {
            Color32::from_rgb(210, 230, 255)
        } else if is_modifier_swapped {
            Color32::from_rgb(220, 240, 230)
        } else if response.hovered() {
            Color32::from_rgb(245, 247, 250)
        } else {
            Color32::WHITE
        };

        let stroke_color = if is_selected {
            Color32::from_rgb(220, 120, 0)
        } else if is_remapped {
            Color32::from_rgb(45, 125, 210)
        } else if is_fn_trigger {
            Color32::from_rgb(180, 100, 40)
        } else {
            Color32::from_rgb(195, 200, 205)
        };

        let stroke_width = if is_selected { 2.0_f32 } else { 1.0_f32 };

        painter.rect(
            key_rect,
            CornerRadius::same(key_corner_radius),
            fill_color,
            Stroke::new(stroke_width, stroke_color),
            egui::StrokeKind::Inside,
        );

        // Get localized label
        let label = state.model.get_key_label(key.id, layout);
        let text_color = if is_selected {
            Color32::from_rgb(160, 80, 0)
        } else if is_remapped {
            Color32::from_rgb(20, 80, 180)
        } else {
            Color32::from_rgb(40, 45, 50)
        };

        let font_size =
            calculate_key_font_size(unit_size, key.width, key.cluster, label.chars().count());

        painter.text(
            key_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }

    clicked_key_id
}

/// Synchronizes the key editor fields with the newly selected key and current profile.
fn sync_editor_with_selection(editor: &mut KeyEditorState, config: &Config, layer: ActiveLayer) {
    if let Some(ref key_id) = editor.selected_key_id
        && let Some(profile) = config.active_profile()
    {
        let mapping = match layer {
            ActiveLayer::Normal => profile.mappings.get(key_id),
            ActiveLayer::Fn => profile.fn_mappings.get(key_id),
        };

        if let Some(action) = mapping {
            match action {
                KeyAction::Key(target) => {
                    editor.action_type_idx = 0;
                    editor.target_key_name = target.clone();
                }
                KeyAction::Custom(CustomAction::Media { command }) => {
                    editor.action_type_idx = 1;
                    editor.media_command = command.clone();
                }
                KeyAction::Custom(CustomAction::Launch { command }) => {
                    editor.action_type_idx = 2;
                    editor.launch_command = command.clone();
                }
                KeyAction::Custom(CustomAction::OpenUrl { url }) => {
                    editor.action_type_idx = 3;
                    editor.url_target = url.clone();
                }
                KeyAction::Custom(CustomAction::Shortcut { keys }) => {
                    editor.action_type_idx = 4;
                    editor.shortcut_keys = keys.join("+");
                }
            }
        } else {
            // Default target key for unmapped keys
            editor.action_type_idx = 0;
            editor.target_key_name = key_id.clone();
        }
    }
}

/// Renders the Key Editor side/bottom panel for the selected key.
fn render_key_editor(
    ui: &mut Ui,
    state: &mut KeyboardViewState,
    config: &mut Config,
    i18n: &I18n,
) -> Option<ConfigChange> {
    let mut change = None;
    let available_width = ui.available_width();
    let is_compact = available_width < 850.0;

    ui.group(|ui| {
        ui.heading(i18n.t("editor.title"));

        let selected_id = state.editor.selected_key_id.clone();
        if let Some(key_id) = selected_id {
            let physical = state.model.find_by_id(&key_id);
            let vk = physical.map(|p| p.vk).unwrap_or(0);

            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} {}", i18n.t("editor.selected_key"), key_id));
                ui.label(format!("({} 0x{:02X})", i18n.t("editor.original_vk"), vk));
                ui.label(format!(
                    "{} {}",
                    i18n.t("editor.current_layer"),
                    match state.active_layer {
                        ActiveLayer::Normal => i18n.t("keyboard.layer_normal"),
                        ActiveLayer::Fn => i18n.t("keyboard.layer_fn"),
                    }
                ));
            });

            ui.add_space(2.0);

            // Action type selector (wrapped for responsive adaptation)
            ui.horizontal_wrapped(|ui| {
                ui.label(i18n.t("editor.action_type"));
                ui.selectable_value(
                    &mut state.editor.action_type_idx,
                    0,
                    i18n.t("editor.action_key"),
                );
                ui.selectable_value(
                    &mut state.editor.action_type_idx,
                    1,
                    i18n.t("editor.action_media"),
                );
                ui.selectable_value(
                    &mut state.editor.action_type_idx,
                    2,
                    i18n.t("editor.action_launch"),
                );
                ui.selectable_value(
                    &mut state.editor.action_type_idx,
                    3,
                    i18n.t("editor.action_url"),
                );
                ui.selectable_value(
                    &mut state.editor.action_type_idx,
                    4,
                    i18n.t("editor.action_shortcut"),
                );
            });

            ui.add_space(4.0);

            if is_compact {
                // Stacked layout when available width is restricted (<850px)
                ui.horizontal_wrapped(|ui| {
                    render_editor_parameters(ui, &mut state.editor, i18n);
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(c) = render_editor_buttons(
                        ui,
                        &state.editor,
                        state.active_layer,
                        config,
                        &key_id,
                        i18n,
                    ) {
                        change = Some(c);
                    }
                });
            } else {
                // Side-by-side layout when width is ample (>=850px)
                ui.horizontal(|ui| {
                    render_editor_parameters(ui, &mut state.editor, i18n);
                    ui.separator();
                    if let Some(c) = render_editor_buttons(
                        ui,
                        &state.editor,
                        state.active_layer,
                        config,
                        &key_id,
                        i18n,
                    ) {
                        change = Some(c);
                    }
                });
            }
        } else {
            ui.label(i18n.t("editor.no_selection"));
        }
    });

    change
}

/// Helper to render the parameters for the currently selected action type.
fn render_editor_parameters(ui: &mut Ui, editor: &mut KeyEditorState, i18n: &I18n) {
    match editor.action_type_idx {
        0 => {
            // Remap Key (target key)
            ui.label(i18n.t("editor.target_key"));
            let targets = [
                "PrintScreen",
                "ScrollLock",
                "Pause",
                "Insert",
                "Delete",
                "Home",
                "End",
                "PageUp",
                "PageDown",
                "Left",
                "Right",
                "Up",
                "Down",
                "F1",
                "F2",
                "F3",
                "F4",
                "F5",
                "F6",
                "F7",
                "F8",
                "F9",
                "F10",
                "F11",
                "F12",
                "F13",
                "F14",
                "F15",
                "F16",
                "F17",
                "F18",
                "F19",
                "LeftControl",
                "RightControl",
                "LeftOption",
                "RightOption",
                "LeftCommand",
                "RightCommand",
                "Escape",
                "Backspace",
                "Tab",
                "Space",
            ];

            egui::ComboBox::from_id_salt("target_key_combo")
                .selected_text(&editor.target_key_name)
                .show_ui(ui, |ui| {
                    for t in targets {
                        ui.selectable_value(&mut editor.target_key_name, t.to_string(), t);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut editor.target_key_name).desired_width(120.0));
        }
        1 => {
            // Media Action
            ui.label(i18n.t("editor.media_command"));
            let media_ops = [
                ("volume_up", "Volume Up"),
                ("volume_down", "Volume Down"),
                ("volume_mute", "Mute"),
                ("play_pause", "Play / Pause"),
                ("next_track", "Next Track"),
                ("prev_track", "Previous Track"),
                ("stop", "Stop"),
            ];
            egui::ComboBox::from_id_salt("media_combo")
                .selected_text(&editor.media_command)
                .show_ui(ui, |ui| {
                    for (cmd, desc) in media_ops {
                        ui.selectable_value(&mut editor.media_command, cmd.to_string(), desc);
                    }
                });
        }
        2 => {
            // Launch Program
            ui.label(i18n.t("editor.command"));
            ui.add(egui::TextEdit::singleline(&mut editor.launch_command).desired_width(260.0));
        }
        3 => {
            // Open URL
            ui.label(i18n.t("editor.url"));
            ui.add(egui::TextEdit::singleline(&mut editor.url_target).desired_width(260.0));
        }
        4 => {
            // Send Shortcut
            ui.label(i18n.t("editor.shortcut_keys"));
            ui.add(egui::TextEdit::singleline(&mut editor.shortcut_keys).desired_width(180.0));
        }
        _ => {}
    }
}

/// Helper to render the Save and Reset buttons for the key editor.
fn render_editor_buttons(
    ui: &mut Ui,
    editor: &KeyEditorState,
    layer: ActiveLayer,
    config: &mut Config,
    key_id: &str,
    i18n: &I18n,
) -> Option<ConfigChange> {
    let mut change = None;

    // Apply button
    if ui.button(i18n.t("editor.save_mapping")).clicked() {
        let action = match editor.action_type_idx {
            0 => Some(KeyAction::Key(editor.target_key_name.trim().to_string())),
            1 => Some(KeyAction::Custom(CustomAction::Media {
                command: editor.media_command.clone(),
            })),
            2 => Some(KeyAction::Custom(CustomAction::Launch {
                command: editor.launch_command.trim().to_string(),
            })),
            3 => Some(KeyAction::Custom(CustomAction::OpenUrl {
                url: editor.url_target.trim().to_string(),
            })),
            4 => {
                let keys = editor
                    .shortcut_keys
                    .split('+')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                Some(KeyAction::Custom(CustomAction::Shortcut { keys }))
            }
            _ => None,
        };

        if let Some(act) = action
            && let Some(profile) = config.active_profile_mut()
        {
            match layer {
                ActiveLayer::Normal => {
                    profile.mappings.insert(key_id.to_string(), act);
                }
                ActiveLayer::Fn => {
                    profile.fn_mappings.insert(key_id.to_string(), act);
                }
            }
            change = Some(ConfigChange::Modified);
        }
    }

    // Reset key to default button
    if ui.button(i18n.t("editor.reset_key")).clicked()
        && let Some(profile) = config.active_profile_mut()
    {
        match layer {
            ActiveLayer::Normal => {
                profile.mappings.remove(key_id);
            }
            ActiveLayer::Fn => {
                profile.fn_mappings.remove(key_id);
            }
        }
        change = Some(ConfigChange::Modified);
    }

    change
}

/// Renders profile management modals (New, Duplicate, Rename, Delete confirmation).
fn render_profile_dialogs(
    ui: &mut Ui,
    state: &mut KeyboardViewState,
    config: &mut Config,
    i18n: &I18n,
) -> Option<ConfigChange> {
    let mut change = None;
    let mut close_dialog = false;

    match &mut state.dialog {
        ProfileDialog::None => {}
        ProfileDialog::New { name, layout } => {
            ui.group(|ui| {
                ui.heading(i18n.t("profile.create_title"));
                ui.horizontal(|ui| {
                    ui.label(i18n.t("profile.name_prompt"));
                    ui.text_edit_singleline(name);
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.t("profile.layout"));
                    egui::ComboBox::from_id_salt("dialog_new_layout")
                        .selected_text(format!("{layout}"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(layout, KeyboardLayout::IsoItalian, "ISO Italian");
                            ui.selectable_value(layout, KeyboardLayout::AnsiUs, "ANSI US");
                            ui.selectable_value(layout, KeyboardLayout::IsoUk, "ISO UK");
                            ui.selectable_value(layout, KeyboardLayout::IsoGerman, "ISO German");
                            ui.selectable_value(layout, KeyboardLayout::IsoFrench, "ISO French");
                        });
                });

                if let Some(ref err) = state.dialog_error {
                    ui.colored_label(Color32::RED, err);
                }

                ui.horizontal(|ui| {
                    if ui.button(i18n.t("profile.ok")).clicked() {
                        let trimmed = name.trim().to_string();
                        if trimmed.is_empty() || config.profiles.contains_key(&trimmed) {
                            state.dialog_error = Some(i18n.t("profile.error_exists").to_string());
                        } else {
                            let mut new_prof = NamedProfile::new(trimmed.clone(), layout.clone());
                            // Initialize with standard missing keys
                            new_prof
                                .mappings
                                .insert("F13".to_string(), KeyAction::key("PrintScreen"));
                            new_prof
                                .mappings
                                .insert("F14".to_string(), KeyAction::key("ScrollLock"));
                            new_prof
                                .mappings
                                .insert("F15".to_string(), KeyAction::key("Pause"));
                            config.add_profile(new_prof);
                            let _ = config.set_active_profile(&trimmed);
                            close_dialog = true;
                            change = Some(ConfigChange::ProfileCreated(trimmed));
                        }
                    }
                    if ui.button(i18n.t("profile.cancel")).clicked() {
                        close_dialog = true;
                    }
                });
            });
        }
        ProfileDialog::Duplicate { name } => {
            ui.group(|ui| {
                ui.heading(i18n.t("profile.duplicate_title"));
                ui.horizontal(|ui| {
                    ui.label(i18n.t("profile.name_prompt"));
                    ui.text_edit_singleline(name);
                });

                if let Some(ref err) = state.dialog_error {
                    ui.colored_label(Color32::RED, err);
                }

                ui.horizontal(|ui| {
                    if ui.button(i18n.t("profile.ok")).clicked() {
                        let trimmed = name.trim().to_string();
                        match config
                            .duplicate_profile(&config.active_profile.clone(), trimmed.clone())
                        {
                            Ok(_) => {
                                let _ = config.set_active_profile(&trimmed);
                                close_dialog = true;
                                change = Some(ConfigChange::ProfileCreated(trimmed));
                            }
                            Err(_) => {
                                state.dialog_error =
                                    Some(i18n.t("profile.error_exists").to_string());
                            }
                        }
                    }
                    if ui.button(i18n.t("profile.cancel")).clicked() {
                        close_dialog = true;
                    }
                });
            });
        }
        ProfileDialog::Rename { name } => {
            ui.group(|ui| {
                ui.heading(i18n.t("profile.rename_title"));
                ui.horizontal(|ui| {
                    ui.label(i18n.t("profile.name_prompt"));
                    ui.text_edit_singleline(name);
                });

                if let Some(ref err) = state.dialog_error {
                    ui.colored_label(Color32::RED, err);
                }

                ui.horizontal(|ui| {
                    if ui.button(i18n.t("profile.ok")).clicked() {
                        let trimmed = name.trim().to_string();
                        let old_name = config.active_profile.clone();
                        match config.rename_profile(&old_name, trimmed.clone()) {
                            Ok(()) => {
                                close_dialog = true;
                                change = Some(ConfigChange::ActiveProfileChanged(trimmed));
                            }
                            Err(_) => {
                                state.dialog_error =
                                    Some(i18n.t("profile.error_exists").to_string());
                            }
                        }
                    }
                    if ui.button(i18n.t("profile.cancel")).clicked() {
                        close_dialog = true;
                    }
                });
            });
        }
        ProfileDialog::DeleteConfirm { name } => {
            ui.group(|ui| {
                ui.heading(i18n.t("profile.delete_title"));
                let confirm_text = i18n.t_format("profile.delete_confirm", &[("name", name)]);
                ui.label(confirm_text);

                if let Some(ref err) = state.dialog_error {
                    ui.colored_label(Color32::RED, err);
                }

                ui.horizontal(|ui| {
                    if ui.button(i18n.t("profile.ok")).clicked() {
                        let to_delete = name.clone();
                        // Find alternate profile to activate
                        if let Some(next_name) =
                            config.profiles.keys().find(|&k| k != &to_delete).cloned()
                        {
                            let _ = config.set_active_profile(&next_name);
                            let _ = config.delete_profile(&to_delete);
                            close_dialog = true;
                            change = Some(ConfigChange::ProfileDeleted(to_delete));
                        } else {
                            state.dialog_error =
                                Some(i18n.t("profile.error_cannot_delete_last").to_string());
                        }
                    }
                    if ui.button(i18n.t("profile.cancel")).clicked() {
                        close_dialog = true;
                    }
                });
            });
        }
    }

    if close_dialog {
        state.dialog = ProfileDialog::None;
    }

    change
}

/// Matches an egui `Key` to an A1843 physical key in the model for "Press a key to select".
fn find_matching_physical_key(key: egui::Key, model: &A1843Model) -> Option<&PhysicalKey> {
    use egui::Key as K;
    let target_id = match key {
        K::Escape => "Escape",
        K::F1 => "F1",
        K::F2 => "F2",
        K::F3 => "F3",
        K::F4 => "F4",
        K::F5 => "F5",
        K::F6 => "F6",
        K::F7 => "F7",
        K::F8 => "F8",
        K::F9 => "F9",
        K::F10 => "F10",
        K::F11 => "F11",
        K::F12 => "F12",
        K::F13 => "F13",
        K::F14 => "F14",
        K::F15 => "F15",
        K::F16 => "F16",
        K::F17 => "F17",
        K::F18 => "F18",
        K::F19 => "F19",
        K::Tab => "Tab",
        K::Backspace => "Backspace",
        K::Enter => "Enter",
        K::Space => "Space",
        K::Insert => "Fn",
        K::Delete => "Delete",
        K::Home => "Home",
        K::End => "End",
        K::PageUp => "PageUp",
        K::PageDown => "PageDown",
        K::ArrowLeft => "Left",
        K::ArrowRight => "Right",
        K::ArrowUp => "Up",
        K::ArrowDown => "Down",
        K::A => "A",
        K::B => "B",
        K::C => "C",
        K::D => "D",
        K::E => "E",
        K::F => "F",
        K::G => "G",
        K::H => "H",
        K::I => "I",
        K::J => "J",
        K::K => "K",
        K::L => "L",
        K::M => "M",
        K::N => "N",
        K::O => "O",
        K::P => "P",
        K::Q => "Q",
        K::R => "R",
        K::S => "S",
        K::T => "T",
        K::U => "U",
        K::V => "V",
        K::W => "W",
        K::X => "X",
        K::Y => "Y",
        K::Z => "Z",
        K::Num0 => "0",
        K::Num1 => "1",
        K::Num2 => "2",
        K::Num3 => "3",
        K::Num4 => "4",
        K::Num5 => "5",
        K::Num6 => "6",
        K::Num7 => "7",
        K::Num8 => "8",
        K::Num9 => "9",
        _ => return None,
    };

    model.find_by_id(target_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_view_state_defaults() {
        let state = KeyboardViewState::default();
        assert_eq!(state.active_layer, ActiveLayer::Normal);
        assert!(!state.listen_key_active);
        assert!(state.editor.selected_key_id.is_none());
        assert_eq!(state.dialog, ProfileDialog::None);
    }

    #[test]
    fn test_find_matching_physical_key() {
        let model = A1843Model::new();
        let key = find_matching_physical_key(egui::Key::F13, &model).expect("F13 found");
        assert_eq!(key.id, "F13");

        let key = find_matching_physical_key(egui::Key::Space, &model).expect("Space found");
        assert_eq!(key.id, "Space");
    }

    #[test]
    fn test_calculate_unit_size_responsive_clamping() {
        // Narrow/compact display: clamped to 20.0 minimum
        assert_eq!(calculate_unit_size(400.0), 20.0);
        assert_eq!(calculate_unit_size(480.0), 20.0);

        // Standard 1080p window width (~735px -> 30.0)
        let standard_unit = calculate_unit_size(735.0);
        assert!((standard_unit - 30.0).abs() < 0.01);

        // Wide display: 1200px -> clamped to 42.0 maximum
        assert_eq!(calculate_unit_size(1200.0), 42.0);
        // 4K Ultra-wide: clamped to 42.0 maximum
        assert_eq!(calculate_unit_size(3840.0), 42.0);
    }

    #[test]
    fn test_calculate_key_font_size_scaling() {
        // Compact unit_size = 20.0
        let compact_main = calculate_key_font_size(20.0, 1.0, KeyCluster::Main, 1);
        assert!((8.5..=16.0).contains(&compact_main));

        let compact_mod = calculate_key_font_size(20.0, 1.5, KeyCluster::Main, 7);
        assert!((7.0..=12.5).contains(&compact_mod));

        // 4K unit_size = 42.0
        let ultra_main = calculate_key_font_size(42.0, 1.0, KeyCluster::Main, 1);
        assert!(ultra_main >= 15.0);

        let ultra_mod = calculate_key_font_size(42.0, 1.5, KeyCluster::Main, 4);
        assert!(ultra_mod >= 13.0);
    }
}
