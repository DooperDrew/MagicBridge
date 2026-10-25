//! Diagnostic debug logger for MagicBridge daemon (NFR-Q2).
//!
//! Provides zero-overhead logging when disabled (single relaxed atomic check),
//! and writes diagnostic traces to `%APPDATA%\\MagicBridge\\debug.log` as well as
//! an in-memory circular ring buffer when enabled.
//!
//! CRITICAL: In accordance with `NFR-P3`, logger calls MUST NEVER be placed
//! in the low-level keyboard hook callback (`WH_KEYBOARD_LL`).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

/// Whether debug logging is currently enabled.
static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Configured destination path for the debug log file.
static LOG_FILE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// In-memory ring buffer of recent log entries.
static MEMORY_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Maximum number of entries retained in the in-memory log buffer.
pub const MAX_MEMORY_LOG_ENTRIES: usize = 200;

/// Initializes the diagnostic logger with the specified state and file path.
pub fn init(enabled: bool, path: Option<PathBuf>) {
    if let Some(p) = path {
        set_log_path(p);
    }
    set_enabled(enabled);
}

/// Sets whether diagnostic logging is active.
pub fn set_enabled(enabled: bool) {
    LOGGING_ENABLED.store(enabled, Ordering::Release);
}

/// Returns whether diagnostic logging is currently active.
#[inline]
pub fn is_enabled() -> bool {
    LOGGING_ENABLED.load(Ordering::Relaxed)
}

/// Sets the filesystem destination path for the debug log file.
pub fn set_log_path(path: impl Into<PathBuf>) {
    let mut guard = LOG_FILE_PATH.lock().unwrap();
    *guard = Some(path.into());
}

/// Returns the currently configured debug log path, if any.
#[must_use]
pub fn get_log_path() -> Option<PathBuf> {
    let guard = LOG_FILE_PATH.lock().unwrap();
    guard.clone()
}

/// Writes a diagnostic log entry if logging is enabled.
///
/// Fast-path: When logging is disabled, this returns immediately with zero heap allocations,
/// zero locks, and zero I/O overhead.
pub fn log(level: &str, target: &str, message: &str) {
    if !is_enabled() {
        return;
    }

    let timestamp = format_timestamp();
    let entry = format!("[{timestamp}] [{level}] [{target}] {message}");

    // 1. Append to in-memory buffer
    if let Ok(mut mem_guard) = MEMORY_LOG.lock() {
        if mem_guard.len() >= MAX_MEMORY_LOG_ENTRIES {
            mem_guard.remove(0);
        }
        mem_guard.push(entry.clone());
    }

    // 2. Append to log file on disk if path configured
    let path = {
        let guard = LOG_FILE_PATH.lock().unwrap();
        guard.clone()
    };

    if let Some(ref file_path) = path {
        append_to_file(file_path, &entry);
    }
}

/// Appends a line to the specified file path, creating parent directories if necessary.
fn append_to_file(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

/// Formats the current UTC/local system time into a readable timestamp string.
fn format_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;

    format!("{h:02}:{m:02}:{s:02}.{millis:03}")
}

/// Returns a clone of all recent in-memory log entries.
#[must_use]
pub fn get_recent_logs() -> Vec<String> {
    if let Ok(guard) = MEMORY_LOG.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
}

/// Clears the in-memory log buffer.
pub fn clear_memory_logs() {
    if let Ok(mut guard) = MEMORY_LOG.lock() {
        guard.clear();
    }
}

/// Macro for formatted diagnostic logging at INFO level.
#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)*) => {
        if $crate::logger::is_enabled() {
            $crate::logger::log("INFO", $target, &format!($($arg)*));
        }
    };
}

/// Macro for formatted diagnostic logging at WARN level.
#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)*) => {
        if $crate::logger::is_enabled() {
            $crate::logger::log("WARN", $target, &format!($($arg)*));
        }
    };
}

/// Macro for formatted diagnostic logging at ERROR level.
#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)*) => {
        if $crate::logger::is_enabled() {
            $crate::logger::log("ERROR", $target, &format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_logger_disabled_by_default_produces_no_logs() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_enabled(false);
        clear_memory_logs();

        log("INFO", "test", "should not appear");
        assert!(get_recent_logs().is_empty());
    }

    #[test]
    fn test_logger_enabled_captures_in_memory_and_file() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("mb_log_test_{}", std::process::id()));
        let temp_file = temp_dir.join("debug.log");

        init(true, Some(temp_file.clone()));
        clear_memory_logs();

        log("INFO", "test_target", "Hello diagnostic logging!");
        log("WARN", "test_target", "Warning event occurred");

        let entries: Vec<String> = get_recent_logs()
            .into_iter()
            .filter(|e| e.contains("[test_target]"))
            .collect();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("[INFO] [test_target] Hello diagnostic logging!"));
        assert!(entries[1].contains("[WARN] [test_target] Warning event occurred"));

        // Verify disk file
        assert!(temp_file.exists());
        let file_contents = std::fs::read_to_string(&temp_file).expect("read test log file");
        assert!(file_contents.contains("Hello diagnostic logging!"));
        assert!(file_contents.contains("Warning event occurred"));

        // Disable again and verify no new entries
        set_enabled(false);
        log("ERROR", "test_target", "This should be discarded");
        let entries: Vec<String> = get_recent_logs()
            .into_iter()
            .filter(|e| e.contains("[test_target]"))
            .collect();
        assert_eq!(entries.len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_logger_ring_buffer_limit() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_enabled(true);
        clear_memory_logs();

        for i in 0..(MAX_MEMORY_LOG_ENTRIES + 50) {
            log("DEBUG", "stress", &format!("msg {i}"));
        }

        let entries = get_recent_logs();
        assert_eq!(entries.len(), MAX_MEMORY_LOG_ENTRIES);
        assert!(
            entries
                .last()
                .unwrap()
                .contains(&format!("msg {}", MAX_MEMORY_LOG_ENTRIES + 49))
        );

        set_enabled(false);
        clear_memory_logs();
    }
}
