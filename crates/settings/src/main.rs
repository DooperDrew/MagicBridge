//! MagicBridge settings GUI native binary (runs only while open).
//!
//! Complies with F7.1, NFR-P5, and docs/04-ux.md:
//! - Window dimensions ~920x540.
//! - Non-admin user mode execution.
//! - Terminates completely when closed (zero resident memory).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{NativeOptions, egui::ViewportBuilder};
use magicbridge_settings::MagicBridgeApp;

fn main() -> Result<(), eframe::Error> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([920.0, 540.0])
            .with_min_inner_size([860.0, 480.0])
            .with_title("MagicBridge Settings"),
        ..Default::default()
    };

    eframe::run_native(
        "MagicBridge Settings",
        native_options,
        Box::new(|cc| Ok(Box::new(MagicBridgeApp::new(cc)))),
    )
}
