//! Virtual key models, Windows virtual key constants, and scancode mappings.
//!
//! Provides platform-independent representations of keyboard keys, virtual key codes (VK),
//! hardware scancodes, extended-key flags, and the MagicBridge synthetic event sentinel.
//!
//! Conforms to `NFR-Q1` (pure safe Rust, `#![forbid(unsafe_code)]`, zero Win32 crate dependencies).

use core::fmt;
use serde::{Deserialize, Serialize};

/// Distinct `dwExtraInfo` sentinel marker injected into all synthetic keyboard events.
///
/// Encodes ASCII `"MBGE"` (`0x4D424745`). The `WH_KEYBOARD_LL` low-level hook checks
/// `dwExtraInfo == MAGICBRIDGE_EXTRA_INFO` to immediately pass through its own events
/// and prevent infinite feedback loops.
pub const MAGICBRIDGE_EXTRA_INFO: usize = 0x4D42_4745;

/// Hardware scancode synthesized by Windows for Left Control (~0.8 ms before Right Option)
/// on ISO keyboard layouts equipped with `AltGr` (e.g. Italian ISO A1843).
///
/// See `docs/hardware-findings.md` §4.1.
pub const SC_ISO_ALTGR_SYNTHETIC_LCTRL: u16 = 0x021D;

// ============================================================================
// Standard Windows Virtual Key Codes (VK_*)
// ============================================================================

// Modifiers
pub const VK_LSHIFT: u16 = 0xA0;
pub const VK_RSHIFT: u16 = 0xA1;
pub const VK_LCONTROL: u16 = 0xA2;
pub const VK_RCONTROL: u16 = 0xA3;
pub const VK_LMENU: u16 = 0xA4; // Left Alt / Left Option
pub const VK_RMENU: u16 = 0xA5; // Right Alt / Right Option (AltGr on ISO)
pub const VK_LWIN: u16 = 0x5B; // Left Windows / Left Command
pub const VK_RWIN: u16 = 0x5C; // Right Windows / Right Command
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;

// Standard Navigation & Control
pub const VK_BACK: u16 = 0x08; // Backspace
pub const VK_TAB: u16 = 0x09; // Tab
pub const VK_CLEAR: u16 = 0x0C; // Clear
pub const VK_RETURN: u16 = 0x0D; // Enter / Return
pub const VK_PAUSE: u16 = 0x13; // Pause / Break
pub const VK_CAPITAL: u16 = 0x14; // Caps Lock
pub const VK_ESCAPE: u16 = 0x1B; // Escape
pub const VK_SPACE: u16 = 0x20; // Spacebar
pub const VK_PRIOR: u16 = 0x21; // Page Up
pub const VK_NEXT: u16 = 0x22; // Page Down
pub const VK_END: u16 = 0x23; // End
pub const VK_HOME: u16 = 0x24; // Home
pub const VK_LEFT: u16 = 0x25; // Left Arrow
pub const VK_UP: u16 = 0x26; // Up Arrow
pub const VK_RIGHT: u16 = 0x27; // Right Arrow
pub const VK_DOWN: u16 = 0x28; // Down Arrow
pub const VK_SELECT: u16 = 0x29; // Select
pub const VK_PRINT: u16 = 0x2A; // Print
pub const VK_EXECUTE: u16 = 0x2B; // Execute
pub const VK_SNAPSHOT: u16 = 0x2C; // Print Screen
pub const VK_INSERT: u16 = 0x2D; // Insert
pub const VK_DELETE: u16 = 0x2E; // Forward Delete
pub const VK_HELP: u16 = 0x2F; // Help
pub const VK_APPS: u16 = 0x5D; // Context Menu / Applications
pub const VK_SLEEP: u16 = 0x5F; // Sleep

// Digits 0-9
pub const VK_0: u16 = 0x30;
pub const VK_1: u16 = 0x31;
pub const VK_2: u16 = 0x32;
pub const VK_3: u16 = 0x33;
pub const VK_4: u16 = 0x34;
pub const VK_5: u16 = 0x35;
pub const VK_6: u16 = 0x36;
pub const VK_7: u16 = 0x37;
pub const VK_8: u16 = 0x38;
pub const VK_9: u16 = 0x39;

// Letters A-Z
pub const VK_A: u16 = 0x41;
pub const VK_B: u16 = 0x42;
pub const VK_C: u16 = 0x43;
pub const VK_D: u16 = 0x44;
pub const VK_E: u16 = 0x45;
pub const VK_F: u16 = 0x46;
pub const VK_G: u16 = 0x47;
pub const VK_H: u16 = 0x48;
pub const VK_I: u16 = 0x49;
pub const VK_J: u16 = 0x4A;
pub const VK_K: u16 = 0x4B;
pub const VK_L: u16 = 0x4C;
pub const VK_M: u16 = 0x4D;
pub const VK_N: u16 = 0x4E;
pub const VK_O: u16 = 0x4F;
pub const VK_P: u16 = 0x50;
pub const VK_Q: u16 = 0x51;
pub const VK_R: u16 = 0x52;
pub const VK_S: u16 = 0x53;
pub const VK_T: u16 = 0x54;
pub const VK_U: u16 = 0x55;
pub const VK_V: u16 = 0x56;
pub const VK_W: u16 = 0x57;
pub const VK_X: u16 = 0x58;
pub const VK_Y: u16 = 0x59;
pub const VK_Z: u16 = 0x5A;

// Numpad Keys
pub const VK_NUMPAD0: u16 = 0x60;
pub const VK_NUMPAD1: u16 = 0x61;
pub const VK_NUMPAD2: u16 = 0x62;
pub const VK_NUMPAD3: u16 = 0x63;
pub const VK_NUMPAD4: u16 = 0x64;
pub const VK_NUMPAD5: u16 = 0x65;
pub const VK_NUMPAD6: u16 = 0x66;
pub const VK_NUMPAD7: u16 = 0x67;
pub const VK_NUMPAD8: u16 = 0x68;
pub const VK_NUMPAD9: u16 = 0x69;
pub const VK_MULTIPLY: u16 = 0x6A;
pub const VK_ADD: u16 = 0x6B;
pub const VK_SEPARATOR: u16 = 0x6C;
pub const VK_SUBTRACT: u16 = 0x6D;
pub const VK_DECIMAL: u16 = 0x6E;
pub const VK_DIVIDE: u16 = 0x6F;

// Function Keys F1..F24
pub const VK_F1: u16 = 0x70;
pub const VK_F2: u16 = 0x71;
pub const VK_F3: u16 = 0x72;
pub const VK_F4: u16 = 0x73;
pub const VK_F5: u16 = 0x74;
pub const VK_F6: u16 = 0x75;
pub const VK_F7: u16 = 0x76;
pub const VK_F8: u16 = 0x77;
pub const VK_F9: u16 = 0x78;
pub const VK_F10: u16 = 0x79;
pub const VK_F11: u16 = 0x7A;
pub const VK_F12: u16 = 0x7B;
pub const VK_F13: u16 = 0x7C;
pub const VK_F14: u16 = 0x7D;
pub const VK_F15: u16 = 0x7E;
pub const VK_F16: u16 = 0x7F;
pub const VK_F17: u16 = 0x80;
pub const VK_F18: u16 = 0x81;
pub const VK_F19: u16 = 0x82;
pub const VK_F20: u16 = 0x83;
pub const VK_F21: u16 = 0x84;
pub const VK_F22: u16 = 0x85;
pub const VK_F23: u16 = 0x86;
pub const VK_F24: u16 = 0x87;

// Lock Keys
pub const VK_NUMLOCK: u16 = 0x90; // Clear on Apple A1843
pub const VK_SCROLL: u16 = 0x91; // Scroll Lock

// Media Keys
pub const VK_VOLUME_MUTE: u16 = 0xAD;
pub const VK_VOLUME_DOWN: u16 = 0xAE;
pub const VK_VOLUME_UP: u16 = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
pub const VK_MEDIA_STOP: u16 = 0xB2;
pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;
pub const VK_LAUNCH_MAIL: u16 = 0xB4;
pub const VK_LAUNCH_MEDIA_SELECT: u16 = 0xB5;
pub const VK_LAUNCH_APP1: u16 = 0xB6;
pub const VK_LAUNCH_APP2: u16 = 0xB7;

// OEM Punctuation (US standard layouts / ISO Italian equivalents)
pub const VK_OEM_1: u16 = 0xBA; // ';:' (US) or 'è' (IT)
pub const VK_OEM_PLUS: u16 = 0xBB; // '+'
pub const VK_OEM_COMMA: u16 = 0xBC; // ','
pub const VK_OEM_MINUS: u16 = 0xBD; // '-'
pub const VK_OEM_PERIOD: u16 = 0xBE; // '.'
pub const VK_OEM_2: u16 = 0xBF; // '/?'
pub const VK_OEM_3: u16 = 0xC0; // '`~' (US) or 'ò' (IT)
pub const VK_OEM_4: u16 = 0xDB; // '[{' (US) or 'è' (IT)
pub const VK_OEM_5: u16 = 0xDC; // '\|' (US) or '\' (IT)
pub const VK_OEM_6: u16 = 0xDD; // ']}' (US) or 'ì' (IT)
pub const VK_OEM_7: u16 = 0xDE; // ''"' (US) or 'à' (IT)
pub const VK_OEM_102: u16 = 0xE2; // '<>' on ISO layouts

// ============================================================================
// Key Definition & Abstraction
// ============================================================================

/// Concrete representation of a keyboard key, combining its Windows Virtual Key
/// code (VK), hardware scancode, and extended key flag (`LLKHF_EXTENDED`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    /// Windows Virtual Key code (`0..=255`).
    pub vk: u16,
    /// Hardware make/break scancode.
    pub scan_code: u16,
    /// Whether this is an extended key (`KEYEVENTF_EXTENDEDKEY` / `LLKHF_EXTENDED`).
    pub extended: bool,
}

impl Key {
    /// Creates a key representation with explicit virtual key, scancode, and extended flag.
    #[must_use]
    pub const fn new(vk: u16, scan_code: u16, extended: bool) -> Self {
        Self {
            vk,
            scan_code,
            extended,
        }
    }

    /// Constructs a `Key` from a virtual key code using standard Windows default scancodes.
    #[must_use]
    pub fn from_vk(vk: u16) -> Self {
        let (scan_code, extended) = default_scancode_and_extended(vk);
        Self {
            vk,
            scan_code,
            extended,
        }
    }

    /// Returns `true` if this key is a modifier (Shift, Ctrl, Alt/Option, Win/Cmd).
    #[must_use]
    pub const fn is_modifier(&self) -> bool {
        matches!(
            self.vk,
            VK_LSHIFT
                | VK_RSHIFT
                | VK_LCONTROL
                | VK_RCONTROL
                | VK_LMENU
                | VK_RMENU
                | VK_LWIN
                | VK_RWIN
                | VK_SHIFT
                | VK_CONTROL
                | VK_MENU
        )
    }

    /// Returns `true` if this key is a function key (`F1..F24`).
    #[must_use]
    pub const fn is_function_key(&self) -> bool {
        self.vk >= VK_F1 && self.vk <= VK_F24
    }

    /// Canonical user-friendly name for this key.
    #[must_use]
    pub fn name(&self) -> &'static str {
        vk_name(self.vk)
    }

    /// Parses a case-insensitive key name or hexadecimal/decimal string into a `Key`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let vk = parse_key_name(s)?;
        Some(Self::from_vk(vk))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Returns the standard default hardware scancode and extended flag for a given Windows VK code.
#[must_use]
pub const fn default_scancode_and_extended(vk: u16) -> (u16, bool) {
    match vk {
        VK_ESCAPE => (0x01, false),
        VK_1 => (0x02, false),
        VK_2 => (0x03, false),
        VK_3 => (0x04, false),
        VK_4 => (0x05, false),
        VK_5 => (0x06, false),
        VK_6 => (0x07, false),
        VK_7 => (0x08, false),
        VK_8 => (0x09, false),
        VK_9 => (0x0A, false),
        VK_0 => (0x0B, false),
        VK_OEM_MINUS => (0x0C, false),
        VK_OEM_PLUS => (0x0D, false),
        VK_BACK => (0x0E, false),
        VK_TAB => (0x0F, false),
        VK_Q => (0x10, false),
        VK_W => (0x11, false),
        VK_E => (0x12, false),
        VK_R => (0x13, false),
        VK_T => (0x14, false),
        VK_Y => (0x15, false),
        VK_U => (0x16, false),
        VK_I => (0x17, false),
        VK_O => (0x18, false),
        VK_P => (0x19, false),
        VK_OEM_4 => (0x1A, false),
        VK_OEM_6 => (0x1B, false),
        VK_RETURN => (0x1C, false),
        VK_LCONTROL => (0x1D, false),
        VK_A => (0x1E, false),
        VK_S => (0x1F, false),
        VK_D => (0x20, false),
        VK_F => (0x21, false),
        VK_G => (0x22, false),
        VK_H => (0x23, false),
        VK_J => (0x24, false),
        VK_K => (0x25, false),
        VK_L => (0x26, false),
        VK_OEM_1 => (0x27, false),
        VK_OEM_7 => (0x28, false),
        VK_OEM_3 => (0x29, false),
        VK_LSHIFT => (0x2A, false),
        VK_OEM_5 => (0x2B, false),
        VK_Z => (0x2C, false),
        VK_X => (0x2D, false),
        VK_C => (0x2E, false),
        VK_V => (0x2F, false),
        VK_B => (0x30, false),
        VK_N => (0x31, false),
        VK_M => (0x32, false),
        VK_OEM_COMMA => (0x33, false),
        VK_OEM_PERIOD => (0x34, false),
        VK_OEM_2 => (0x35, false),
        VK_RSHIFT => (0x36, true),
        VK_MULTIPLY => (0x37, false),
        VK_LMENU => (0x38, false),
        VK_SPACE => (0x39, false),
        VK_CAPITAL => (0x3A, false),
        VK_F1 => (0x3B, false),
        VK_F2 => (0x3C, false),
        VK_F3 => (0x3D, false),
        VK_F4 => (0x3E, false),
        VK_F5 => (0x3F, false),
        VK_F6 => (0x40, false),
        VK_F7 => (0x41, false),
        VK_F8 => (0x42, false),
        VK_F9 => (0x43, false),
        VK_F10 => (0x44, false),
        VK_NUMLOCK => (0x45, true),
        VK_SCROLL => (0x46, false),
        VK_NUMPAD7 => (0x47, false),
        VK_NUMPAD8 => (0x48, false),
        VK_NUMPAD9 => (0x49, false),
        VK_SUBTRACT => (0x4A, false),
        VK_NUMPAD4 => (0x4B, false),
        VK_NUMPAD5 => (0x4C, false),
        VK_NUMPAD6 => (0x4D, false),
        VK_ADD => (0x4E, false),
        VK_NUMPAD1 => (0x4F, false),
        VK_NUMPAD2 => (0x50, false),
        VK_NUMPAD3 => (0x51, false),
        VK_NUMPAD0 => (0x52, false),
        VK_DECIMAL => (0x53, false),
        VK_OEM_102 => (0x56, false),
        VK_F11 => (0x57, false),
        VK_F12 => (0x58, false),
        VK_F13 => (0x64, false),
        VK_F14 => (0x65, false),
        VK_F15 => (0x66, false),
        VK_F16 => (0x67, false),
        VK_F17 => (0x68, false),
        VK_F18 => (0x69, false),
        VK_F19 => (0x6A, false),
        VK_F20 => (0x6B, false),
        VK_F21 => (0x6C, false),
        VK_F22 => (0x6D, false),
        VK_F23 => (0x6E, false),
        VK_F24 => (0x6F, false),

        // Extended Navigation & System Keys
        VK_SNAPSHOT => (0x37, true),
        VK_PAUSE => (0x45, false),
        VK_PRIOR => (0x49, true),
        VK_NEXT => (0x51, true),
        VK_END => (0x4F, true),
        VK_HOME => (0x47, true),
        VK_LEFT => (0x4B, true),
        VK_UP => (0x48, true),
        VK_RIGHT => (0x4D, true),
        VK_DOWN => (0x50, true),
        VK_INSERT => (0x52, true),
        VK_DELETE => (0x53, true),
        VK_DIVIDE => (0x35, true),
        VK_RCONTROL => (0x1D, true),
        VK_RMENU => (0x38, true),
        VK_LWIN => (0x5B, true),
        VK_RWIN => (0x5C, true),
        VK_APPS => (0x5D, true),

        // Media Keys
        VK_VOLUME_MUTE => (0x20, true),
        VK_VOLUME_DOWN => (0x2E, true),
        VK_VOLUME_UP => (0x30, true),
        VK_MEDIA_NEXT_TRACK => (0x19, true),
        VK_MEDIA_PREV_TRACK => (0x10, true),
        VK_MEDIA_STOP => (0x24, true),
        VK_MEDIA_PLAY_PAUSE => (0x22, true),

        _ => (0, false),
    }
}

/// Returns the canonical display name for a Windows VK code.
#[must_use]
pub fn vk_name(vk: u16) -> &'static str {
    match vk {
        VK_BACK => "Backspace",
        VK_TAB => "Tab",
        VK_CLEAR => "Clear",
        VK_RETURN => "Enter",
        VK_PAUSE => "Pause",
        VK_CAPITAL => "CapsLock",
        VK_ESCAPE => "Escape",
        VK_SPACE => "Space",
        VK_PRIOR => "PageUp",
        VK_NEXT => "PageDown",
        VK_END => "End",
        VK_HOME => "Home",
        VK_LEFT => "Left",
        VK_UP => "Up",
        VK_RIGHT => "Right",
        VK_DOWN => "Down",
        VK_SNAPSHOT => "PrintScreen",
        VK_INSERT => "Insert",
        VK_DELETE => "Delete",
        VK_LWIN => "LeftCommand",
        VK_RWIN => "RightCommand",
        VK_APPS => "Apps",
        VK_LSHIFT => "LeftShift",
        VK_RSHIFT => "RightShift",
        VK_LCONTROL => "LeftControl",
        VK_RCONTROL => "RightControl",
        VK_LMENU => "LeftOption",
        VK_RMENU => "RightOption",
        VK_NUMLOCK => "NumLock",
        VK_SCROLL => "ScrollLock",

        VK_VOLUME_MUTE => "VolumeMute",
        VK_VOLUME_DOWN => "VolumeDown",
        VK_VOLUME_UP => "VolumeUp",
        VK_MEDIA_NEXT_TRACK => "NextTrack",
        VK_MEDIA_PREV_TRACK => "PrevTrack",
        VK_MEDIA_STOP => "MediaStop",
        VK_MEDIA_PLAY_PAUSE => "PlayPause",

        VK_F1 => "F1",
        VK_F2 => "F2",
        VK_F3 => "F3",
        VK_F4 => "F4",
        VK_F5 => "F5",
        VK_F6 => "F6",
        VK_F7 => "F7",
        VK_F8 => "F8",
        VK_F9 => "F9",
        VK_F10 => "F10",
        VK_F11 => "F11",
        VK_F12 => "F12",
        VK_F13 => "F13",
        VK_F14 => "F14",
        VK_F15 => "F15",
        VK_F16 => "F16",
        VK_F17 => "F17",
        VK_F18 => "F18",
        VK_F19 => "F19",
        VK_F20 => "F20",
        VK_F21 => "F21",
        VK_F22 => "F22",
        VK_F23 => "F23",
        VK_F24 => "F24",

        VK_0 => "0",
        VK_1 => "1",
        VK_2 => "2",
        VK_3 => "3",
        VK_4 => "4",
        VK_5 => "5",
        VK_6 => "6",
        VK_7 => "7",
        VK_8 => "8",
        VK_9 => "9",

        VK_A => "A",
        VK_B => "B",
        VK_C => "C",
        VK_D => "D",
        VK_E => "E",
        VK_F => "F",
        VK_G => "G",
        VK_H => "H",
        VK_I => "I",
        VK_J => "J",
        VK_K => "K",
        VK_L => "L",
        VK_M => "M",
        VK_N => "N",
        VK_O => "O",
        VK_P => "P",
        VK_Q => "Q",
        VK_R => "R",
        VK_S => "S",
        VK_T => "T",
        VK_U => "U",
        VK_V => "V",
        VK_W => "W",
        VK_X => "X",
        VK_Y => "Y",
        VK_Z => "Z",

        _ => "Unknown",
    }
}

/// Parses a key name, identifier, or numeric/hex VK code into a `u16` virtual key code.
#[must_use]
pub fn parse_key_name(s: &str) -> Option<u16> {
    let clean = s.trim().to_ascii_lowercase().replace(['_', '-', ' '], "");

    // Hexadecimal string e.g. "0x2c" or "0x5b"
    if let Some(hex_str) = clean.strip_prefix("0x") {
        return u16::from_str_radix(hex_str, 16).ok().filter(|&c| c <= 255);
    }

    // Direct decimal string e.g. "44"
    if let Ok(code) = clean.parse::<u16>() {
        return (code <= 255).then_some(code);
    }

    // Strip optional "vk" prefix e.g. "vkf13" or "vksnapshot"
    let clean = clean.strip_prefix("vk").unwrap_or(&clean);

    match clean {
        // Missing Windows Keys (F4.1)
        "printscreen" | "prtsc" | "prtscn" | "snapshot" => Some(VK_SNAPSHOT),
        "scrolllock" | "scrlk" | "scroll" => Some(VK_SCROLL),
        "pause" | "break" | "pausebreak" => Some(VK_PAUSE),
        "insert" | "ins" => Some(VK_INSERT),
        "delete" | "del" | "forwarddelete" => Some(VK_DELETE),

        // Navigation (F1.2)
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "pageup" | "pgup" | "prior" => Some(VK_PRIOR),
        "pagedown" | "pgdn" | "next" => Some(VK_NEXT),
        "left" | "leftarrow" | "arrowleft" => Some(VK_LEFT),
        "right" | "rightarrow" | "arrowright" => Some(VK_RIGHT),
        "up" | "uparrow" | "arrowup" => Some(VK_UP),
        "down" | "downarrow" | "arrowdown" => Some(VK_DOWN),
        "backspace" | "back" => Some(VK_BACK),
        "enter" | "return" => Some(VK_RETURN),
        "tab" => Some(VK_TAB),
        "space" | "spacebar" => Some(VK_SPACE),
        "escape" | "esc" => Some(VK_ESCAPE),
        "capslock" | "capital" | "caps" => Some(VK_CAPITAL),
        "numlock" | "clear" => Some(VK_NUMLOCK),
        "apps" | "menu" | "contextmenu" => Some(VK_APPS),

        // Modifiers (F3.1, F3.2)
        "leftcommand" | "leftcmd" | "cmdleft" | "commandleft" | "lwin" | "leftwin" | "winleft" => {
            Some(VK_LWIN)
        }
        "rightcommand" | "rightcmd" | "cmdright" | "commandright" | "rwin" | "rightwin"
        | "winright" => Some(VK_RWIN),
        "leftoption" | "leftopt" | "optleft" | "optionleft" | "lalt" | "leftalt" | "altleft"
        | "lmenu" => Some(VK_LMENU),
        "rightoption" | "rightopt" | "optright" | "optionright" | "ralt" | "rightalt"
        | "altright" | "altgr" | "rmenu" => Some(VK_RMENU),
        "leftcontrol" | "leftctrl" | "ctrlleft" | "controlleft" | "lctrl" => Some(VK_LCONTROL),
        "rightcontrol" | "rightctrl" | "ctrlright" | "controlright" | "rctrl" => Some(VK_RCONTROL),
        "leftshift" | "shiftleft" | "lshift" => Some(VK_LSHIFT),
        "rightshift" | "shiftright" | "rshift" => Some(VK_RSHIFT),

        // Function Keys F1..F24
        "f1" => Some(VK_F1),
        "f2" => Some(VK_F2),
        "f3" => Some(VK_F3),
        "f4" => Some(VK_F4),
        "f5" => Some(VK_F5),
        "f6" => Some(VK_F6),
        "f7" => Some(VK_F7),
        "f8" => Some(VK_F8),
        "f9" => Some(VK_F9),
        "f10" => Some(VK_F10),
        "f11" => Some(VK_F11),
        "f12" => Some(VK_F12),
        "f13" => Some(VK_F13),
        "f14" => Some(VK_F14),
        "f15" => Some(VK_F15),
        "f16" => Some(VK_F16),
        "f17" => Some(VK_F17),
        "f18" => Some(VK_F18),
        "f19" => Some(VK_F19),
        "f20" => Some(VK_F20),
        "f21" => Some(VK_F21),
        "f22" => Some(VK_F22),
        "f23" => Some(VK_F23),
        "f24" => Some(VK_F24),

        // Media Keys (F2.3)
        "volumemute" | "mute" => Some(VK_VOLUME_MUTE),
        "volumedown" => Some(VK_VOLUME_DOWN),
        "volumeup" => Some(VK_VOLUME_UP),
        "playpause" | "mediaplaypause" => Some(VK_MEDIA_PLAY_PAUSE),
        "nexttrack" | "medianexttrack" => Some(VK_MEDIA_NEXT_TRACK),
        "prevtrack" | "previoustrack" | "mediaprevtrack" => Some(VK_MEDIA_PREV_TRACK),
        "stop" | "mediastop" => Some(VK_MEDIA_STOP),

        // Digits
        "0" => Some(VK_0),
        "1" => Some(VK_1),
        "2" => Some(VK_2),
        "3" => Some(VK_3),
        "4" => Some(VK_4),
        "5" => Some(VK_5),
        "6" => Some(VK_6),
        "7" => Some(VK_7),
        "8" => Some(VK_8),
        "9" => Some(VK_9),

        // Letters
        "a" => Some(VK_A),
        "b" => Some(VK_B),
        "c" => Some(VK_C),
        "d" => Some(VK_D),
        "e" => Some(VK_E),
        "f" => Some(VK_F),
        "g" => Some(VK_G),
        "h" => Some(VK_H),
        "i" => Some(VK_I),
        "j" => Some(VK_J),
        "k" => Some(VK_K),
        "l" => Some(VK_L),
        "m" => Some(VK_M),
        "n" => Some(VK_N),
        "o" => Some(VK_O),
        "p" => Some(VK_P),
        "q" => Some(VK_Q),
        "r" => Some(VK_R),
        "s" => Some(VK_S),
        "t" => Some(VK_T),
        "u" => Some(VK_U),
        "v" => Some(VK_V),
        "w" => Some(VK_W),
        "x" => Some(VK_X),
        "y" => Some(VK_Y),
        "z" => Some(VK_Z),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_marker_ascii() {
        assert_eq!(MAGICBRIDGE_EXTRA_INFO, 0x4D42_4745);
        let bytes = (MAGICBRIDGE_EXTRA_INFO as u32).to_be_bytes();
        assert_eq!(&bytes, b"MBGE");
    }

    #[test]
    fn test_synthetic_altgr_scancode_value() {
        assert_eq!(SC_ISO_ALTGR_SYNTHETIC_LCTRL, 0x021D);
    }

    #[test]
    fn test_vk_constants_match_windows_specs() {
        assert_eq!(VK_LWIN, 0x5B);
        assert_eq!(VK_RWIN, 0x5C);
        assert_eq!(VK_LMENU, 0xA4);
        assert_eq!(VK_RMENU, 0xA5);
        assert_eq!(VK_LCONTROL, 0xA2);
        assert_eq!(VK_RCONTROL, 0xA3);
        assert_eq!(VK_SNAPSHOT, 0x2C);
        assert_eq!(VK_SCROLL, 0x91);
        assert_eq!(VK_PAUSE, 0x13);
        assert_eq!(VK_INSERT, 0x2D);
        assert_eq!(VK_DELETE, 0x2E);
        assert_eq!(VK_HOME, 0x24);
        assert_eq!(VK_END, 0x23);
        assert_eq!(VK_PRIOR, 0x21);
        assert_eq!(VK_NEXT, 0x22);
        assert_eq!(VK_NUMLOCK, 0x90);
        assert_eq!(VK_F1, 0x70);
        assert_eq!(VK_F13, 0x7C);
        assert_eq!(VK_F19, 0x82);
    }

    #[test]
    fn test_key_from_vk_default_scancodes() {
        let prtsc = Key::from_vk(VK_SNAPSHOT);
        assert_eq!(prtsc.vk, 0x2C);
        assert_eq!(prtsc.scan_code, 0x37);
        assert!(prtsc.extended);

        let scrlk = Key::from_vk(VK_SCROLL);
        assert_eq!(scrlk.vk, 0x91);
        assert_eq!(scrlk.scan_code, 0x46);
        assert!(!scrlk.extended);

        let pause = Key::from_vk(VK_PAUSE);
        assert_eq!(pause.vk, 0x13);
        assert_eq!(pause.scan_code, 0x45);
        assert!(!pause.extended);

        let f13 = Key::from_vk(VK_F13);
        assert_eq!(f13.vk, 0x7C);
        assert_eq!(f13.scan_code, 0x64);
        assert!(!f13.extended);
    }

    #[test]
    fn test_key_name_parsing_synonyms() {
        assert_eq!(parse_key_name("PrintScreen"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("PrtSc"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("snapshot"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("VK_SNAPSHOT"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("ScrollLock"), Some(VK_SCROLL));
        assert_eq!(parse_key_name("ScrLk"), Some(VK_SCROLL));
        assert_eq!(parse_key_name("Pause"), Some(VK_PAUSE));
        assert_eq!(parse_key_name("Break"), Some(VK_PAUSE));
        assert_eq!(parse_key_name("Insert"), Some(VK_INSERT));
        assert_eq!(parse_key_name("Delete"), Some(VK_DELETE));
        assert_eq!(parse_key_name("Home"), Some(VK_HOME));
        assert_eq!(parse_key_name("End"), Some(VK_END));
        assert_eq!(parse_key_name("PageUp"), Some(VK_PRIOR));
        assert_eq!(parse_key_name("PageDown"), Some(VK_NEXT));
        assert_eq!(parse_key_name("LeftCommand"), Some(VK_LWIN));
        assert_eq!(parse_key_name("RightOption"), Some(VK_RMENU));
        assert_eq!(parse_key_name("AltGr"), Some(VK_RMENU));
        assert_eq!(parse_key_name("F13"), Some(VK_F13));
        assert_eq!(parse_key_name("0x2C"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("44"), Some(VK_SNAPSHOT));
        assert_eq!(parse_key_name("InvalidKeyXYZ"), None);
    }

    #[test]
    fn test_key_methods() {
        let cmd = Key::from_vk(VK_LWIN);
        assert!(cmd.is_modifier());
        assert!(!cmd.is_function_key());
        assert_eq!(cmd.name(), "LeftCommand");

        let f14 = Key::from_vk(VK_F14);
        assert!(!f14.is_modifier());
        assert!(f14.is_function_key());
        assert_eq!(f14.name(), "F14");
    }
}
