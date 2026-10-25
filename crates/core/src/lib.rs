//! Pure MagicBridge logic: key model, mapping engine and config schema.
//!
//! This crate is platform-independent and unit-tested. It must not depend on
//! the `windows` crate or perform any I/O in the hot path.

#![forbid(unsafe_code)]

/// Apple Inc. USB / Bluetooth vendor ID.
pub const APPLE_VID: u16 = 0x05AC;

/// Returns `true` if the given vendor ID belongs to Apple.
#[must_use]
pub const fn is_apple_vendor(vid: u16) -> bool {
    vid == APPLE_VID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_vendor_is_recognised() {
        assert!(is_apple_vendor(0x05AC));
        assert!(!is_apple_vendor(0x046D));
    }
}
