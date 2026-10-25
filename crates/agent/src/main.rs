//! MagicBridge background agent (always running, tiny).
//!
//! Skeleton only (M0.1). Hook, HID reader, tray and IPC arrive in M1–M4.

// No console window in release builds; keep it in debug for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Placeholder: referenced so the dependency on `core` is exercised.
    let _ = magicbridge_core::APPLE_VID;
}
