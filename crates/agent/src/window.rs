//! Hidden Win32 message window and message pump (F8.2, NFR-P2).

use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA,
    GetMessageW, GetWindowLongPtrW, HCURSOR, HICON, IsWindow, MSG, PostQuitMessage,
    RegisterClassExW, RegisterWindowMessageW, SetWindowLongPtrW, TranslateMessage, WINDOW_EX_STYLE,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NCCREATE, WM_NCDESTROY, WM_RBUTTONUP,
    WNDCLASS_STYLES, WNDCLASSEXW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::state::DaemonState;
use crate::tray::{
    IDM_OPEN_SETTINGS, IDM_PROFILE_BASE, IDM_QUIT, IDM_START_WITH_WINDOWS, IDM_TOGGLE_ENABLED,
    IDM_TOPROW_FKEYS_FIRST, IDM_TOPROW_MEDIA_FIRST, TrayIcon, WM_TRAY_CALLBACK,
};
use magicbridge_core::TopRowMode;

/// Dedicated window class name for the agent's hidden window.
pub const WINDOW_CLASS_NAME: PCWSTR = w!("MagicBridge_MsgWndClass");

/// TaskbarCreated broadcast message string.
pub const TASKBAR_CREATED_STR: PCWSTR = w!("TaskbarCreated");

/// WM_CONTEXTMENU message constant.
pub const WM_CONTEXTMENU_VAL: u32 = 0x007B;

/// Internal state owned by the window procedure.
struct WindowData {
    state: DaemonState,
    tray: Option<TrayIcon>,
    taskbar_created_msg: u32,
}

/// Hidden Win32 message window.
#[derive(Debug)]
pub struct MessageWindow {
    hwnd: HWND,
}

impl MessageWindow {
    /// Registers the window class and creates the hidden message window.
    pub fn create(state: DaemonState) -> Result<Self, windows::core::Error> {
        // SAFETY: RegisterWindowMessageW registers or retrieves the message ID for TaskbarCreated.
        let taskbar_created_msg = unsafe { RegisterWindowMessageW(TASKBAR_CREATED_STR) };

        // SAFETY: GetModuleHandleW(None) returns the HINSTANCE of the current process module.
        let hinstance = unsafe { GetModuleHandleW(None)? };

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: WINDOW_CLASS_NAME,
            hIconSm: HICON::default(),
        };

        // SAFETY: RegisterClassExW registers the window class.
        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            // SAFETY: Queries the last error code for class registration.
            let err = unsafe { GetLastError() };
            if err != ERROR_CLASS_ALREADY_EXISTS {
                return Err(windows::core::Error::from_thread());
            }
        }

        let window_data = Box::new(WindowData {
            state,
            tray: None,
            taskbar_created_msg,
        });
        let raw_ptr = Box::into_raw(window_data);

        // SAFETY: CreateWindowExW creates a hidden top-level window (WS_OVERLAPPED without WS_VISIBLE).
        // It passes `raw_ptr` in lpParam to be set in GWLP_USERDATA by WM_NCCREATE.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WINDOW_CLASS_NAME,
                w!("MagicBridge Agent"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                Some(raw_ptr as *const std::ffi::c_void),
            )
        };

        match hwnd {
            Ok(h) => Ok(Self { hwnd: h }),
            Err(e) => {
                // SAFETY: If window creation failed, reclaim the allocated WindowData to prevent leak.
                unsafe {
                    let _ = Box::from_raw(raw_ptr);
                }
                Err(e)
            }
        }
    }

    /// Returns the raw window handle.
    #[must_use]
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        // SAFETY: Checks if window is still valid before calling DestroyWindow.
        let is_valid = unsafe { IsWindow(Some(self.hwnd)).as_bool() };
        if is_valid {
            // SAFETY: Destroys the window handle.
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

/// Win32 window procedure handling tray notifications and menu commands.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: In WM_NCCREATE, lparam points to CREATESTRUCTW.
        let cs = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let state_ptr = cs.lpCreateParams as *mut WindowData;
        // SAFETY: Store pointer in GWLP_USERDATA.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        }
        // SAFETY: Delegate to default window procedure.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    // SAFETY: Retrieves our pointer stored in GWLP_USERDATA.
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData };
    if state_ptr.is_null() {
        // SAFETY: Delegate to default window procedure before NCCREATE or after NCDESTROY.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    // SAFETY: `state_ptr` is valid, non-null, and only accessed on this message loop thread.
    let data = unsafe { &mut *state_ptr };

    match msg {
        WM_CREATE => {
            match TrayIcon::create(hwnd, &data.state) {
                Ok(tray) => data.tray = Some(tray),
                Err(e) => eprintln!("Failed to initialize system tray icon: {e}"),
            }
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            let event = lparam.0 as u32;
            match event {
                WM_RBUTTONUP | WM_CONTEXTMENU_VAL => {
                    if let Some(ref tray) = data.tray {
                        tray.show_context_menu(&data.state);
                    }
                }
                WM_LBUTTONDBLCLK => {
                    launch_settings();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = wparam.0 & 0xFFFF;
            match cmd_id {
                IDM_TOGGLE_ENABLED => {
                    data.state.toggle_enabled();
                    if let Some(ref tray) = data.tray {
                        tray.update_tooltip(&data.state);
                    }
                }
                IDM_TOPROW_MEDIA_FIRST => {
                    data.state.set_top_row_mode(TopRowMode::MediaKeysFirst);
                    if let Some(ref tray) = data.tray {
                        tray.update_tooltip(&data.state);
                    }
                }
                IDM_TOPROW_FKEYS_FIRST => {
                    data.state.set_top_row_mode(TopRowMode::FKeysFirst);
                    if let Some(ref tray) = data.tray {
                        tray.update_tooltip(&data.state);
                    }
                }
                IDM_OPEN_SETTINGS => {
                    launch_settings();
                }
                IDM_START_WITH_WINDOWS => {
                    data.state.toggle_autostart();
                }
                IDM_QUIT => {
                    // SAFETY: DestroyWindow triggers WM_DESTROY and message loop shutdown.
                    let _ = unsafe { DestroyWindow(hwnd) };
                }
                other if other >= IDM_PROFILE_BASE => {
                    let idx = other - IDM_PROFILE_BASE;
                    let profiles = data.state.profile_names();
                    if let Some(name) = profiles.get(idx) {
                        data.state.set_active_profile(name);
                        if let Some(ref tray) = data.tray {
                            tray.update_tooltip(&data.state);
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Drop tray icon before window is gone.
            data.tray.take();
            // SAFETY: PostQuitMessage posts WM_QUIT to exit message loop cleanly.
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // Reset GWLP_USERDATA.
            // SAFETY: Setting GWLP_USERDATA to 0 so subsequent calls see null.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            // SAFETY: Reclaiming ownership of WindowData originally allocated via Box::into_raw.
            let _ = unsafe { Box::from_raw(state_ptr) };
            // SAFETY: Final non-client destruction.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ if data.taskbar_created_msg != 0 && msg == data.taskbar_created_msg => {
            // Explorer restarted: re-add tray icon.
            if let Some(ref tray) = data.tray {
                tray.re_add(&data.state);
            }
            LRESULT(0)
        }
        _ => {
            // SAFETY: Forward all unhandled messages to DefWindowProcW.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

/// Runs the Win32 message pump until WM_QUIT is posted.
pub fn run_message_pump() -> i32 {
    let mut msg = MSG::default();
    loop {
        // SAFETY: GetMessageW retrieves messages from the calling thread's message queue.
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if ret.0 <= 0 {
            // ret.0 == 0 indicates WM_QUIT; ret.0 == -1 indicates an error.
            break;
        }
        // SAFETY: Translates virtual-key messages into character messages.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    msg.wParam.0 as i32
}

/// Attempts to spawn the settings application (`magicbridge-settings.exe`).
pub fn launch_settings() {
    let mut cmd = None;
    if let Ok(exe_path) = std::env::current_exe() {
        let sibling = exe_path.with_file_name("magicbridge-settings.exe");
        if sibling.exists() {
            cmd = Some(std::process::Command::new(sibling));
        }
    }
    let mut command = cmd.unwrap_or_else(|| std::process::Command::new("magicbridge-settings.exe"));
    let _ = command.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::Config;

    #[test]
    fn message_window_create_and_destroy() {
        let config = Config::default();
        let state = DaemonState::new(config, None);

        let window =
            MessageWindow::create(state).expect("Creating hidden message window should succeed");
        assert!(!window.hwnd().is_invalid());

        drop(window);
    }
}
