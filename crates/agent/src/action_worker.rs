//! Off-thread custom action dispatcher (F5.1, F5.2, NFR-P3).
//!
//! Executes user-configured actions (launching executables, opening URLs,
//! media commands, or dispatching key combinations) on a dedicated background worker
//! thread, preventing any blocking of the low-level input hook thread (`WH_KEYBOARD_LL`).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::{self, JoinHandle};

use arc_swap::ArcSwapOption;
use magicbridge_core::config::CustomAction;
use magicbridge_core::key::{Key, parse_key_name};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, w};

use crate::injector;

/// Maximum number of queued custom action events in the non-blocking ring buffer.
pub const ACTION_CHANNEL_CAPACITY: usize = 64;

/// Lock-free global sender for dispatching actions from the low-level hook callback.
static GLOBAL_ACTION_SENDER: ArcSwapOption<SyncSender<ActionMessage>> =
    ArcSwapOption::const_empty();

/// Internal message passed to the action worker thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMessage {
    /// Execute a registered action by ID from the active compiled table.
    Execute(u32),
    /// Shutdown worker thread cleanly.
    Shutdown,
}

/// Errors that can occur during custom action execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The command string was empty.
    EmptyCommand,
    /// An unknown key name was encountered in a shortcut sequence.
    UnknownKey(String),
    /// An unknown media command was encountered.
    UnknownMediaCommand(String),
    /// Spawning the external process failed.
    ProcessSpawnFailed(String),
    /// ShellExecuteW returned an error code.
    ShellExecuteFailed(usize),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCommand => write!(f, "Empty command string"),
            Self::UnknownKey(k) => write!(f, "Unknown key name in shortcut: {k}"),
            Self::UnknownMediaCommand(m) => write!(f, "Unknown media command: {m}"),
            Self::ProcessSpawnFailed(msg) => write!(f, "Failed to spawn process: {msg}"),
            Self::ShellExecuteFailed(code) => write!(f, "ShellExecuteW failed with code {code}"),
        }
    }
}

impl std::error::Error for ActionError {}

/// Dispatches a custom action ID to the background worker thread.
///
/// Strictly conforms to `NFR-P3`:
/// - Lock-free atomic load via `ArcSwapOption`.
/// - Zero heap allocations on send.
/// - Non-blocking `try_send` into bounded channel.
/// - Never blocks the caller (hook thread).
///
/// Returns `true` if the action was successfully queued.
pub fn dispatch_action(action_id: u32) -> bool {
    let sender_guard = GLOBAL_ACTION_SENDER.load();
    if let Some(ref tx) = *sender_guard {
        tx.try_send(ActionMessage::Execute(action_id)).is_ok()
    } else {
        false
    }
}

/// RAII handle managing the background action worker thread.
///
/// When dropped, sends `ActionMessage::Shutdown`, joins the thread,
/// and clears the global sender.
pub struct ActionWorkerHandle {
    sender: SyncSender<ActionMessage>,
    thread: Option<JoinHandle<()>>,
}

impl ActionWorkerHandle {
    /// Starts the background action worker thread and registers the global sender.
    pub fn start() -> Self {
        let (tx, rx) = sync_channel::<ActionMessage>(ACTION_CHANNEL_CAPACITY);
        GLOBAL_ACTION_SENDER.store(Some(Arc::new(tx.clone())));

        let thread = thread::Builder::new()
            .name("MagicBridge-ActionWorker".to_string())
            .spawn(move || {
                run_action_worker_loop(rx);
            })
            .expect("Failed to spawn MagicBridge action worker thread");

        Self {
            sender: tx,
            thread: Some(thread),
        }
    }

    /// Stops the worker thread and waits for it to finish.
    pub fn stop(mut self) {
        let _ = self.sender.try_send(ActionMessage::Shutdown);
        GLOBAL_ACTION_SENDER.store(None);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ActionWorkerHandle {
    fn drop(&mut self) {
        let _ = self.sender.try_send(ActionMessage::Shutdown);
        GLOBAL_ACTION_SENDER.store(None);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Starts the action worker thread, returning its RAII handle.
#[must_use]
pub fn start_action_worker() -> ActionWorkerHandle {
    ActionWorkerHandle::start()
}

/// Main loop for the action worker thread.
fn run_action_worker_loop(rx: Receiver<ActionMessage>) {
    while let Ok(msg) = rx.recv() {
        match msg {
            ActionMessage::Execute(id) => {
                if let Some(action) = crate::hook::get_active_custom_action(id)
                    && let Err(err) = execute_action(&action)
                {
                    eprintln!("[MagicBridge] Custom action execution error: {err}");
                }
            }
            ActionMessage::Shutdown => {
                break;
            }
        }
    }
}

/// Executes a custom action off-thread (F5.1).
pub fn execute_action(action: &CustomAction) -> Result<(), ActionError> {
    match action {
        CustomAction::Launch { command } => execute_launch(command),
        CustomAction::OpenUrl { url } => execute_open_url(url),
        CustomAction::Shortcut { keys } => execute_shortcut(keys),
        CustomAction::Media { command } => execute_media(command),
    }
}

/// Executes a `Launch` action using `std::process::Command` or `ShellExecuteW`.
pub fn execute_launch(command_str: &str) -> Result<(), ActionError> {
    let trimmed = command_str.trim();
    if trimmed.is_empty() {
        return Err(ActionError::EmptyCommand);
    }

    // If the command is a URL or protocol scheme, open via default shell handler.
    if is_url_or_protocol(trimmed) {
        return execute_open_url(trimmed);
    }

    let tokens = parse_command_line(trimmed);
    if tokens.is_empty() {
        return Err(ActionError::EmptyCommand);
    }

    let program = &tokens[0];
    let args = &tokens[1..];

    let mut cmd = std::process::Command::new(program);
    cmd.args(args);

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) => {
            // Fallback: If direct Command::new fails (e.g. document file or path with spaces
            // requiring shell association), attempt ShellExecuteW.
            if execute_open_url(trimmed).is_ok() {
                Ok(())
            } else {
                Err(ActionError::ProcessSpawnFailed(err.to_string()))
            }
        }
    }
}

/// Opens a URL, file, or document using Windows `ShellExecuteW`.
pub fn execute_open_url(url: &str) -> Result<(), ActionError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ActionError::EmptyCommand);
    }

    let hstr = HSTRING::from(trimmed);

    // SAFETY:
    // ShellExecuteW is called with:
    // - hwnd = None (no owner window).
    // - lpoperation = w!("open") (standard default open verb).
    // - lpfile = &hstr (valid null-terminated UTF-16 string representing URL or path).
    // - lpparameters = None.
    // - lpdirectory = None.
    // - nshowcmd = SW_SHOWNORMAL (standard window display).
    let res = unsafe { ShellExecuteW(None, w!("open"), &hstr, None, None, SW_SHOWNORMAL) };

    // ShellExecuteW returns a value greater than 32 on success.
    let code = res.0 as usize;
    if code > 32 {
        Ok(())
    } else {
        Err(ActionError::ShellExecuteFailed(code))
    }
}

/// Executes a key shortcut sequence (pressing keys down, then releasing them in reverse).
pub fn execute_shortcut(keys: &[String]) -> Result<(), ActionError> {
    if keys.is_empty() {
        return Ok(());
    }

    let mut resolved = Vec::with_capacity(keys.len());
    for k in keys {
        if let Some(vk) = parse_key_name(k) {
            resolved.push(Key::from_vk(vk));
        } else {
            return Err(ActionError::UnknownKey(k.clone()));
        }
    }

    // Press all keys down in forward order
    for key in &resolved {
        injector::send_key_down(key.vk, key.scan_code, key.extended);
    }

    // Release all keys in reverse order
    for key in resolved.iter().rev() {
        injector::send_key_up(key.vk, key.scan_code, key.extended);
    }

    Ok(())
}

/// Executes a media action keypress.
pub fn execute_media(command: &str) -> Result<(), ActionError> {
    let clean = command.trim().to_ascii_lowercase();
    let vk = match clean.as_str() {
        "play_pause" | "play" | "pause" => magicbridge_core::VK_MEDIA_PLAY_PAUSE,
        "next" | "next_track" => magicbridge_core::VK_MEDIA_NEXT_TRACK,
        "prev" | "prev_track" | "previous" => magicbridge_core::VK_MEDIA_PREV_TRACK,
        "stop" => magicbridge_core::VK_MEDIA_STOP,
        "volume_up" | "vol_up" => magicbridge_core::VK_VOLUME_UP,
        "volume_down" | "vol_down" => magicbridge_core::VK_VOLUME_DOWN,
        "mute" | "volume_mute" => magicbridge_core::VK_VOLUME_MUTE,
        _ => {
            if let Some(parsed) = parse_key_name(command) {
                parsed
            } else {
                return Err(ActionError::UnknownMediaCommand(command.to_string()));
            }
        }
    };

    let key = Key::from_vk(vk);
    injector::send_key_down(key.vk, key.scan_code, key.extended);
    injector::send_key_up(key.vk, key.scan_code, key.extended);
    Ok(())
}

/// Returns `true` if the string starts with a common URL scheme or Windows URI.
#[must_use]
pub fn is_url_or_protocol(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("ms-settings:")
        || lower.starts_with("file://")
        || lower.starts_with("www.")
}

/// Parses a command line string into program and argument tokens, respecting quotes.
#[must_use]
pub fn parse_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for c in input.chars() {
        match c {
            '"' | '\'' if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_line() {
        let cmd = "notepad.exe";
        assert_eq!(parse_command_line(cmd), vec!["notepad.exe"]);

        let cmd = "cmd.exe /c \"dir /s\"";
        assert_eq!(parse_command_line(cmd), vec!["cmd.exe", "/c", "dir /s"]);

        let cmd = r#""C:\Program Files\App\app.exe" --debug --name="My App""#;
        assert_eq!(
            parse_command_line(cmd),
            vec![
                r#"C:\Program Files\App\app.exe"#,
                "--debug",
                "--name=My App"
            ]
        );

        let empty = "   ";
        assert!(parse_command_line(empty).is_empty());
    }

    #[test]
    fn test_is_url_or_protocol() {
        assert!(is_url_or_protocol("https://github.com"));
        assert!(is_url_or_protocol("http://localhost:8080"));
        assert!(is_url_or_protocol("mailto:test@example.com"));
        assert!(is_url_or_protocol("ms-settings:bluetooth"));
        assert!(is_url_or_protocol("www.google.com"));
        assert!(!is_url_or_protocol("notepad.exe"));
        assert!(!is_url_or_protocol(r"C:\Windows\notepad.exe"));
    }

    #[test]
    fn test_execute_empty_command() {
        assert_eq!(execute_launch(""), Err(ActionError::EmptyCommand));
        assert_eq!(execute_launch("   "), Err(ActionError::EmptyCommand));
        assert_eq!(execute_open_url(""), Err(ActionError::EmptyCommand));
    }

    #[test]
    fn test_execute_shortcut_validation() {
        let valid = vec!["Ctrl".to_string(), "Alt".to_string(), "Delete".to_string()];
        assert!(execute_shortcut(&valid).is_ok());

        let invalid = vec!["NonExistentKey123".to_string()];
        assert_eq!(
            execute_shortcut(&invalid),
            Err(ActionError::UnknownKey("NonExistentKey123".to_string()))
        );

        let empty: Vec<String> = vec![];
        assert!(execute_shortcut(&empty).is_ok());
    }

    #[test]
    fn test_execute_media_commands() {
        assert!(execute_media("play_pause").is_ok());
        assert!(execute_media("volume_up").is_ok());
        assert!(execute_media("volume_down").is_ok());
        assert!(execute_media("mute").is_ok());
        assert!(execute_media("next").is_ok());
        assert!(execute_media("prev").is_ok());
        assert!(execute_media("stop").is_ok());
        assert_eq!(
            execute_media("invalid_command_xyz"),
            Err(ActionError::UnknownMediaCommand(
                "invalid_command_xyz".to_string()
            ))
        );
    }

    #[test]
    fn test_action_worker_lifecycle() {
        let worker = ActionWorkerHandle::start();
        assert!(dispatch_action(999));
        worker.stop();
        assert!(!dispatch_action(999));
    }
}
