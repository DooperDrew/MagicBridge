//! Thin Win32 SendInput injection wrapper.
//!
//! Conforms strictly to NFR-P3 (<0.1ms latency, zero heap allocations, zero locks, zero I/O)
//! and tags every synthetic event with `MAGICBRIDGE_EXTRA_INFO` to prevent hook feedback loops.

use magicbridge_core::MAGICBRIDGE_EXTRA_INFO;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};

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
    use magicbridge_core::key::{VK_DELETE, VK_HOME, VK_SNAPSHOT};

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
}
