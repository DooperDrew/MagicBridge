//! Interactive step-by-step Hardware QA Key Verification Tool for Apple Magic Keyboard (A1843).
//!
//! Complies with Issue #10 Prep, F7.2, F7.4, and AGENTS.md:
//! - Visual matrix of all 109 physical keys on A1843.
//! - Captures window key events and Win32 modifier states.
//! - Turns verified keys bright green.
//! - Progress bar and category breakdowns (Modifiers, Nav, Number row, Function row, Numpad, Alphanumeric).
//! - Reset test and Export QA report to Desktop or AppData.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use egui::{
    Color32, CornerRadius, ProgressBar, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};
use magicbridge_core::key::*;

use crate::i18n::I18n;
use crate::keyboard_model::{A1843Model, KeyCluster, PhysicalKey};

/// Category classification for A1843 hardware keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QaCategory {
    /// Modifiers: Command, Option, Control, Shift (Left and Right).
    Modifiers,
    /// Function Row: Escape, F1..F12, F13..F15, F16..F19.
    FunctionRow,
    /// Number Row: Grave (~), 1..0, Minus, Equals, Backspace.
    NumberRow,
    /// Navigation Cluster: Fn, Home, End, PageUp, PageDown, Delete, Arrows.
    Navigation,
    /// Numeric Keypad: Clear, =, /, *, 0..9, +, -, Enter, Decimal.
    Numpad,
    /// Alphanumeric Typing Block: Letters A-Z, punctuation, Tab, CapsLock, Enter, Space.
    Alphanumeric,
}

impl QaCategory {
    /// Returns all QA categories in logical display order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Modifiers,
            Self::FunctionRow,
            Self::NumberRow,
            Self::Navigation,
            Self::Numpad,
            Self::Alphanumeric,
        ]
    }

    /// Translation key for the category name.
    #[must_use]
    pub const fn label_key(&self) -> &'static str {
        match self {
            Self::Modifiers => "qa.cat_modifiers",
            Self::FunctionRow => "qa.cat_function_row",
            Self::NumberRow => "qa.cat_number_row",
            Self::Navigation => "qa.cat_navigation",
            Self::Numpad => "qa.cat_numpad",
            Self::Alphanumeric => "qa.cat_alphanumeric",
        }
    }
}

/// Categorizes a physical key on the A1843 keyboard.
#[must_use]
pub fn categorize_key(key: &PhysicalKey) -> QaCategory {
    match key.id {
        // Modifiers (Cmd, Opt, Ctrl, Shift)
        "LeftShift" | "RightShift" | "LeftControl" | "RightControl" | "LeftOption"
        | "RightOption" | "LeftCommand" | "RightCommand" => QaCategory::Modifiers,

        // Function Row: Esc + F1..F19
        "Escape" | "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11"
        | "F12" | "F13" | "F14" | "F15" | "F16" | "F17" | "F18" | "F19" => QaCategory::FunctionRow,

        // Number Row: Grave, 1..0, Minus, Equals, Backspace
        "Grave" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "0" | "Minus"
        | "Equals" | "Backspace" => QaCategory::NumberRow,

        // Navigation cluster: Fn, Home/End, PgUp/PgDn, Delete, Arrows
        "Fn" | "Home" | "PageUp" | "Delete" | "End" | "PageDown" | "Up" | "Left" | "Down"
        | "Right" => QaCategory::Navigation,

        // Numpad: Clear, arithmetic, digits, Enter, Decimal
        "Clear" | "NumEquals" | "NumDivide" | "NumMultiply" | "Num7" | "Num8" | "Num9"
        | "NumSubtract" | "Num4" | "Num5" | "Num6" | "NumAdd" | "Num1" | "Num2" | "Num3"
        | "NumEnter" | "Num0" | "NumDecimal" => QaCategory::Numpad,

        // Main alphanumeric block
        _ => QaCategory::Alphanumeric,
    }
}

/// A recorded keypress event during QA verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaKeyEvent {
    /// Unique key ID string from `A1843Model`.
    pub key_id: String,
    /// Canonical key name.
    pub key_name: String,
    /// Windows Virtual Key code.
    pub vk: u16,
    /// Hardware scancode.
    pub scancode: u16,
    /// Extended key flag.
    pub extended: bool,
    /// Milliseconds since the start of this QA test session.
    pub elapsed_ms: u128,
    /// User-friendly relative time string (e.g. "+1.42s").
    pub formatted_time: String,
    /// Category classification.
    pub category: QaCategory,
}

/// State for the interactive Hardware QA Key Verification view.
#[derive(Debug, Clone)]
pub struct HardwareQaViewState {
    /// Set of physical key IDs that have been verified during this session.
    pub tested_keys: HashSet<String>,
    /// Total key count on the physical keyboard (109 keys on A1843).
    pub total_keys: usize,
    /// Timestamp when this QA test session started.
    pub test_started: Instant,
    /// Event log of recently pressed keys in reverse chronological order (newest first).
    pub event_log: VecDeque<QaKeyEvent>,
    /// Whether the key listener is currently active.
    pub is_listening: bool,
    /// Keyboard geometry and key definitions.
    pub model: A1843Model,
    /// Status of the last export attempt: `(is_error, message)`.
    pub export_status: Option<(bool, String)>,
    /// Active category filter for highlighting (None = show all).
    pub selected_category: Option<QaCategory>,
}

impl Default for HardwareQaViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareQaViewState {
    /// Maximum number of events kept in the in-memory recent event log.
    pub const MAX_LOG_EVENTS: usize = 300;

    /// Creates and initializes a new Hardware QA view state.
    #[must_use]
    pub fn new() -> Self {
        let model = A1843Model::new();
        let total_keys = model.keys.len();

        Self {
            tested_keys: HashSet::with_capacity(total_keys),
            total_keys,
            test_started: Instant::now(),
            event_log: VecDeque::with_capacity(Self::MAX_LOG_EVENTS),
            is_listening: true,
            model,
            export_status: None,
            selected_category: None,
        }
    }

    /// Resets the QA test session, clearing verified keys and event log.
    pub fn reset(&mut self) {
        self.tested_keys.clear();
        self.event_log.clear();
        self.test_started = Instant::now();
        self.export_status = None;
        self.selected_category = None;
    }

    /// Marks a physical key as verified and logs the keypress event.
    pub fn mark_key_verified(&mut self, key_id: &str, vk: u16, scancode: u16, extended: bool) {
        let key_id_owned = key_id.to_string();
        self.tested_keys.insert(key_id_owned.clone());

        let elapsed = self.test_started.elapsed();
        let elapsed_ms = elapsed.as_millis();
        let formatted_time = format!("+{:.2}s", elapsed.as_secs_f32());

        let category = self
            .model
            .find_by_id(key_id)
            .map_or(QaCategory::Alphanumeric, categorize_key);

        let key_name = if vk != 0 {
            vk_name(vk).to_string()
        } else {
            key_id.to_string()
        };

        let event = QaKeyEvent {
            key_id: key_id_owned,
            key_name,
            vk,
            scancode,
            extended,
            elapsed_ms,
            formatted_time,
            category,
        };

        self.event_log.push_front(event);
        if self.event_log.len() > Self::MAX_LOG_EVENTS {
            self.event_log.pop_back();
        }
    }

    /// Returns the verified key count and completion percentage.
    #[must_use]
    pub fn progress(&self) -> (usize, usize, f32) {
        let verified = self.tested_keys.len();
        let total = self.total_keys;
        let percentage = if total > 0 {
            (verified as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        (verified, total, percentage)
    }

    /// Returns the count of verified keys within a specific category.
    #[must_use]
    pub fn category_progress(&self, category: QaCategory) -> (usize, usize) {
        let total = self
            .model
            .keys
            .iter()
            .filter(|k| categorize_key(k) == category)
            .count();
        let verified = self
            .model
            .keys
            .iter()
            .filter(|k| categorize_key(k) == category && self.tested_keys.contains(k.id))
            .count();
        (verified, total)
    }
}

/// Matches an `egui::Key` (and optional physical key fallback) to an A1843 physical key ID.
#[must_use]
pub fn map_egui_key_to_a1843_id(
    key: egui::Key,
    physical: Option<egui::Key>,
) -> Option<&'static str> {
    use egui::Key as K;

    let id = match key {
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
        K::Backtick => "Grave",
        K::Minus => "Minus",
        K::Equals => "Equals",
        K::OpenBracket => "LBracket",
        K::CloseBracket => "RBracket",
        K::Backslash => "Backslash",
        K::Semicolon => "Semicolon",
        K::Quote => "Quote",
        K::Comma => "Comma",
        K::Period => "Period",
        K::Slash => "Slash",
        K::Plus => "NumAdd",
        _ => {
            if let Some(phys) = physical {
                return map_egui_key_to_a1843_id(phys, None);
            }
            return None;
        }
    };

    Some(id)
}

/// Matches input text characters (e.g. on ISO keyboards) to physical key IDs.
#[must_use]
pub fn map_text_to_a1843_id(text: &str) -> Option<&'static str> {
    match text {
        "<" | ">" => Some("IsoLessGreater"),
        "\\" | "|" => Some("Grave"),
        "à" | "°" | "#" => Some("Quote"),
        "è" | "é" | "[" | "{" => Some("LBracket"),
        "ò" | "ç" | "@" => Some("Semicolon"),
        "ù" | "§" => Some("RBracket"),
        "+" | "*" => Some("NumAdd"),
        "-" | "_" => Some("Minus"),
        "=" => Some("Equals"),
        _ => None,
    }
}

/// Polls physical modifier keys using Win32 `GetAsyncKeyState` to distinguish left/right modifiers.
#[cfg(windows)]
fn poll_win32_modifiers_and_numpad(
    state: &mut HardwareQaViewState,
    recorded_this_frame: &mut HashSet<String>,
) {
    // Modifier VKs to check
    const MODIFIER_VKS: &[(u16, &str)] = &[
        (VK_LSHIFT, "LeftShift"),
        (VK_RSHIFT, "RightShift"),
        (VK_LCONTROL, "LeftControl"),
        (VK_RCONTROL, "RightControl"),
        (VK_LMENU, "LeftOption"),
        (VK_RMENU, "RightOption"),
        (VK_LWIN, "LeftCommand"),
        (VK_RWIN, "RightCommand"),
        (VK_CAPITAL, "CapsLock"),
    ];

    for &(vk, id) in MODIFIER_VKS {
        // SAFETY: Calling GetAsyncKeyState with a standard virtual key code is safe and has no side effects.
        let key_state =
            unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32) };
        let is_down = (key_state as u16 & 0x8000) != 0;
        let was_pressed = (key_state as u16 & 0x0001) != 0;

        if (is_down || was_pressed) && !recorded_this_frame.contains(id) {
            recorded_this_frame.insert(id.to_string());
            let (scancode, ext) = default_scancode_and_extended(vk);
            state.mark_key_verified(id, vk, scancode, ext);
        }
    }

    // Numpad VKs to distinguish from number row
    const NUMPAD_VKS: &[(u16, &str)] = &[
        (VK_NUMPAD0, "Num0"),
        (VK_NUMPAD1, "Num1"),
        (VK_NUMPAD2, "Num2"),
        (VK_NUMPAD3, "Num3"),
        (VK_NUMPAD4, "Num4"),
        (VK_NUMPAD5, "Num5"),
        (VK_NUMPAD6, "Num6"),
        (VK_NUMPAD7, "Num7"),
        (VK_NUMPAD8, "Num8"),
        (VK_NUMPAD9, "Num9"),
        (VK_DECIMAL, "NumDecimal"),
        (VK_ADD, "NumAdd"),
        (VK_SUBTRACT, "NumSubtract"),
        (VK_MULTIPLY, "NumMultiply"),
        (VK_DIVIDE, "NumDivide"),
        (VK_NUMLOCK, "Clear"),
    ];

    for &(vk, id) in NUMPAD_VKS {
        // SAFETY: Calling GetAsyncKeyState with a standard virtual key code is safe and has no side effects.
        let key_state =
            unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32) };
        let is_down = (key_state as u16 & 0x8000) != 0;
        let was_pressed = (key_state as u16 & 0x0001) != 0;

        if (is_down || was_pressed) && !recorded_this_frame.contains(id) {
            recorded_this_frame.insert(id.to_string());
            let (scancode, ext) = default_scancode_and_extended(vk);
            state.mark_key_verified(id, vk, scancode, ext);
        }
    }

    // ISO angle brackets key (< >)
    // SAFETY: Calling GetAsyncKeyState with standard virtual key code is safe.
    let iso_state =
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(VK_OEM_102 as i32) };
    if (iso_state as u16 & 0x8000) != 0 || (iso_state as u16 & 0x0001) != 0 {
        let id = "IsoLessGreater";
        if !recorded_this_frame.contains(id) {
            recorded_this_frame.insert(id.to_string());
            let (scancode, ext) = default_scancode_and_extended(VK_OEM_102);
            state.mark_key_verified(id, VK_OEM_102, scancode, ext);
        }
    }
}

/// Fallback modifier poller for non-Windows platforms.
#[cfg(not(windows))]
fn poll_win32_modifiers_and_numpad(
    _state: &mut HardwareQaViewState,
    _recorded_this_frame: &mut HashSet<String>,
) {
    // Non-Windows fallback: no-op, relying on egui events.
}

/// Formats the current UTC timestamp string for reports (YYYYMMDD-HHMMSS).
#[must_use]
pub fn current_timestamp_string() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let s = rem_secs % 60;

    let mut year = 1970;
    let mut d = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        year += 1;
    }

    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &dim in &days_in_months {
        if d < dim {
            break;
        }
        d -= dim;
        month += 1;
    }
    let day = d + 1;

    format!("{year:04}{month:02}{day:02}-{hours:02}{mins:02}{s:02}")
}

/// Generates a comprehensive QA report and saves it to a specified directory.
pub fn export_qa_report_to_dir(
    state: &HardwareQaViewState,
    i18n: &I18n,
    target_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let ts = current_timestamp_string();
    let filename_txt = format!("qa-hardware-report-{ts}.txt");
    let filename_json = format!("qa-hardware-report-{ts}.json");

    if !target_dir.exists() {
        let _ = fs::create_dir_all(target_dir);
    }

    let report_path_txt = target_dir.join(filename_txt);
    let report_path_json = target_dir.join(filename_json);

    let (verified, total, pct) = state.progress();

    // 1. Generate text report
    let mut txt = String::new();
    txt.push_str("=================================================================\n");
    txt.push_str(" MAGICBRIDGE HARDWARE QA KEY VERIFICATION REPORT\n");
    txt.push_str("=================================================================\n\n");
    txt.push_str(&format!("Report Generated : {ts} (UTC)\n"));
    txt.push_str("Hardware Model   : Apple Magic Keyboard with Numeric Keypad (A1843)\n");
    txt.push_str(&format!(
        "Test Duration    : {:.2}s\n",
        state.test_started.elapsed().as_secs_f32()
    ));
    txt.push_str(&format!(
        "Verification Score: {verified} / {total} keys verified ({pct:.1}%)\n"
    ));
    txt.push_str(&format!(
        "Overall Status   : {}\n\n",
        if verified == total {
            "PASSED (100% COMPLETE)"
        } else {
            "IN PROGRESS"
        }
    ));

    txt.push_str("-----------------------------------------------------------------\n");
    txt.push_str(" CATEGORY BREAKDOWNS\n");
    txt.push_str("-----------------------------------------------------------------\n");
    for &cat in QaCategory::all() {
        let (v, t) = state.category_progress(cat);
        let cat_pct = if t > 0 {
            (v as f32 / t as f32) * 100.0
        } else {
            0.0
        };
        let status = if v == t { "[PASSED]" } else { "[PENDING]" };
        txt.push_str(&format!(
            "{:<35} : {:2} / {:2} keys ({:5.1}%) {}\n",
            i18n.t(cat.label_key()),
            v,
            t,
            cat_pct,
            status
        ));
    }
    txt.push('\n');

    // List unverified keys if any
    let unverified: Vec<_> = state
        .model
        .keys
        .iter()
        .filter(|k| !state.tested_keys.contains(k.id))
        .collect();

    if unverified.is_empty() {
        txt.push_str("All 109 keys successfully tested and verified.\n\n");
    } else {
        txt.push_str("-----------------------------------------------------------------\n");
        txt.push_str(&format!(" UNVERIFIED KEYS ({})\n", unverified.len()));
        txt.push_str("-----------------------------------------------------------------\n");
        for key in &unverified {
            let cat = categorize_key(key);
            txt.push_str(&format!(
                "- {:<16} (Default: {:<8}, VK: 0x{:02X}, Cluster: {:?}, Category: {:?})\n",
                key.id, key.default_label, key.vk, key.cluster, cat
            ));
        }
        txt.push('\n');
    }

    // Chronological event log
    txt.push_str("-----------------------------------------------------------------\n");
    txt.push_str(&format!(
        " EVENT LOG ({} recent entries)\n",
        state.event_log.len()
    ));
    txt.push_str("-----------------------------------------------------------------\n");
    txt.push_str(
        "Idx | Time    | Key Name        | ID              | VK Code | Scancode | Extended\n",
    );
    txt.push_str(
        "----+---------+-----------------+-----------------+---------+----------+---------\n",
    );
    for (i, ev) in state.event_log.iter().enumerate() {
        txt.push_str(&format!(
            "{:3} | {:<7} | {:<15} | {:<15} | 0x{:02X}    | 0x{:02X}     | {}\n",
            i + 1,
            ev.formatted_time,
            ev.key_name,
            ev.key_id,
            ev.vk,
            ev.scancode,
            ev.extended
        ));
    }

    fs::write(&report_path_txt, txt).map_err(|e| format!("Failed to write text report: {e}"))?;

    // 2. Generate JSON report for programmatic ingestion
    let json_events: Vec<serde_json::Value> = state
        .event_log
        .iter()
        .map(|ev| {
            serde_json::json!({
                "key_id": ev.key_id,
                "key_name": ev.key_name,
                "vk": ev.vk,
                "scancode": ev.scancode,
                "extended": ev.extended,
                "elapsed_ms": ev.elapsed_ms,
                "formatted_time": ev.formatted_time,
                "category": format!("{:?}", ev.category),
            })
        })
        .collect();

    let json_data = serde_json::json!({
        "timestamp": ts,
        "keyboard_model": "A1843",
        "verified_keys": verified,
        "total_keys": total,
        "completion_percentage": pct,
        "status": if verified == total { "PASSED" } else { "IN_PROGRESS" },
        "tested_keys": state.tested_keys.iter().collect::<Vec<_>>(),
        "unverified_keys": unverified.iter().map(|k| k.id).collect::<Vec<_>>(),
        "events": json_events,
    });

    if let Ok(json_str) = serde_json::to_string_pretty(&json_data) {
        let _ = fs::write(&report_path_json, json_str);
    }

    Ok(report_path_txt)
}

/// Generates a comprehensive QA report and saves it to Desktop or AppData.
pub fn export_qa_report(state: &HardwareQaViewState, i18n: &I18n) -> Result<PathBuf, String> {
    // Resolve target directory: Desktop -> AppData -> Current Dir
    let target_dir = std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Desktop"))
        .ok()
        .filter(|p| p.is_dir())
        .or_else(|| {
            std::env::var("APPDATA")
                .map(|p| PathBuf::from(p).join("MagicBridge"))
                .ok()
        })
        .unwrap_or_else(|| PathBuf::from("."));

    export_qa_report_to_dir(state, i18n, &target_dir)
}

/// Renders the visual A1843 QA keyboard matrix.
fn render_qa_visual_keyboard(ui: &mut Ui, state: &mut HardwareQaViewState) {
    let available_width = ui.available_width().min(ui.max_rect().width());
    let total_units_x = 23.5f32;
    let unit_size = ((available_width - 8.0) / total_units_x).clamp(16.0, 34.0);
    let key_height = unit_size * 0.95;
    let key_spacing = 2.0;
    let total_matrix_width = (total_units_x * (unit_size + key_spacing)) + 8.0;
    let total_height = (6.0 * (key_height + key_spacing)) + 8.0;

    let (rect, _response) = ui.allocate_exact_size(
        Vec2::new(available_width.min(total_matrix_width), total_height),
        Sense::hover(),
    );

    let painter = ui.painter_at(rect);

    // Draw keyboard base plate
    painter.rect_filled(
        rect,
        CornerRadius::same(6),
        Color32::from_rgb(235, 237, 240),
    );

    let mut clicked_key: Option<PhysicalKey> = None;

    // Draw each physical key
    for key in &state.model.keys {
        let key_x = rect.left() + (key.x * (unit_size + key_spacing)) + 4.0;
        let key_y = rect.top() + (key.row as f32 * (key_height + key_spacing)) + 4.0;
        let key_w = (key.width * unit_size) + ((key.width - 1.0) * key_spacing);
        let key_h = (key.height * key_height) + ((key.height - 1.0) * key_spacing);

        let key_rect = Rect::from_min_size(egui::pos2(key_x, key_y), Vec2::new(key_w, key_h));

        let id = ui.make_persistent_id(format!("qa_key_{}", key.id));
        let mut response = ui.interact(key_rect, id, Sense::click());
        let label = key.default_label;
        response
            .widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        response = response.on_hover_text(format!("Key: {} (VK: {:#04x})", label, key.vk));

        if response.clicked() {
            clicked_key = Some(key.clone());
        }

        let is_verified = state.tested_keys.contains(key.id);
        let key_cat = categorize_key(key);
        let is_in_selected_cat = state.selected_category.is_some_and(|cat| cat == key_cat);

        // Styling: Bright green for verified keys!
        let fill_color = if is_verified {
            Color32::from_rgb(46, 204, 113) // Bright verified green
        } else if is_in_selected_cat {
            Color32::from_rgb(255, 243, 205) // Highlighted category
        } else if response.hovered() {
            Color32::from_rgb(240, 244, 248)
        } else {
            Color32::WHITE
        };

        let stroke_color = if is_verified {
            Color32::from_rgb(25, 135, 65)
        } else if is_in_selected_cat {
            Color32::from_rgb(220, 140, 0)
        } else {
            Color32::from_rgb(195, 200, 205)
        };

        let stroke_width = if is_verified || is_in_selected_cat {
            1.5_f32
        } else {
            1.0_f32
        };

        painter.rect(
            key_rect,
            CornerRadius::same(3),
            fill_color,
            Stroke::new(stroke_width, stroke_color),
            egui::StrokeKind::Inside,
        );

        // Key label
        let text_color = if is_verified {
            Color32::WHITE
        } else {
            Color32::from_rgb(45, 50, 55)
        };

        let font_size = if key.width >= 1.5
            || key.cluster == KeyCluster::Main && key.default_label.len() <= 2
        {
            11.0
        } else {
            9.5
        };

        let label_text = if is_verified {
            format!("{}\n✓", key.default_label)
        } else {
            key.default_label.to_string()
        };

        painter.text(
            key_rect.center(),
            egui::Align2::CENTER_CENTER,
            label_text,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }

    // Handle manual click to verify
    if let Some(key) = clicked_key {
        let (sc, ext) = default_scancode_and_extended(key.vk);
        state.mark_key_verified(key.id, key.vk, sc, ext);
    }
}

/// Renders the complete Hardware QA View tab.
pub fn show(ui: &mut Ui, state: &mut HardwareQaViewState, i18n: &I18n) {
    let mut recorded_this_frame = HashSet::new();

    // 1. Keyboard event listener mode
    if state.is_listening {
        // Capture egui keypress events from window
        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Key {
                        key,
                        physical_key,
                        pressed: true,
                        ..
                    } => {
                        if let Some(id) = map_egui_key_to_a1843_id(*key, *physical_key)
                            && !recorded_this_frame.contains(id)
                        {
                            recorded_this_frame.insert(id.to_string());
                            let (vk, scancode, ext) = state
                                .model
                                .find_by_id(id)
                                .map(|k| {
                                    let (sc, e) = default_scancode_and_extended(k.vk);
                                    (k.vk, sc, e)
                                })
                                .unwrap_or((0, 0, false));
                            state.mark_key_verified(id, vk, scancode, ext);
                        }
                    }
                    egui::Event::Text(text) => {
                        if let Some(id) = map_text_to_a1843_id(text)
                            && !recorded_this_frame.contains(id)
                        {
                            recorded_this_frame.insert(id.to_string());
                            let (vk, scancode, ext) = state
                                .model
                                .find_by_id(id)
                                .map(|k| {
                                    let (sc, e) = default_scancode_and_extended(k.vk);
                                    (k.vk, sc, e)
                                })
                                .unwrap_or((0, 0, false));
                            state.mark_key_verified(id, vk, scancode, ext);
                        }
                    }
                    _ => {}
                }
            }
        });

        // Query Win32 modifier and numpad states to unambiguously detect Left vs Right keys
        poll_win32_modifiers_and_numpad(state, &mut recorded_this_frame);
    }

    // 2. Header and Controls Toolbar
    ui.horizontal(|ui| {
        ui.heading(i18n.t("qa.title"));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Export QA Log button
            if ui
                .button(format!("📄 {}", i18n.t("qa.export_button")))
                .clicked()
            {
                match export_qa_report(state, i18n) {
                    Ok(path) => {
                        let path_str = path.to_string_lossy().to_string();
                        state.export_status = Some((
                            false,
                            i18n.t_format("qa.export_success", &[("path", &path_str)]),
                        ));
                    }
                    Err(err) => {
                        state.export_status =
                            Some((true, i18n.t_format("qa.export_failed", &[("error", &err)])));
                    }
                }
            }

            // Reset Test button
            if ui
                .button(format!("🔄 {}", i18n.t("qa.reset_button")))
                .clicked()
            {
                state.reset();
            }

            // Listen Mode toggle
            let listen_label = if state.is_listening {
                format!("👂 {}", i18n.t("qa.listen_active"))
            } else {
                format!("⏸ {}", i18n.t("qa.listen_paused"))
            };
            ui.toggle_value(&mut state.is_listening, listen_label);
        });
    });

    ui.label(
        RichText::new(i18n.t("qa.subtitle"))
            .color(Color32::from_rgb(110, 115, 120))
            .small(),
    );

    // Export banner feedback
    if let Some((is_error, ref msg)) = state.export_status {
        let (bg, fg) = if is_error {
            (
                Color32::from_rgb(255, 235, 235),
                Color32::from_rgb(180, 40, 30),
            )
        } else {
            (
                Color32::from_rgb(235, 250, 240),
                Color32::from_rgb(25, 135, 65),
            )
        };
        egui::Frame::NONE
            .fill(bg)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.colored_label(fg, msg);
            });
    }

    ui.add_space(4.0);

    // 3. Progress Bar and Counter
    let (verified, total, pct) = state.progress();
    let is_all_verified = verified == total && total > 0;

    ui.horizontal(|ui| {
        let count_text = if is_all_verified {
            RichText::new(i18n.t("qa.progress_all_verified"))
                .strong()
                .color(Color32::from_rgb(25, 135, 65))
        } else {
            let v_str = verified.to_string();
            let t_str = total.to_string();
            let p_str = format!("{pct:.0}");
            RichText::new(i18n.t_format(
                "qa.progress",
                &[("verified", &v_str), ("total", &t_str), ("percent", &p_str)],
            ))
            .strong()
        };
        ui.label(count_text);
    });

    let progress_bar = ProgressBar::new(if total > 0 {
        verified as f32 / total as f32
    } else {
        0.0
    })
    .show_percentage()
    .animate(state.is_listening);
    ui.add(progress_bar);

    ui.add_space(6.0);

    // 4. Category Breakdown Cards Bar (wraps responsibly to prevent clipping)
    ui.horizontal_wrapped(|ui| {
        let is_all_selected = state.selected_category.is_none();
        if ui
            .selectable_label(is_all_selected, i18n.t("qa.category_all"))
            .clicked()
        {
            state.selected_category = None;
        }

        for &cat in QaCategory::all() {
            let (v, t) = state.category_progress(cat);
            let is_selected = state.selected_category == Some(cat);
            let badge = if v == t && t > 0 {
                format!("{} ({v}/{t} ✓)", i18n.t(cat.label_key()))
            } else {
                format!("{} ({v}/{t})", i18n.t(cat.label_key()))
            };

            let text = if is_selected {
                RichText::new(badge).strong()
            } else if v == t && t > 0 {
                RichText::new(badge).color(Color32::from_rgb(25, 135, 65))
            } else {
                RichText::new(badge)
            };

            if ui.selectable_label(is_selected, text).clicked() {
                state.selected_category = if is_selected { None } else { Some(cat) };
            }
        }
    });

    ui.add_space(4.0);

    // 5. Visual A1843 Keyboard Canvas
    render_qa_visual_keyboard(ui, state);

    // Informational note about physical Fn key stripped by kbdhid.sys
    ui.label(
        RichText::new(i18n.t("qa.fn_note"))
            .color(Color32::from_rgb(130, 135, 140))
            .small(),
    );

    ui.separator();

    // 6. Recent Keypress Events Log Table
    ui.horizontal(|ui| {
        ui.strong(i18n.t("qa.recent_events"));
        ui.label(format!("({} events)", state.event_log.len()));
    });

    if state.event_log.is_empty() {
        ui.label(
            RichText::new(i18n.t("qa.no_events"))
                .color(Color32::from_rgb(120, 125, 130))
                .italics(),
        );
    } else {
        ScrollArea::vertical()
            .max_height(140.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("qa_event_log_grid")
                    .striped(true)
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.strong(i18n.t("qa.col_time"));
                        ui.strong(i18n.t("qa.col_key"));
                        ui.strong(i18n.t("qa.col_id"));
                        ui.strong(i18n.t("qa.col_vk"));
                        ui.strong(i18n.t("qa.col_scancode"));
                        ui.strong(i18n.t("qa.col_category"));
                        ui.end_row();

                        for ev in state.event_log.iter().take(30) {
                            ui.label(&ev.formatted_time);
                            ui.colored_label(Color32::from_rgb(25, 135, 65), &ev.key_name);
                            ui.label(&ev.key_id);
                            ui.label(format!("0x{:02X}", ev.vk));
                            ui.label(format!(
                                "0x{:02X}{}",
                                ev.scancode,
                                if ev.extended { " (Ext)" } else { "" }
                            ));
                            ui.label(i18n.t(ev.category.label_key()));
                            ui.end_row();
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qa_state_defaults() {
        let state = HardwareQaViewState::new();
        assert_eq!(state.total_keys, 109);
        assert_eq!(state.tested_keys.len(), 0);
        assert!(state.is_listening);
        assert!(state.event_log.is_empty());

        let (verified, total, pct) = state.progress();
        assert_eq!(verified, 0);
        assert_eq!(total, 109);
        assert!((pct - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_all_109_keys_categorized_without_overlap() {
        let state = HardwareQaViewState::new();
        assert_eq!(state.model.keys.len(), 109);

        let mut counts = std::collections::HashMap::new();
        for key in &state.model.keys {
            let cat = categorize_key(key);
            *counts.entry(cat).or_insert(0) += 1;
        }

        // Verify counts per category
        assert_eq!(*counts.get(&QaCategory::Modifiers).unwrap_or(&0), 8);
        assert_eq!(*counts.get(&QaCategory::FunctionRow).unwrap_or(&0), 20);
        assert_eq!(*counts.get(&QaCategory::NumberRow).unwrap_or(&0), 14);
        assert_eq!(*counts.get(&QaCategory::Navigation).unwrap_or(&0), 10);
        assert_eq!(*counts.get(&QaCategory::Numpad).unwrap_or(&0), 18);
        assert_eq!(*counts.get(&QaCategory::Alphanumeric).unwrap_or(&0), 39);

        let total_sum: usize = counts.values().sum();
        assert_eq!(total_sum, 109);
    }

    #[test]
    fn test_mark_key_verified_and_progress() {
        let mut state = HardwareQaViewState::new();

        state.mark_key_verified("F13", VK_F13, 0x64, false);
        assert_eq!(state.tested_keys.len(), 1);
        assert!(state.tested_keys.contains("F13"));
        assert_eq!(state.event_log.len(), 1);

        let ev = &state.event_log[0];
        assert_eq!(ev.key_id, "F13");
        assert_eq!(ev.vk, VK_F13);
        assert_eq!(ev.category, QaCategory::FunctionRow);

        let (verified, total, pct) = state.progress();
        assert_eq!(verified, 1);
        assert_eq!(total, 109);
        assert!(pct > 0.9 && pct < 1.0);
    }

    #[test]
    fn test_qa_state_reset() {
        let mut state = HardwareQaViewState::new();
        state.mark_key_verified("Space", VK_SPACE, 0x39, false);
        state.mark_key_verified("Enter", VK_RETURN, 0x1C, false);
        assert_eq!(state.tested_keys.len(), 2);

        state.reset();
        assert_eq!(state.tested_keys.len(), 0);
        assert_eq!(state.event_log.len(), 0);
        assert!(state.export_status.is_none());
    }

    #[test]
    fn test_map_egui_key_and_text() {
        assert_eq!(
            map_egui_key_to_a1843_id(egui::Key::Escape, None),
            Some("Escape")
        );
        assert_eq!(map_egui_key_to_a1843_id(egui::Key::F19, None), Some("F19"));
        assert_eq!(
            map_egui_key_to_a1843_id(egui::Key::Space, None),
            Some("Space")
        );
        assert_eq!(map_egui_key_to_a1843_id(egui::Key::A, None), Some("A"));

        assert_eq!(map_text_to_a1843_id("<"), Some("IsoLessGreater"));
        assert_eq!(map_text_to_a1843_id(">"), Some("IsoLessGreater"));
        assert_eq!(map_text_to_a1843_id("à"), Some("Quote"));
    }

    #[test]
    fn test_current_timestamp_format() {
        let ts = current_timestamp_string();
        assert_eq!(ts.len(), 15); // YYYYMMDD-HHMMSS
        assert!(ts.contains('-'));
    }

    #[test]
    fn test_export_qa_report() {
        let mut state = HardwareQaViewState::new();
        state.mark_key_verified("Escape", VK_ESCAPE, 0x01, false);
        state.mark_key_verified("F13", VK_F13, 0x64, false);

        let i18n = I18n::new("en");
        let temp_dir = std::env::temp_dir().join(format!(
            "magicbridge-qa-test-{}",
            current_timestamp_string()
        ));
        let res = export_qa_report_to_dir(&state, &i18n, &temp_dir);
        assert!(res.is_ok());

        let report_path = res.expect("report exported");
        assert!(report_path.exists());

        let txt = fs::read_to_string(&report_path).expect("read report");
        assert!(txt.contains("MAGICBRIDGE HARDWARE QA KEY VERIFICATION REPORT"));
        assert!(txt.contains("Escape"));
        assert!(txt.contains("F13"));
        assert!(txt.contains("2 / 109 keys verified"));

        // Check json file was also generated
        let json_path = report_path.with_extension("json");
        assert!(json_path.exists());
        let json_content = fs::read_to_string(&json_path).expect("read json");
        assert!(json_content.contains("\"keyboard_model\": \"A1843\""));
        assert!(json_content.contains("\"verified_keys\": 2"));

        // Clean up
        let _ = fs::remove_file(&report_path);
        let _ = fs::remove_file(&json_path);
        let _ = fs::remove_dir(&temp_dir);
    }
}
