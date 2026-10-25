//! Named Pipe IPC Server and Atomic Mapping Table Hot-Swap (Milestone 2.5).
//!
//! Provides:
//! - Thread-safe, lock-free atomic hot-swap of the compiled mapping table ([`AtomicMappingTable`]).
//! - Win32 Named Pipe server listening on `\\.\pipe\MagicBridge_Agent_IPC` (accessible per-user).
//! - Newline-delimited JSON and TOML request/response protocol for `Reload`, `SwitchProfile`, `GetStatus`, and `Pause`.
//! - Lightweight [`IpcClient`] for UI, CLI, and integration tests.
//!
//! Complies strictly with:
//! - `F7.5`: Changes apply instantly without restarting the background agent.
//! - `NFR-P3`: WH_KEYBOARD_LL hook reads the active table with zero heap allocation, zero locks, and zero blocking syscalls.

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use arc_swap::ArcSwap;
use magicbridge_core::{CompiledMappingTable, Config, TopRowMode};
use serde::{Deserialize, Serialize};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_CONNECTED, HANDLE, HWND,
    INVALID_HANDLE_VALUE, LPARAM, WPARAM,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_MODE,
    FlushFileBuffers, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, NAMED_PIPE_MODE, PIPE_READMODE_BYTE,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WaitNamedPipeW,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::PCWSTR;

use crate::state::DaemonState;

/// Default named pipe name per user session (Milestone 2.5).
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\MagicBridge_Agent_IPC";

/// Custom window message to notify the agent message window of an IPC state change.
pub const WM_IPC_STATE_UPDATED: u32 = 0x8000 + 10; // WM_APP + 10

/// Wrapper for [`HWND`] that implements [`Send`] and [`Sync`] for cross-thread message posting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyHwnd(pub HWND);

// SAFETY: Win32 HWND handles are valid across threads within the same desktop session,
// and PostMessageW is explicitly thread-safe per Win32 API specifications.
unsafe impl Send for NotifyHwnd {}
// SAFETY: NotifyHwnd holds a copyable handle value with no thread-unsafe interior mutability.
unsafe impl Sync for NotifyHwnd {}

impl From<HWND> for NotifyHwnd {
    fn from(h: HWND) -> Self {
        Self(h)
    }
}

/// Thread-safe, lock-free container for the active compiled mapping table.
///
/// Complies with `NFR-P3`: reading the table in the low-level hook thread
/// involves zero locks, zero blocking syscalls, and zero heap allocations.
/// Hot-swapping the table atomically from IPC or tray menu takes effect immediately
/// without interrupting input handling.
#[derive(Debug, Clone)]
pub struct AtomicMappingTable {
    inner: Arc<ArcSwap<CompiledMappingTable>>,
    enabled: Arc<AtomicBool>,
}

impl AtomicMappingTable {
    /// Creates a new atomic mapping table with the specified compiled table and enabled status.
    #[must_use]
    pub fn new(table: CompiledMappingTable) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(table)),
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Creates a new atomic mapping table with an explicit initial enabled flag.
    #[must_use]
    pub fn with_enabled(table: CompiledMappingTable, enabled: bool) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(table)),
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }

    /// Atomically replaces the compiled mapping table.
    pub fn store(&self, table: CompiledMappingTable) {
        self.inner.store(Arc::new(table));
    }

    /// Atomically swaps the compiled mapping table and returns the previous table.
    pub fn swap(&self, table: CompiledMappingTable) -> Arc<CompiledMappingTable> {
        self.inner.swap(Arc::new(table))
    }

    /// Loads the current compiled mapping table guard (zero locks, zero heap allocations).
    #[must_use]
    pub fn load(&self) -> arc_swap::Guard<Arc<CompiledMappingTable>> {
        self.inner.load()
    }

    /// Loads a full clone of `Arc<CompiledMappingTable>`.
    #[must_use]
    pub fn load_full(&self) -> Arc<CompiledMappingTable> {
        self.inner.load_full()
    }

    /// Returns `true` if remapping engine is currently active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Returns `true` if remapping engine is currently paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        !self.is_enabled()
    }

    /// Sets the remapping engine enabled state.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Status report returned by the agent over IPC (F7.6, F8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonStatus {
    /// Whether the background agent is running.
    pub agent_running: bool,
    /// Number of connected Apple keyboard HID collections.
    pub connected_keyboards: usize,
    /// Whether an Apple keyboard is currently connected.
    pub is_connected: bool,
    /// Connection transport ("USB", "Bluetooth", etc.) if connected.
    pub transport: Option<String>,
    /// Current battery level percentage (`0..=100`), if known.
    pub battery_percent: Option<u8>,
    /// Name of the currently active configuration profile.
    pub active_profile: String,
    /// Top-row mode for the active profile (Media keys first or F-keys first).
    pub top_row_mode: TopRowMode,
    /// Whether remapping is active.
    pub enabled: bool,
    /// Whether remapping is paused.
    pub paused: bool,
    /// Whether game mode auto-pause is currently active (F9.2).
    #[serde(default)]
    pub game_mode_active: bool,
}

/// Request commands sent by clients (GUI, CLI, tests) over the named pipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum IpcRequest {
    /// Reload config from `%APPDATA%\MagicBridge\config.toml` and recompile active table.
    #[serde(alias = "Reload", alias = "reload")]
    Reload,
    /// Switch active profile immediately.
    #[serde(alias = "SwitchProfile", alias = "switch_profile")]
    SwitchProfile {
        /// Name of profile to activate.
        #[serde(alias = "profile")]
        name: String,
    },
    /// Query current status of the daemon.
    #[serde(alias = "GetStatus", alias = "get_status", alias = "status")]
    GetStatus,
    /// Set remapping pause state.
    #[serde(alias = "Pause", alias = "pause")]
    Pause {
        /// Desired pause state (`true` = paused, `false` = active).
        #[serde(default = "default_true")]
        paused: bool,
    },
}

const fn default_true() -> bool {
    true
}

impl IpcRequest {
    /// Parses an IPC request from a string, supporting JSON and TOML formats.
    pub fn parse(s: &str) -> Result<Self, IpcError> {
        let trimmed = s.trim();
        // 1. Try flexible untagged JSON
        if let Ok(any_req) = serde_json::from_str::<AnyIpcRequest>(trimmed) {
            return Ok(any_req.into());
        }
        // 2. Try direct JSON
        if let Ok(req) = serde_json::from_str::<IpcRequest>(trimmed) {
            return Ok(req);
        }
        // 3. Try direct TOML
        if let Ok(req) = toml::from_str::<IpcRequest>(trimmed) {
            return Ok(req);
        }
        // 4. Try flexible TOML
        if let Ok(any_req) = toml::from_str::<AnyIpcRequest>(trimmed) {
            return Ok(any_req.into());
        }

        Err(IpcError::Protocol(format!(
            "Failed to parse request payload as JSON or TOML: {trimmed}"
        )))
    }
}

/// Flexible intermediate format to accept various JSON structures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum AnyIpcRequest {
    ByCommand(RequestByCommand),
    ByType(RequestByType),
    SimpleString(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum RequestByCommand {
    #[serde(alias = "Reload", alias = "reload")]
    Reload,
    #[serde(alias = "SwitchProfile", alias = "switch_profile")]
    SwitchProfile {
        #[serde(alias = "profile")]
        name: String,
    },
    #[serde(
        alias = "GetStatus",
        alias = "get_status",
        alias = "status",
        alias = "Status"
    )]
    GetStatus,
    #[serde(alias = "Pause", alias = "pause")]
    Pause {
        #[serde(default = "default_true")]
        paused: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RequestByType {
    #[serde(alias = "Reload", alias = "reload")]
    Reload,
    #[serde(alias = "SwitchProfile", alias = "switch_profile")]
    SwitchProfile {
        #[serde(alias = "profile")]
        name: String,
    },
    #[serde(
        alias = "GetStatus",
        alias = "get_status",
        alias = "status",
        alias = "Status"
    )]
    GetStatus,
    #[serde(alias = "Pause", alias = "pause")]
    Pause {
        #[serde(default = "default_true")]
        paused: bool,
    },
}

impl From<AnyIpcRequest> for IpcRequest {
    fn from(any: AnyIpcRequest) -> Self {
        match any {
            AnyIpcRequest::ByCommand(cmd) => match cmd {
                RequestByCommand::Reload => IpcRequest::Reload,
                RequestByCommand::SwitchProfile { name } => IpcRequest::SwitchProfile { name },
                RequestByCommand::GetStatus => IpcRequest::GetStatus,
                RequestByCommand::Pause { paused } => IpcRequest::Pause { paused },
            },
            AnyIpcRequest::ByType(t) => match t {
                RequestByType::Reload => IpcRequest::Reload,
                RequestByType::SwitchProfile { name } => IpcRequest::SwitchProfile { name },
                RequestByType::GetStatus => IpcRequest::GetStatus,
                RequestByType::Pause { paused } => IpcRequest::Pause { paused },
            },
            AnyIpcRequest::SimpleString(s) => match s.trim().to_ascii_lowercase().as_str() {
                "reload" => IpcRequest::Reload,
                "getstatus" | "get_status" | "status" => IpcRequest::GetStatus,
                "pause" => IpcRequest::Pause { paused: true },
                "unpause" | "resume" => IpcRequest::Pause { paused: false },
                _ => IpcRequest::GetStatus,
            },
        }
    }
}

/// Response returned to the client over the named pipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcResponse {
    /// Whether the request succeeded.
    pub success: bool,
    /// Optional status or confirmation message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional error description if `success == false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Status snapshot included on successful query or state change.
    #[serde(flatten)]
    pub status: Option<DaemonStatus>,
}

impl IpcResponse {
    /// Constructs a successful response with status and optional message.
    #[must_use]
    pub fn ok(status: DaemonStatus, message: Option<String>) -> Self {
        Self {
            success: true,
            message,
            error: None,
            status: Some(status),
        }
    }

    /// Constructs a successful status-only response.
    #[must_use]
    pub fn status(status: DaemonStatus) -> Self {
        Self {
            success: true,
            message: None,
            error: None,
            status: Some(status),
        }
    }

    /// Constructs an error response.
    #[must_use]
    pub fn err(err: impl Into<String>) -> Self {
        Self {
            success: false,
            message: None,
            error: Some(err.into()),
            status: None,
        }
    }
}

/// IPC-related error conditions.
#[derive(Debug)]
pub enum IpcError {
    /// Standard I/O error.
    Io(io::Error),
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// TOML parsing error.
    Toml(toml::de::Error),
    /// Windows system API error.
    Windows(windows::core::Error),
    /// Protocol or validation error.
    Protocol(String),
    /// IPC server is closed or shut down.
    ServerClosed,
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Toml(e) => write!(f, "TOML error: {e}"),
            Self::Windows(e) => write!(f, "Windows API error: {e}"),
            Self::Protocol(msg) => write!(f, "Protocol error: {msg}"),
            Self::ServerClosed => write!(f, "IPC server closed"),
        }
    }
}

impl std::error::Error for IpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Toml(e) => Some(e),
            Self::Windows(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for IpcError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<toml::de::Error> for IpcError {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e)
    }
}

impl From<windows::core::Error> for IpcError {
    fn from(e: windows::core::Error) -> Self {
        Self::Windows(e)
    }
}

/// Dispatches an incoming [`IpcRequest`] against daemon state and updates [`AtomicMappingTable`].
pub fn handle_request(
    request: &IpcRequest,
    shared_state: &Arc<Mutex<DaemonState>>,
    atomic_table: &AtomicMappingTable,
) -> IpcResponse {
    let mut state = match shared_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    match request {
        IpcRequest::GetStatus => IpcResponse::status(state.status()),
        IpcRequest::Pause { paused } => {
            state.enabled = !paused;
            atomic_table.set_enabled(!paused);
            state.save_config();
            IpcResponse::ok(
                state.status(),
                Some(format!("Remapping engine pause state set to {paused}")),
            )
        }
        IpcRequest::SwitchProfile { name } => {
            if state.config.profiles.contains_key(name) {
                state.set_active_profile(name);
                let new_table = state.compile_active_profile();
                atomic_table.store(new_table);
                IpcResponse::ok(
                    state.status(),
                    Some(format!("Switched active profile to '{name}'")),
                )
            } else {
                IpcResponse::err(format!("Profile '{name}' does not exist"))
            }
        }
        IpcRequest::Reload => {
            let path = state.config_path.clone().unwrap_or_else(|| {
                magicbridge_core::default_config_path()
                    .unwrap_or_else(|_| PathBuf::from("config.toml"))
            });
            match Config::load_from_path(&path) {
                Ok(new_config) => {
                    state.config = new_config;
                    let new_table = state.compile_active_profile();
                    atomic_table.store(new_table);
                    IpcResponse::ok(
                        state.status(),
                        Some("Configuration reloaded successfully".to_string()),
                    )
                }
                Err(e) => {
                    IpcResponse::err(format!("Failed to reload configuration from {path:?}: {e}"))
                }
            }
        }
    }
}

/// Helper to read newline-terminated lines from a named pipe handle.
struct PipeReader {
    handle: HANDLE,
    buffer: Vec<u8>,
}

impl PipeReader {
    fn new(handle: HANDLE) -> Self {
        Self {
            handle,
            buffer: Vec::with_capacity(512),
        }
    }

    fn read_line(&mut self) -> Result<Option<String>, IpcError> {
        let mut chunk = [0u8; 512];
        loop {
            // Check if buffer already contains newline
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let mut line_bytes = self.buffer.drain(..=pos).collect::<Vec<u8>>();
                if line_bytes.last() == Some(&b'\n') {
                    line_bytes.pop();
                }
                if line_bytes.last() == Some(&b'\r') {
                    line_bytes.pop();
                }
                let line_str = String::from_utf8(line_bytes)
                    .map_err(|e| IpcError::Protocol(format!("Invalid UTF-8 in request: {e}")))?;
                return Ok(Some(line_str));
            }

            let mut bytes_read = 0u32;
            // SAFETY: ReadFile reads up to `chunk.len()` bytes from the pipe handle into `chunk`.
            let res =
                unsafe { ReadFile(self.handle, Some(&mut chunk), Some(&mut bytes_read), None) };

            match res {
                Ok(()) => {
                    if bytes_read == 0 {
                        // EOF / connection closed by client
                        if self.buffer.is_empty() {
                            return Ok(None);
                        }
                        let mut line_bytes = std::mem::take(&mut self.buffer);
                        if line_bytes.last() == Some(&b'\r') {
                            line_bytes.pop();
                        }
                        let line_str = String::from_utf8(line_bytes).map_err(|e| {
                            IpcError::Protocol(format!("Invalid UTF-8 in request: {e}"))
                        })?;
                        return Ok(Some(line_str));
                    }
                    self.buffer.extend_from_slice(&chunk[..bytes_read as usize]);
                }
                Err(e) => {
                    let code = e.code();
                    if code == ERROR_BROKEN_PIPE.to_hresult() || code == ERROR_NO_DATA.to_hresult()
                    {
                        if self.buffer.is_empty() {
                            return Ok(None);
                        }
                        let mut line_bytes = std::mem::take(&mut self.buffer);
                        if line_bytes.last() == Some(&b'\r') {
                            line_bytes.pop();
                        }
                        let line_str = String::from_utf8(line_bytes).map_err(|e| {
                            IpcError::Protocol(format!("Invalid UTF-8 in request: {e}"))
                        })?;
                        return Ok(Some(line_str));
                    }
                    return Err(IpcError::Windows(e));
                }
            }
        }
    }
}

/// Helper to write a newline-terminated string to a named pipe handle.
fn write_line(handle: HANDLE, line: &str) -> Result<(), IpcError> {
    let mut data = line.as_bytes().to_vec();
    if !data.ends_with(b"\n") {
        data.push(b'\n');
    }
    let mut bytes_written = 0u32;
    // SAFETY: WriteFile writes bytes from `data` into the pipe handle.
    unsafe {
        WriteFile(handle, Some(&data), Some(&mut bytes_written), None)?;
        // SAFETY: FlushFileBuffers ensures all buffered data is transmitted to the client.
        FlushFileBuffers(handle)?;
    }
    Ok(())
}

/// Server handle returned when launching the Named Pipe IPC listener.
pub struct IpcServerHandle {
    pipe_name: String,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl IpcServerHandle {
    /// Stops the IPC server thread cleanly and unblocks any waiting pipe connection.
    pub fn stop(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            // Wake up ConnectNamedPipe by issuing a connection
            let wide: Vec<u16> = self
                .pipe_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: CreateFileW wakes up any pending ConnectNamedPipe so the server thread can observe running==false.
            unsafe {
                let _ = CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                    FILE_SHARE_MODE(0),
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                );
            }
            if let Some(th) = self.thread_handle.take() {
                let _ = th.join();
            }
        }
    }

    /// Returns `true` if the IPC server is currently active.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for IpcServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Named pipe IPC server listening for GUI/CLI commands.
pub struct IpcServer;

impl IpcServer {
    /// Starts the IPC server on the default pipe name (`DEFAULT_PIPE_NAME`).
    pub fn start_default(
        state: Arc<Mutex<DaemonState>>,
        atomic_table: AtomicMappingTable,
        hwnd: Option<NotifyHwnd>,
    ) -> Result<IpcServerHandle, IpcError> {
        Self::start(DEFAULT_PIPE_NAME, state, atomic_table, hwnd)
    }

    /// Starts the IPC server on a custom pipe name (useful for isolated tests).
    pub fn start(
        pipe_name: &str,
        state: Arc<Mutex<DaemonState>>,
        atomic_table: AtomicMappingTable,
        hwnd: Option<NotifyHwnd>,
    ) -> Result<IpcServerHandle, IpcError> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let pipe_name_str = pipe_name.to_string();
        let pipe_name_clone = pipe_name_str.clone();

        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        let thread_handle = thread::Builder::new()
            .name("MagicBridge_IPC".to_string())
            .spawn(move || {
                Self::server_loop(
                    &pipe_name_clone,
                    running_clone,
                    state,
                    atomic_table,
                    hwnd,
                    ready_tx,
                );
            })?;

        // Wait until server loop has created the initial pipe instance
        let _ = ready_rx.recv();

        Ok(IpcServerHandle {
            pipe_name: pipe_name_str,
            running,
            thread_handle: Some(thread_handle),
        })
    }

    fn server_loop(
        pipe_name: &str,
        running: Arc<AtomicBool>,
        shared_state: Arc<Mutex<DaemonState>>,
        atomic_table: AtomicMappingTable,
        hwnd: Option<NotifyHwnd>,
        ready_tx: std::sync::mpsc::Sender<()>,
    ) {
        let wide_name: Vec<u16> = pipe_name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut ready_signalled = false;

        while running.load(Ordering::SeqCst) {
            // SAFETY: CreateNamedPipeW creates an instance of the named pipe with duplex byte mode.
            let pipe_handle = unsafe {
                CreateNamedPipeW(
                    PCWSTR(wide_name.as_ptr()),
                    PIPE_ACCESS_DUPLEX,
                    NAMED_PIPE_MODE(PIPE_TYPE_BYTE.0 | PIPE_READMODE_BYTE.0 | PIPE_WAIT.0),
                    PIPE_UNLIMITED_INSTANCES,
                    4096,
                    4096,
                    0,
                    None,
                )
            };

            if pipe_handle == INVALID_HANDLE_VALUE {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }

            if !ready_signalled {
                let _ = ready_tx.send(());
                ready_signalled = true;
            }

            // SAFETY: ConnectNamedPipe blocks waiting for a client to connect.
            let connect_res = unsafe { ConnectNamedPipe(pipe_handle, None) };
            let is_connected = match connect_res {
                Ok(()) => true,
                Err(e) => e.code() == ERROR_PIPE_CONNECTED.to_hresult(),
            };

            if !running.load(Ordering::SeqCst) {
                // SAFETY: Clean up pipe handle on shutdown.
                unsafe {
                    let _ = DisconnectNamedPipe(pipe_handle);
                    let _ = CloseHandle(pipe_handle);
                }
                break;
            }

            if is_connected {
                let mut reader = PipeReader::new(pipe_handle);
                while running.load(Ordering::SeqCst) {
                    match reader.read_line() {
                        Ok(Some(line)) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            let response = match IpcRequest::parse(trimmed) {
                                Ok(req) => {
                                    let resp = handle_request(&req, &shared_state, &atomic_table);
                                    if let Some(h) = hwnd {
                                        // SAFETY: Notify message window that state changed so tray tooltip refreshes.
                                        unsafe {
                                            let _ = PostMessageW(
                                                Some(h.0),
                                                WM_IPC_STATE_UPDATED,
                                                WPARAM(0),
                                                LPARAM(0),
                                            );
                                        }
                                    }
                                    resp
                                }
                                Err(e) => IpcResponse::err(format!("Invalid request: {e}")),
                            };

                            if let Ok(resp_json) = serde_json::to_string(&response) {
                                let _ = write_line(pipe_handle, &resp_json);
                            }
                        }
                        Ok(None) => break, // client disconnected
                        Err(_) => break,   // error / broken pipe
                    }
                }
            }

            // SAFETY: Disconnect client and close instance handle.
            unsafe {
                let _ = DisconnectNamedPipe(pipe_handle);
                let _ = CloseHandle(pipe_handle);
            }
        }
    }
}

/// Lightweight IPC client for GUI, CLI, and integration tests.
#[derive(Debug, Clone)]
pub struct IpcClient {
    pipe_name: String,
}

impl IpcClient {
    /// Creates a new client connected to the given named pipe.
    #[must_use]
    pub fn new(pipe_name: impl Into<String>) -> Self {
        Self {
            pipe_name: pipe_name.into(),
        }
    }

    /// Creates a new client connected to the default agent pipe (`DEFAULT_PIPE_NAME`).
    #[must_use]
    pub fn default_client() -> Self {
        Self::new(DEFAULT_PIPE_NAME)
    }

    /// Sends an IPC request and waits for the response.
    pub fn send(&self, request: &IpcRequest) -> Result<IpcResponse, IpcError> {
        let wide: Vec<u16> = self
            .pipe_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let start = std::time::Instant::now();
        let handle = loop {
            // SAFETY: CreateFileW connects to the named pipe instance.
            let h = unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                    FILE_SHARE_MODE(0),
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                )
            };

            match h {
                Ok(handle) => break handle,
                Err(e) => {
                    if start.elapsed() >= std::time::Duration::from_millis(2000) {
                        return Err(IpcError::Windows(e));
                    }
                    // SAFETY: WaitNamedPipeW waits for an instance of the named pipe to become available.
                    unsafe {
                        let _ = WaitNamedPipeW(PCWSTR(wide.as_ptr()), 50);
                    }
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        };

        let req_json = serde_json::to_string(request)?;
        let write_res = write_line(handle, &req_json);
        if let Err(e) = write_res {
            // SAFETY: Close handle on write failure.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(e);
        }

        let mut reader = PipeReader::new(handle);
        let resp_line = reader.read_line();
        // SAFETY: Close handle after reading response.
        unsafe {
            let _ = CloseHandle(handle);
        }

        match resp_line? {
            Some(line) => {
                let resp: IpcResponse = serde_json::from_str(&line)?;
                Ok(resp)
            }
            None => Err(IpcError::Protocol(
                "Agent closed connection without sending a response".to_string(),
            )),
        }
    }

    /// Convenience query for daemon status.
    pub fn get_status(&self) -> Result<DaemonStatus, IpcError> {
        let resp = self.send(&IpcRequest::GetStatus)?;
        if resp.success {
            resp.status.ok_or_else(|| {
                IpcError::Protocol("Server reported success but omitted status payload".to_string())
            })
        } else {
            Err(IpcError::Protocol(
                resp.error
                    .unwrap_or_else(|| "Unknown server error".to_string()),
            ))
        }
    }

    /// Convenience command to reload configuration.
    pub fn reload(&self) -> Result<IpcResponse, IpcError> {
        self.send(&IpcRequest::Reload)
    }

    /// Convenience command to switch active profile.
    pub fn switch_profile(&self, name: &str) -> Result<IpcResponse, IpcError> {
        self.send(&IpcRequest::SwitchProfile {
            name: name.to_string(),
        })
    }

    /// Convenience command to toggle pause state.
    pub fn pause(&self, paused: bool) -> Result<IpcResponse, IpcError> {
        self.send(&IpcRequest::Pause { paused })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::NamedProfile;

    #[test]
    fn test_ipc_request_parsing_json_commands() {
        // Tagged JSON
        let req1 = IpcRequest::parse(r#"{"command": "reload"}"#).unwrap();
        assert_eq!(req1, IpcRequest::Reload);

        let req2 = IpcRequest::parse(r#"{"command": "get_status"}"#).unwrap();
        assert_eq!(req2, IpcRequest::GetStatus);

        let req3 =
            IpcRequest::parse(r#"{"command": "switch_profile", "name": "Work (US)"}"#).unwrap();
        assert_eq!(
            req3,
            IpcRequest::SwitchProfile {
                name: "Work (US)".to_string()
            }
        );

        let req4 = IpcRequest::parse(r#"{"command": "pause", "paused": true}"#).unwrap();
        assert_eq!(req4, IpcRequest::Pause { paused: true });

        let req5 = IpcRequest::parse(r#"{"command": "pause", "paused": false}"#).unwrap();
        assert_eq!(req5, IpcRequest::Pause { paused: false });
    }

    #[test]
    fn test_ipc_request_parsing_aliases_and_cases() {
        // CamelCase and aliases
        let r1 = IpcRequest::parse(r#"{"command": "Reload"}"#).unwrap();
        assert_eq!(r1, IpcRequest::Reload);

        let r2 = IpcRequest::parse(r#"{"command": "GetStatus"}"#).unwrap();
        assert_eq!(r2, IpcRequest::GetStatus);

        let r3 = IpcRequest::parse(r#"{"type": "GetStatus"}"#).unwrap();
        assert_eq!(r3, IpcRequest::GetStatus);

        let r4 =
            IpcRequest::parse(r#"{"type": "SwitchProfile", "profile": "Default (IT)"}"#).unwrap();
        assert_eq!(
            r4,
            IpcRequest::SwitchProfile {
                name: "Default (IT)".to_string()
            }
        );

        // Simple strings
        let r5 = IpcRequest::parse(r#""reload""#).unwrap();
        assert_eq!(r5, IpcRequest::Reload);

        let r6 = IpcRequest::parse(r#""status""#).unwrap();
        assert_eq!(r6, IpcRequest::GetStatus);
    }

    #[test]
    fn test_ipc_request_parsing_toml() {
        let toml_req = "command = \"reload\"\n";
        let parsed = IpcRequest::parse(toml_req).unwrap();
        assert_eq!(parsed, IpcRequest::Reload);

        let toml_switch = "command = \"switch_profile\"\nname = \"Default (US)\"\n";
        let parsed_switch = IpcRequest::parse(toml_switch).unwrap();
        assert_eq!(
            parsed_switch,
            IpcRequest::SwitchProfile {
                name: "Default (US)".to_string()
            }
        );
    }

    #[test]
    fn test_ipc_response_serialization_flattened_status() {
        let status = DaemonStatus {
            agent_running: true,
            connected_keyboards: 1,
            is_connected: true,
            transport: Some("Bluetooth".to_string()),
            battery_percent: Some(92),
            active_profile: "Default (IT)".to_string(),
            top_row_mode: TopRowMode::MediaKeysFirst,
            enabled: true,
            paused: false,
            game_mode_active: false,
        };

        let response = IpcResponse::ok(status.clone(), Some("Operation OK".to_string()));
        let json = serde_json::to_string(&response).unwrap();

        // Check that fields appear at root due to #[serde(flatten)]
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"active_profile\":\"Default (IT)\""));
        assert!(json.contains("\"battery_percent\":92"));
        assert!(json.contains("\"paused\":false"));

        // Verify roundtrip deserialization to IpcResponse
        let roundtrip: IpcResponse = serde_json::from_str(&json).unwrap();
        assert!(roundtrip.success);
        assert_eq!(roundtrip.status, Some(status.clone()));

        // Verify that client can also deserialize directly into DaemonStatus
        let direct_status: DaemonStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(direct_status, status);
    }

    #[test]
    fn test_atomic_mapping_table_operations() {
        let initial_profile = NamedProfile::default_it();
        let initial_table = magicbridge_core::compile_profile(&initial_profile);

        let atomic_table = AtomicMappingTable::new(initial_table);
        assert!(atomic_table.is_enabled());
        assert!(!atomic_table.is_paused());

        // Test zero-lock loading
        {
            let guard = atomic_table.load();
            assert_eq!(guard.fn_vk(), Some(magicbridge_core::VK_RMENU));
        }

        // Test atomic swap with US profile
        let us_profile = NamedProfile::default_us();
        let us_table = magicbridge_core::compile_profile(&us_profile);
        atomic_table.store(us_table);

        {
            let guard = atomic_table.load();
            assert_eq!(guard.fn_vk(), Some(magicbridge_core::VK_RMENU));
        }

        // Test pause toggle
        atomic_table.set_enabled(false);
        assert!(!atomic_table.is_enabled());
        assert!(atomic_table.is_paused());

        atomic_table.set_enabled(true);
        assert!(atomic_table.is_enabled());
    }

    #[test]
    fn test_atomic_mapping_table_concurrent_reads() {
        let profile = NamedProfile::default_it();
        let table = magicbridge_core::compile_profile(&profile);
        let atomic_table = AtomicMappingTable::new(table);

        let mut handles = Vec::new();
        // Spawn 4 reader threads
        for _ in 0..4 {
            let table_clone = atomic_table.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10_000 {
                    let guard = table_clone.load();
                    let action = guard.lookup(magicbridge_core::VK_F13, false);
                    assert!(!action.is_passthrough());
                }
            }));
        }

        // Writer thread concurrently swapping tables
        let table_writer = atomic_table.clone();
        let writer_handle = thread::spawn(move || {
            let mut alt_profile = NamedProfile::default_us();
            for i in 0..100 {
                if i % 2 == 0 {
                    alt_profile.top_row_mode = TopRowMode::MediaKeysFirst;
                } else {
                    alt_profile.top_row_mode = TopRowMode::FKeysFirst;
                }
                table_writer.store(magicbridge_core::compile_profile(&alt_profile));
            }
        });

        writer_handle.join().unwrap();
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_handle_request_lifecycle() {
        let mut config = Config::default();
        let us_profile = NamedProfile::default_us();
        config.add_profile(us_profile);

        let state = DaemonState::new(config, None);
        let shared_state = Arc::new(Mutex::new(state));
        let atomic_table = AtomicMappingTable::new(CompiledMappingTable::default());

        // 1. GetStatus
        let resp = handle_request(&IpcRequest::GetStatus, &shared_state, &atomic_table);
        assert!(resp.success);
        let st = resp.status.unwrap();
        assert_eq!(st.active_profile, "Default (IT)");
        assert!(st.enabled);

        // 2. Pause { paused: true }
        let resp_pause = handle_request(
            &IpcRequest::Pause { paused: true },
            &shared_state,
            &atomic_table,
        );
        assert!(resp_pause.success);
        assert!(resp_pause.status.unwrap().paused);
        assert!(!atomic_table.is_enabled());

        // 3. SwitchProfile to valid
        let resp_switch = handle_request(
            &IpcRequest::SwitchProfile {
                name: "Default (US)".to_string(),
            },
            &shared_state,
            &atomic_table,
        );
        assert!(resp_switch.success);
        assert_eq!(resp_switch.status.unwrap().active_profile, "Default (US)");
        // Table should have been compiled and updated
        assert_eq!(
            atomic_table.load().fn_vk(),
            Some(magicbridge_core::VK_RMENU)
        );

        // 4. SwitchProfile to invalid
        let resp_invalid = handle_request(
            &IpcRequest::SwitchProfile {
                name: "NonExistent".to_string(),
            },
            &shared_state,
            &atomic_table,
        );
        assert!(!resp_invalid.success);
        assert!(resp_invalid.error.unwrap().contains("does not exist"));
    }

    #[test]
    fn test_named_pipe_ipc_server_client_roundtrip() {
        use std::sync::atomic::AtomicU64;
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pipe_name = format!(
            r"\\.\pipe\MagicBridge_Test_Pipe_{}_{}",
            std::process::id(),
            id
        );

        let mut config = Config::default();
        let us_profile = NamedProfile::default_us();
        config.add_profile(us_profile);

        let state = DaemonState::new(config, None);
        let shared_state = Arc::new(Mutex::new(state));
        let atomic_table = AtomicMappingTable::new(CompiledMappingTable::default());

        let mut server = IpcServer::start(
            &pipe_name,
            Arc::clone(&shared_state),
            atomic_table.clone(),
            None,
        )
        .unwrap();

        assert!(server.is_running());

        let client = IpcClient::new(&pipe_name);

        // 1. Get status over named pipe
        let status = client.get_status().unwrap();
        assert!(status.agent_running);
        assert_eq!(status.active_profile, "Default (IT)");
        assert!(status.enabled);
        assert!(!status.paused);

        // 2. Switch profile over named pipe
        let switch_resp = client.switch_profile("Default (US)").unwrap();
        assert!(switch_resp.success);
        assert_eq!(switch_resp.status.unwrap().active_profile, "Default (US)");
        assert_eq!(
            shared_state.lock().unwrap().active_profile_name(),
            "Default (US)"
        );

        // 3. Pause over named pipe
        let pause_resp = client.pause(true).unwrap();
        assert!(pause_resp.success);
        assert!(pause_resp.status.unwrap().paused);
        assert!(!atomic_table.is_enabled());

        // 4. Resume over named pipe
        let resume_resp = client.pause(false).unwrap();
        assert!(resume_resp.success);
        assert!(!resume_resp.status.unwrap().paused);
        assert!(atomic_table.is_enabled());

        // 5. Clean shutdown
        server.stop();
        assert!(!server.is_running());
    }
}
