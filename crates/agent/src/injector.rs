//! Thin Win32 SendInput injection wrapper.
//!
//! Conforms strictly to NFR-P3 (<0.1ms latency, zero heap allocations, zero locks, zero I/O)
//! and tags every synthetic event with `MAGICBRIDGE_EXTRA_INFO` to prevent hook feedback loops.

use magicbridge_core::{Key, MAGICBRIDGE_EXTRA_INFO};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};

/// Maximum number of keys supported in a single atomic shortcut combo.
pub const MAX_SHORTCUT_KEYS: usize = 4;

/// Builds a single Win32 [`INPUT`] structure for keyboard events.
///
/// Sets `dwExtraInfo = MAGICBRIDGE_EXTRA_INFO` so the low-level keyboard hook
/// can detect and ignore its own synthesized input, preventing infinite loops.
#[inline]
pub fn build_keyboard_input(vk: u16, scan_code: u16, key_up: bool, extended: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MAGICBRIDGE_EXTRA_INFO,
            },
        },
    }
}

/// Injects a single keyboard event (key-down or key-up) via Win32 [`SendInput`].
///
/// Returns `true` if the event was successfully inserted into the system input queue.
pub fn send_keyboard_event(vk: u16, scan_code: u16, key_up: bool, extended: bool) -> bool {
    let input = build_keyboard_input(vk, scan_code, key_up, extended);
    // SAFETY: SendInput inserts synthesized input into the active session input stream.
    // The `INPUT` structure is correctly sized, initialized, and tagged with `MAGICBRIDGE_EXTRA_INFO`.
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    sent == 1
}

/// Injects a key-down event for the given virtual key, scancode, and extended flag.
#[inline]
pub fn send_key_down(vk: u16, scan_code: u16, extended: bool) -> bool {
    send_keyboard_event(vk, scan_code, false, extended)
}

/// Injects a key-up event for the given virtual key, scancode, and extended flag.
#[inline]
pub fn send_key_up(vk: u16, scan_code: u16, extended: bool) -> bool {
    send_keyboard_event(vk, scan_code, true, extended)
}

/// Builds the array of Win32 [`INPUT`] structures needed to execute a multi-key shortcut.
///
/// Pressing all keys down in forward order, then releasing all keys in reverse order.
/// Each event is tagged with `MAGICBRIDGE_EXTRA_INFO`.
///
/// Returns the number of events populated in `buffer`.
pub fn build_shortcut_inputs(keys: &[Key], buffer: &mut [INPUT]) -> usize {
    if keys.is_empty() {
        return 0;
    }
    let total = keys.len() * 2;
    if buffer.len() < total {
        return 0;
    }

    // Key-down events in forward order
    for (i, key) in keys.iter().enumerate() {
        buffer[i] = build_keyboard_input(key.vk, key.scan_code, false, key.extended);
    }
    // Key-up events in reverse order
    for (i, key) in keys.iter().rev().enumerate() {
        buffer[keys.len() + i] = build_keyboard_input(key.vk, key.scan_code, true, key.extended);
    }

    total
}

/// Injects a multi-key shortcut combination atomically via Win32 [`SendInput`].
///
/// Sends key-down in forward order, and key-up in reverse order in a single call.
/// Conforms to `NFR-P3` (zero allocations, zero locks, zero I/O) and `NFR-S3` (stuck-key safety).
/// Returns `true` if all inputs were successfully accepted.
pub fn send_shortcut(keys: &[Key]) -> bool {
    if keys.is_empty() || keys.len() > MAX_SHORTCUT_KEYS {
        return false;
    }
    let mut buffer = [build_keyboard_input(0, 0, false, false); MAX_SHORTCUT_KEYS * 2];
    let count = build_shortcut_inputs(keys, &mut buffer);
    if count == 0 {
        return false;
    }
    send_inputs(&buffer[..count]) == count as u32
}

/// Injects a batch of pre-built keyboard events.
///
/// Returns the number of events successfully inserted into the input stream.
pub fn send_inputs(inputs: &[INPUT]) -> u32 {
    if inputs.is_empty() {
        return 0;
    }
    // SAFETY: SendInput processes the slice of INPUT structs. All elements in the slice
    // must be valid initialized structures.
    unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::key::{VK_DELETE, VK_HOME, VK_LWIN, VK_SNAPSHOT, VK_TAB};

    #[test]
    fn test_build_keyboard_input_tags_sentinel_extra_info() {
        let input = build_keyboard_input(VK_SNAPSHOT, 0x37, false, false);
        // SAFETY: Reading the ki union field of INPUT_KEYBOARD.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.wVk.0, VK_SNAPSHOT);
        assert_eq!(ki.wScan, 0x37);
        assert_eq!(ki.dwExtraInfo, MAGICBRIDGE_EXTRA_INFO);
        assert_eq!(ki.dwFlags, KEYBD_EVENT_FLAGS(0));
    }

    #[test]
    fn test_build_keyboard_input_flags_extended_and_keyup() {
        let input_down = build_keyboard_input(VK_HOME, 0x47, false, true);
        // SAFETY: Reading the ki union field of INPUT_KEYBOARD.
        let ki_down = unsafe { input_down.Anonymous.ki };
        assert_eq!(ki_down.dwFlags, KEYEVENTF_EXTENDEDKEY);

        let input_up = build_keyboard_input(VK_HOME, 0x47, true, true);
        // SAFETY: Reading the ki union field of INPUT_KEYBOARD.
        let ki_up = unsafe { input_up.Anonymous.ki };
        assert_eq!(ki_up.dwFlags, KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY);

        let input_normal_up = build_keyboard_input(VK_DELETE, 0x53, true, false);
        // SAFETY: Reading the ki union field of INPUT_KEYBOARD.
        let ki_norm = unsafe { input_normal_up.Anonymous.ki };
        assert_eq!(ki_norm.dwFlags, KEYEVENTF_KEYUP);
    }

    #[test]
    fn test_send_inputs_empty_slice() {
        assert_eq!(send_inputs(&[]), 0);
    }

    #[test]
    fn test_build_shortcut_inputs_task_view_win_tab() {
        let win = Key::from_vk(VK_LWIN);
        let tab = Key::from_vk(VK_TAB);
        let mut buffer = [build_keyboard_input(0, 0, false, false); 8];

        let count = build_shortcut_inputs(&[win, tab], &mut buffer);
        assert_eq!(count, 4);

        // 1. Win down
        // SAFETY: Reading union field ki for verification.
        let ki0 = unsafe { buffer[0].Anonymous.ki };
        assert_eq!(ki0.wVk.0, VK_LWIN);
        assert_eq!(ki0.dwFlags, KEYEVENTF_EXTENDEDKEY);
        assert_eq!(ki0.dwExtraInfo, MAGICBRIDGE_EXTRA_INFO);

        // 2. Tab down
        // SAFETY: Reading union field ki for verification.
        let ki1 = unsafe { buffer[1].Anonymous.ki };
        assert_eq!(ki1.wVk.0, VK_TAB);
        assert_eq!(ki1.dwFlags, KEYBD_EVENT_FLAGS(0));
        assert_eq!(ki1.dwExtraInfo, MAGICBRIDGE_EXTRA_INFO);

        // 3. Tab up (reverse order)
        // SAFETY: Reading union field ki for verification.
        let ki2 = unsafe { buffer[2].Anonymous.ki };
        assert_eq!(ki2.wVk.0, VK_TAB);
        assert_eq!(ki2.dwFlags, KEYEVENTF_KEYUP);
        assert_eq!(ki2.dwExtraInfo, MAGICBRIDGE_EXTRA_INFO);

        // 4. Win up (reverse order)
        // SAFETY: Reading union field ki for verification.
        let ki3 = unsafe { buffer[3].Anonymous.ki };
        assert_eq!(ki3.wVk.0, VK_LWIN);
        assert_eq!(ki3.dwFlags, KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY);
        assert_eq!(ki3.dwExtraInfo, MAGICBRIDGE_EXTRA_INFO);
    }

    #[test]
    fn test_send_shortcut_empty_returns_false() {
        assert!(!send_shortcut(&[]));
    }
}
