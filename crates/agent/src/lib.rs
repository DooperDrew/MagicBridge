//! MagicBridge background agent library.
//!
//! Provides the autostart manager, input hooks, HID battery reader,
//! tray interface, IPC daemon, game mode manager, and diagnostic logger.

pub mod action_worker;
pub mod autostart;
pub mod battery;
pub mod device_notify;
pub mod game_mode;
pub mod hook;
pub mod injector;
pub mod ipc;
pub mod logger;
pub mod single_instance;
pub mod state;
pub mod tray;
pub mod window;

pub use action_worker::{ActionWorkerHandle, dispatch_action, start_action_worker};
pub use battery::{
    BatteryStatus, query_battery, read_battery_from_hid_path, read_bluetooth_pnp_battery,
};
pub use game_mode::{
    GameModeTracker, WM_GAME_MODE_EVENT, is_foreground_game, is_fullscreen_rect,
    is_ignored_shell_class, matches_process_list,
};
pub use hook::{
    HookAction, HookHandle, KeyState, clear_active_table, decode_remap, encode_remap,
    get_active_custom_action, has_active_table, is_fn_down, is_hook_active, process_keyboard_event,
    release_all_active_keys, set_active_table, set_hook_active,
};
pub use injector::{build_keyboard_input, send_key_down, send_key_up, send_keyboard_event};
pub use ipc::{
    AtomicMappingTable, DaemonStatus, IpcClient, IpcError, IpcRequest, IpcResponse, IpcServer,
    IpcServerHandle,
};
pub use logger::{
    get_recent_logs, is_enabled as is_logging_enabled, log, set_enabled as set_logging_enabled,
};
