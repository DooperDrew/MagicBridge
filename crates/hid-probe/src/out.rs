//! Output sink (stdout + optional log file) and a high-resolution clock.
//!
//! Every line is flushed immediately so a log survives Ctrl+C.

use std::fs::File;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

static LOG: Mutex<Option<File>> = Mutex::new(None);
static CLOCK: OnceLock<(i64, f64)> = OnceLock::new();

/// Opens (truncates) a log file that receives a copy of every output line.
pub fn set_log_file(path: &str) -> io::Result<()> {
    let file = File::create(path)?;
    *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(file);
    Ok(())
}

/// Writes one line to stdout and to the log file, if any.
pub fn line(text: &str) {
    {
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    }
    if let Some(f) = LOG.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let _ = writeln!(f, "{text}");
        let _ = f.flush();
    }
}

/// `println!`-style macro that goes through [`line`].
#[macro_export]
macro_rules! outln {
    () => { $crate::out::line("") };
    ($($arg:tt)*) => { $crate::out::line(&format!($($arg)*)) };
}

/// Raw performance-counter ticks (cheap; safe to call from the hook).
pub fn ticks() -> i64 {
    let mut t = 0i64;
    // SAFETY: `t` is a valid, writable i64; QPC cannot fail on Windows XP+.
    let _ = unsafe { QueryPerformanceCounter(&mut t) };
    t
}

/// Starts the clock. Call once at startup; later timestamps are relative to it.
pub fn start_clock() {
    let _ = CLOCK.get_or_init(|| {
        let mut freq = 0i64;
        // SAFETY: `freq` is a valid, writable i64.
        let _ = unsafe { QueryPerformanceFrequency(&mut freq) };
        (ticks(), freq.max(1) as f64)
    });
}

/// Converts performance-counter ticks to milliseconds since [`start_clock`].
pub fn ms_since_start(t: i64) -> f64 {
    let (t0, freq) = *CLOCK.get_or_init(|| (t, 1.0));
    (t - t0) as f64 * 1000.0 / freq
}

/// Formats a timestamp column, e.g. `[   1234.567]`.
pub fn stamp(t: i64) -> String {
    format!("[{:>11.3}]", ms_since_start(t))
}

/// Lower-case hex dump with spaces: `01 a2 ff`.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02x}"));
    }
    s
}
