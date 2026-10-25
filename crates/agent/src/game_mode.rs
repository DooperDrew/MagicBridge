//! Game mode auto-pause manager (F9.2).
//!
//! Automatically pauses remapping when a user-specified game executable or a
//! fullscreen exclusive application is active in the foreground, and restores
//! active hook remapping upon returning to desktop/work applications.
//!
//! Uses `SetWinEventHook` (`EVENT_SYSTEM_FOREGROUND`) for event-driven, 0% CPU
//! background tracking (NFR-P2).

use std::sync::atomic::{AtomicIsize, Ordering};

use magicbridge_core::Config;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_SYSTEM_FOREGROUND, GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowRect,
    GetWindowThreadProcessId, PostMessageW, SM_CXSCREEN, SM_CYSCREEN, WINEVENT_OUTOFCONTEXT,
    WM_USER,
};
use windows::core::PWSTR;

/// Custom Win32 message sent to the message window when the foreground window changes.
pub const WM_GAME_MODE_EVENT: u32 = WM_USER + 105;

/// Global atomic storing the message window handle for WinEvent notification.
static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

/// WinEvent callback invoked when the foreground window changes.
unsafe extern "system" fn foreground_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let target = TARGET_HWND.load(Ordering::Relaxed);
    if target != 0 {
        // SAFETY: PostMessageW queues WM_GAME_MODE_EVENT to our hidden agent message window.
        unsafe {
            let _ = PostMessageW(
                Some(HWND(target as *mut _)),
                WM_GAME_MODE_EVENT,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

/// RAII handle for the `EVENT_SYSTEM_FOREGROUND` WinEvent hook.
#[derive(Debug)]
pub struct GameModeTracker {
    hook: Option<HWINEVENTHOOK>,
}

impl GameModeTracker {
    /// Installs the foreground tracking hook, notifying `notify_hwnd` on changes.
    pub fn install(notify_hwnd: HWND) -> Result<Self, windows::core::Error> {
        TARGET_HWND.store(notify_hwnd.0 as isize, Ordering::Release);

        // SAFETY: SetWinEventHook installs an out-of-context hook for EVENT_SYSTEM_FOREGROUND on the thread.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };

        if hook.is_invalid() {
            TARGET_HWND.store(0, Ordering::Release);
            return Err(windows::core::Error::from_thread());
        }

        Ok(Self { hook: Some(hook) })
    }
}

impl Drop for GameModeTracker {
    fn drop(&mut self) {
        if let Some(hook) = self.hook.take() {
            // SAFETY: UnhookWinEvent removes the installed WinEvent hook.
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
        }
        TARGET_HWND.store(0, Ordering::Release);
    }
}

/// Returns `true` if the specified window class belongs to Windows shell or desktop.
#[must_use]
pub fn is_ignored_shell_class(class_name: &str) -> bool {
    let name = class_name.trim();
    name.eq_ignore_ascii_case("Progman")
        || name.eq_ignore_ascii_case("WorkerW")
        || name.eq_ignore_ascii_case("Shell_TrayWnd")
        || name.eq_ignore_ascii_case("Shell_SecondaryTrayWnd")
        || name.eq_ignore_ascii_case("Windows.UI.Core.CoreWindow")
}

/// Returns `true` if the window rectangle covers the primary monitor (fullscreen).
#[must_use]
pub fn is_fullscreen_rect(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    screen_w: i32,
    screen_h: i32,
) -> bool {
    if screen_w <= 0 || screen_h <= 0 {
        return false;
    }
    left <= 0 && top <= 0 && right >= screen_w && bottom >= screen_h
}

/// Returns `true` if `exe_name` matches any configured process in `processes`.
#[must_use]
pub fn matches_process_list(exe_name: &str, processes: &[String]) -> bool {
    let trimmed = exe_name.trim();
    processes.iter().any(|p| {
        let p_trimmed = p.trim();
        p_trimmed.eq_ignore_ascii_case(trimmed)
            || (p_trimmed.ends_with(".exe")
                && p_trimmed[..p_trimmed.len() - 4].eq_ignore_ascii_case(trimmed))
            || (trimmed.ends_with(".exe")
                && trimmed[..trimmed.len() - 4].eq_ignore_ascii_case(p_trimmed))
    })
}

/// Queries the executable image filename for a given window handle.
#[must_use]
pub fn get_window_process_name(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    // SAFETY: Retrieves the thread and process ID for the window.
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    if pid == 0 {
        return None;
    }

    // SAFETY: OpenProcess with PROCESS_QUERY_LIMITED_INFORMATION is standard per-process query rights.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };

    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;

    // SAFETY: QueryFullProcessImageNameW retrieves the full path to the process executable.
    let res = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };

    // SAFETY: CloseHandle closes the opened process handle.
    unsafe {
        let _ = CloseHandle(handle);
    }

    if res.is_ok() && size > 0 {
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        let filename = full_path
            .rsplit(&['\\', '/'][..])
            .next()
            .unwrap_or(&full_path)
            .to_string();
        Some(filename)
    } else {
        None
    }
}

/// Checks whether the current foreground window represents an active game.
#[must_use]
pub fn is_foreground_game(config: &Config) -> bool {
    if !config.game_mode_enabled && config.game_mode_processes.is_empty() {
        return false;
    }

    // SAFETY: GetForegroundWindow returns the window handle currently in the foreground.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return false;
    }

    // 1. Check against process list (F9.2)
    if let Some(proc_name) = get_window_process_name(hwnd)
        && matches_process_list(&proc_name, &config.game_mode_processes)
    {
        return true;
    }

    // 2. If game mode enabled, check if window is fullscreen exclusive (F9.2)
    if config.game_mode_enabled {
        let mut class_buf = [0u16; 256];
        // SAFETY: GetClassNameW retrieves the window class name for hwnd.
        let class_len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
        if class_len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
            if is_ignored_shell_class(&class_name) {
                return false;
            }
        }

        let mut rect = RECT::default();
        // SAFETY: GetWindowRect gets the window bounding box.
        let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
        if ok.is_ok() {
            // SAFETY: GetSystemMetrics gets screen width and height.
            let (screen_w, screen_h) =
                unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };

            if is_fullscreen_rect(
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                screen_w,
                screen_h,
            ) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ignored_shell_class() {
        assert!(is_ignored_shell_class("Progman"));
        assert!(is_ignored_shell_class("progman"));
        assert!(is_ignored_shell_class("WorkerW"));
        assert!(is_ignored_shell_class("Shell_TrayWnd"));
        assert!(is_ignored_shell_class("Shell_SecondaryTrayWnd"));
        assert!(is_ignored_shell_class("Windows.UI.Core.CoreWindow"));

        assert!(!is_ignored_shell_class("Notepad"));
        assert!(!is_ignored_shell_class("UnrealWindow"));
        assert!(!is_ignored_shell_class("UnityWndClass"));
    }

    #[test]
    fn test_is_fullscreen_rect() {
        // Fullscreen matching exactly
        assert!(is_fullscreen_rect(0, 0, 1920, 1080, 1920, 1080));
        // Fullscreen stretching slightly over bounds (borderless fullscreen)
        assert!(is_fullscreen_rect(-1, -1, 1921, 1081, 1920, 1080));

        // Windowed (not fullscreen)
        assert!(!is_fullscreen_rect(100, 100, 1000, 800, 1920, 1080));
        assert!(!is_fullscreen_rect(0, 0, 1900, 1000, 1920, 1080));

        // Invalid screen dimensions
        assert!(!is_fullscreen_rect(0, 0, 1920, 1080, 0, 0));
        assert!(!is_fullscreen_rect(0, 0, 1920, 1080, -1, -1));
    }

    #[test]
    fn test_matches_process_list() {
        let processes = vec![
            "Cyberpunk2077.exe".to_string(),
            "Dota2".to_string(),
            "cs2.exe".to_string(),
        ];

        assert!(matches_process_list("Cyberpunk2077.exe", &processes));
        assert!(matches_process_list("cyberpunk2077.exe", &processes));
        assert!(matches_process_list("Cyberpunk2077", &processes));
        assert!(matches_process_list("dota2.exe", &processes));
        assert!(matches_process_list("Dota2", &processes));
        assert!(matches_process_list("cs2", &processes));

        assert!(!matches_process_list("notepad.exe", &processes));
        assert!(!matches_process_list("explorer.exe", &processes));
        assert!(!matches_process_list("code.exe", &processes));
    }

    #[test]
    fn test_is_foreground_game_when_disabled() {
        let config = Config {
            game_mode_enabled: false,
            game_mode_processes: Vec::new(),
            ..Default::default()
        };

        // When completely disabled, is_foreground_game returns false immediately
        assert!(!is_foreground_game(&config));
    }
}
