//! Pure MagicBridge logic: key model, mapping engine and config schema.
//!
//! This crate is platform-independent and unit-tested. It must not depend on
//! the `windows` crate or perform any I/O in the hot path.

#![forbid(unsafe_code)]

pub mod config;
pub mod device;
pub mod engine;
pub mod key;

pub use config::{
    Config, ConfigError, CustomAction, FnKeyChoice, KeyAction, KeyboardLayout, ModifierPreset,
    NamedProfile, TopRowMode, atomic_write, default_config_dir, default_config_path,
};
pub use engine::{CompiledMappingTable, RemapAction, compile_profile};
pub use key::{
    Key, MAGICBRIDGE_EXTRA_INFO, SC_ISO_ALTGR_SYNTHETIC_LCTRL, VK_A, VK_ADD, VK_APPS, VK_B,
    VK_BACK, VK_C, VK_CAPITAL, VK_CLEAR, VK_CONTROL, VK_D, VK_DECIMAL, VK_DELETE, VK_DIVIDE,
    VK_DOWN, VK_E, VK_END, VK_ESCAPE, VK_EXECUTE, VK_F, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6,
    VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18,
    VK_F19, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_G, VK_H, VK_HELP, VK_HOME, VK_I, VK_INSERT,
    VK_J, VK_K, VK_L, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK,
    VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_MULTIPLY, VK_N, VK_NEXT,
    VK_NUMLOCK, VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6,
    VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9, VK_O, VK_P, VK_PAUSE, VK_PRINT, VK_PRIOR, VK_Q, VK_R,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_S, VK_SCROLL, VK_SELECT,
    VK_SEPARATOR, VK_SHIFT, VK_SLEEP, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_T, VK_TAB, VK_U,
    VK_UP, VK_V, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP, VK_W, VK_X, VK_Y, VK_Z,
    parse_key_name, vk_name,
};

/// Apple Inc. USB vendor ID (USB-IF).
pub const APPLE_VID: u16 = 0x05AC;

/// Apple Inc. Bluetooth SIG company identifier (used by A1843 over Bluetooth: `VID&0001004c`).
pub const APPLE_BT_VID: u16 = 0x004C;

/// Apple Magic Keyboard with Numeric Keypad (A1843) product ID.
pub const A1843_PID: u16 = 0x026C;

/// Returns `true` if the given vendor ID belongs to Apple (USB `0x05AC` or Bluetooth `0x004C`).
#[must_use]
pub const fn is_apple_vendor(vid: u16) -> bool {
    vid == APPLE_VID || vid == APPLE_BT_VID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_vendor_is_recognised() {
        assert!(is_apple_vendor(0x05AC));
        assert!(is_apple_vendor(0x004C));
        assert!(!is_apple_vendor(0x046D));
    }
}
