//! Pure MagicBridge logic: key model, mapping engine and config schema.
//!
//! This crate is platform-independent and unit-tested. It must not depend on
//! the `windows` crate or perform any I/O in the hot path.

#![forbid(unsafe_code)]

pub mod device;

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
