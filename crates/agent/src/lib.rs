//! MagicBridge background agent library.
//!
//! Provides the autostart manager, input hooks, HID battery reader,
//! tray interface, and IPC daemon components.

pub mod autostart;
pub mod single_instance;
pub mod state;
pub mod tray;
pub mod window;
