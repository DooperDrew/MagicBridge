//! MagicBridge background agent (always running, tiny).
//!
//! Daemon runtime shell, single-instance mutex, hidden Win32 message window,
//! and system tray icon with context menu (Milestone 1.1).

// No console window in release builds; keep it in debug for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod single_instance;
pub mod state;
pub mod tray;
pub mod window;

use magicbridge_core::Config;
use single_instance::{SingleInstanceError, SingleInstanceGuard};
use state::DaemonState;
use window::{MessageWindow, run_message_pump};

fn main() {
    // 1. Single-instance guard (F8.3): exit cleanly if already running.
    let _guard = match SingleInstanceGuard::acquire() {
        Ok(guard) => guard,
        Err(SingleInstanceError::AlreadyRunning) => {
            // Already running: clean exit with code 0.
            return;
        }
        Err(e) => {
            eprintln!("Failed to acquire single instance guard: {e}");
            std::process::exit(1);
        }
    };

    // 2. Load configuration from default path or use defaults.
    let config_path = magicbridge_core::default_config_path().ok();
    let config = match config_path.as_ref() {
        Some(path) => Config::load_or_default(path).unwrap_or_default(),
        None => Config::default(),
    };

    // 3. Initialize daemon state.
    let state = DaemonState::new(config, config_path);

    // 4. Create hidden Win32 message window & system tray icon.
    let _window = match MessageWindow::create(state) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create background message window: {e}");
            std::process::exit(1);
        }
    };

    // 5. Run Win32 message pump (event-driven, 0% idle CPU per NFR-P2).
    let exit_code = run_message_pump();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
