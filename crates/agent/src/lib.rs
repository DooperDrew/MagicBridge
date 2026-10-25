//! MagicBridge background agent library.
//!
//! Provides the autostart manager, input hooks, HID battery reader,
//! tray interface, and IPC daemon components.

pub mod autostart;
pub mod device_notify;
pub mod hook;
pub mod injector;
pub mod ipc;
pub mod single_instance;
pub mod state;
pub mod tray;
pub mod window;

pub use hook::{HookHandle, set_active_table};
pub use injector::{build_keyboard_input, send_key_down, send_key_up, send_keyboard_event};
pub use ipc::{
    AtomicMappingTable, DaemonStatus, IpcClient, IpcError, IpcRequest, IpcResponse, IpcServer,
    IpcServerHandle,
};
