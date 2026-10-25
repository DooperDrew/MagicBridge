//! MagicBridge background agent (always running, tiny).
//!
//! Daemon runtime shell, single-instance mutex, hidden Win32 message window,
//! low-level keyboard hook, and system tray icon with context menu.

// No console window in release builds; keep it in debug for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod action_worker;
pub mod autostart;
pub mod battery;
pub mod device_notify;
pub mod hook;
pub mod injector;
pub mod ipc;
pub mod single_instance;
pub mod state;
pub mod tray;
pub mod window;

use ipc::{AtomicMappingTable, IpcServer};
use magicbridge_core::Config;
use single_instance::{SingleInstanceError, SingleInstanceGuard};
use state::DaemonState;
use std::sync::{Arc, Mutex};
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

    // 3. Initialize daemon state and atomic mapping table (Milestone 2.1 & 2.5).
    let state = DaemonState::new(config, config_path);
    let table = state.compile_active_profile();
    hook::set_active_table(table.clone());
    let atomic_table = AtomicMappingTable::new(table);
    let shared_state = Arc::new(Mutex::new(state.clone()));

    // Start background action worker for custom actions (F5.1..F5.2, NFR-P3).
    let _action_worker = action_worker::start_action_worker();

    // 4. Create hidden Win32 message window & system tray icon.
    let window = match MessageWindow::create(state) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create background message window: {e}");
            std::process::exit(1);
        }
    };

    // 5. Start Named Pipe IPC server (\\.\pipe\MagicBridge_Agent_IPC).
    let _ipc_server = match IpcServer::start_default(
        shared_state,
        atomic_table,
        Some(ipc::NotifyHwnd(window.hwnd())),
    ) {
        Ok(srv) => Some(srv),
        Err(e) => {
            eprintln!("[MagicBridge] Warning: failed to start Named Pipe IPC server: {e}");
            None
        }
    };

    // 6. Run Win32 message pump (event-driven, 0% idle CPU per NFR-P2).
    let exit_code = run_message_pump();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
