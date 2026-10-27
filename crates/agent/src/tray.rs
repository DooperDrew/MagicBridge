//! System tray icon and context menu management (F8.2, docs/04-ux.md).

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HICON, IDI_APPLICATION, LoadIconW,
    MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, PostMessageW, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::state::DaemonState;
use magicbridge_core::TopRowMode;

/// Custom message ID sent to the window when tray events occur (WM_USER + 1).
pub const WM_TRAY_CALLBACK: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 1;

/// Tray icon ID within this window.
pub const TRAY_ICON_ID: u32 = 1;

/// Menu command IDs.
pub const IDM_TOGGLE_ENABLED: usize = 1001;
pub const IDM_TOPROW_MEDIA_FIRST: usize = 1002;
pub const IDM_TOPROW_FKEYS_FIRST: usize = 1003;
pub const IDM_OPEN_SETTINGS: usize = 1004;
pub const IDM_START_WITH_WINDOWS: usize = 1005;
pub const IDM_QUIT: usize = 1006;
pub const IDM_PROFILE_BASE: usize = 2000;

/// Copies a string slice into a fixed-size UTF-16 null-terminated buffer.
pub fn copy_to_sz_tip(dest: &mut [u16; 128], text: &str) {
    copy_to_sz_buf(dest, text);
}

/// Copies a string slice into a variable-length UTF-16 null-terminated slice buffer.
pub fn copy_to_sz_buf(dest: &mut [u16], text: &str) {
    let max = dest.len().saturating_sub(1);
    let mut i = 0;
    for c in text.encode_utf16() {
        if i >= max {
            break;
        }
        dest[i] = c;
        i += 1;
    }
    if !dest.is_empty() {
        dest[i] = 0;
    }
}

/// System tray icon wrapper.
#[derive(Debug)]
pub struct TrayIcon {
    hwnd: HWND,
    default_hicon: HICON,
    custom_hicon: Option<HICON>,
}

impl TrayIcon {
    /// Creates and displays the system tray icon.
    pub fn create(hwnd: HWND, initial_state: &DaemonState) -> Result<Self, windows::core::Error> {
        // SAFETY: LoadIconW with IDI_APPLICATION loads the standard system application icon.
        let default_hicon = unsafe { LoadIconW(None, IDI_APPLICATION)? };

        let mut custom_hicon = None;
        let active_hicon = if initial_state.config.battery_tray_badge {
            if let Ok(hicon) = crate::tray_icon::create_battery_tray_icon(initial_state) {
                custom_hicon = Some(hicon);
                hicon
            } else {
                default_hicon
            }
        } else {
            default_hicon
        };

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: active_hicon,
            ..Default::default()
        };

        let tooltip = initial_state.format_tooltip();
        copy_to_sz_tip(&mut nid.szTip, &tooltip);

        // SAFETY: `nid` is properly initialized with valid window handle, icon, and callback message.
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &nid).ok()?;
        }

        Ok(Self {
            hwnd,
            default_hicon,
            custom_hicon,
        })
    }

    /// Re-adds the tray icon (e.g. when Explorer restarts and sends `TaskbarCreated`).
    pub fn re_add(&mut self, state: &DaemonState) {
        let active_hicon = if state.config.battery_tray_badge {
            if let Ok(new_hicon) = crate::tray_icon::create_battery_tray_icon(state) {
                if let Some(old) = self.custom_hicon.replace(new_hicon) {
                    crate::tray_icon::destroy_hicon(old);
                }
                new_hicon
            } else {
                self.custom_hicon.unwrap_or(self.default_hicon)
            }
        } else {
            if let Some(old) = self.custom_hicon.take() {
                crate::tray_icon::destroy_hicon(old);
            }
            self.default_hicon
        };

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: active_hicon,
            ..Default::default()
        };

        let tooltip = state.format_tooltip();
        copy_to_sz_tip(&mut nid.szTip, &tooltip);

        // SAFETY: `nid` is initialized with valid window handle and icon.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }
    }

    /// Updates the tray icon and tooltip from the latest daemon state.
    pub fn update(&mut self, state: &DaemonState) {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_TIP,
            ..Default::default()
        };

        let tooltip = state.format_tooltip();
        copy_to_sz_tip(&mut nid.szTip, &tooltip);

        // Dynamically update tray icon if badge is enabled
        if state.config.battery_tray_badge {
            if let Ok(new_hicon) = crate::tray_icon::create_battery_tray_icon(state) {
                if let Some(old) = self.custom_hicon.replace(new_hicon) {
                    crate::tray_icon::destroy_hicon(old);
                }
                nid.uFlags |= NIF_ICON;
                nid.hIcon = new_hicon;
            }
        } else if let Some(old) = self.custom_hicon.take() {
            crate::tray_icon::destroy_hicon(old);
            nid.uFlags |= NIF_ICON;
            nid.hIcon = self.default_hicon;
        }

        // SAFETY: `nid` is initialized with valid window handle and ID.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    /// Backwards-compatible alias for update.
    pub fn update_tooltip(&mut self, state: &DaemonState) {
        self.update(state);
    }

    /// Displays a balloon or toast notification from the system tray icon (F6.2).
    pub fn show_notification(&self, title: &str, message: &str) {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_INFO,
            dwInfoFlags: NIIF_WARNING,
            ..Default::default()
        };

        copy_to_sz_buf(&mut nid.szInfoTitle, title);
        copy_to_sz_buf(&mut nid.szInfo, message);

        // SAFETY: `nid` is initialized with valid window handle, NIF_INFO, title, and body.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    /// Displays the tray context menu at the current cursor position.
    pub fn show_context_menu(&self, state: &DaemonState) {
        let mut pt = POINT::default();
        // SAFETY: `GetCursorPos` has no preconditions and fills `pt`.
        if unsafe { GetCursorPos(&mut pt) }.is_err() {
            return;
        }

        // SAFETY: `CreatePopupMenu` creates a new empty popup menu.
        let Ok(hmenu) = (unsafe { CreatePopupMenu() }) else {
            return;
        };

        // 1. Enabled / Paused (toggle)
        let enabled_flags = if state.enabled {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        // SAFETY: `hmenu` is a valid menu handle.
        unsafe {
            let _ = AppendMenuW(hmenu, enabled_flags, IDM_TOGGLE_ENABLED, w!("Enabled"));
        }

        // 2. Profile submenu
        // SAFETY: `CreatePopupMenu` creates a new empty popup submenu.
        let profile_menu_res = unsafe { CreatePopupMenu() };
        if let Ok(h_profile_menu) = profile_menu_res {
            let profiles = state.profile_names();
            let active_profile = state.active_profile_name();

            for (i, name) in profiles.iter().enumerate() {
                let flags = if name == active_profile {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING | MF_UNCHECKED
                };
                let hstr = HSTRING::from(name.as_str());
                // SAFETY: `h_profile_menu` is valid; `hstr` provides a valid null-terminated PCWSTR.
                unsafe {
                    let _ = AppendMenuW(
                        h_profile_menu,
                        flags,
                        IDM_PROFILE_BASE + i,
                        PCWSTR(hstr.as_ptr()),
                    );
                }
            }

            // SAFETY: Attach profile submenu to root popup menu.
            unsafe {
                let _ = AppendMenuW(hmenu, MF_POPUP, h_profile_menu.0 as usize, w!("Profile"));
            }
        }

        // 3. Top Row Mode submenu
        // SAFETY: `CreatePopupMenu` creates a new empty popup submenu.
        let toprow_menu_res = unsafe { CreatePopupMenu() };
        if let Ok(h_toprow_menu) = toprow_menu_res {
            let mode = state.active_top_row_mode();
            let media_flags = if mode == TopRowMode::MediaKeysFirst {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING | MF_UNCHECKED
            };
            let fkeys_flags = if mode == TopRowMode::FKeysFirst {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING | MF_UNCHECKED
            };

            // SAFETY: `h_toprow_menu` is valid.
            unsafe {
                let _ = AppendMenuW(
                    h_toprow_menu,
                    media_flags,
                    IDM_TOPROW_MEDIA_FIRST,
                    w!("Media keys first"),
                );
                let _ = AppendMenuW(
                    h_toprow_menu,
                    fkeys_flags,
                    IDM_TOPROW_FKEYS_FIRST,
                    w!("F-keys first"),
                );
                let _ = AppendMenuW(
                    hmenu,
                    MF_POPUP,
                    h_toprow_menu.0 as usize,
                    w!("Top Row Mode"),
                );
            }
        }

        // 4. Open Settings...
        // SAFETY: `hmenu` is a valid menu handle.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN_SETTINGS, w!("Open Settings..."));
        }

        // 5. Start with Windows (checkable)
        let autostart_flags = if state.config.autostart {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING | MF_UNCHECKED
        };
        // SAFETY: `hmenu` is a valid menu handle.
        unsafe {
            let _ = AppendMenuW(
                hmenu,
                autostart_flags,
                IDM_START_WITH_WINDOWS,
                w!("Start with Windows"),
            );
        }

        // 6. Separator
        // SAFETY: `hmenu` is a valid menu handle.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        }

        // 7. Quit
        // SAFETY: `hmenu` is a valid menu handle.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_QUIT, w!("Quit"));
        }

        // Microsoft KB Q135788:
        // SetForegroundWindow must be called prior to TrackPopupMenuEx so clicking outside
        // dismisses the menu, followed by a benign WM_NULL message.
        // SAFETY: `self.hwnd` is a valid window handle.
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
            let _ = TrackPopupMenuEx(
                hmenu,
                (TPM_RIGHTBUTTON | TPM_BOTTOMALIGN).0,
                pt.x,
                pt.y,
                self.hwnd,
                None,
            );
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(hmenu);
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        // SAFETY: Cleans up the tray icon upon agent exit.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
        if let Some(custom) = self.custom_hicon.take() {
            crate::tray_icon::destroy_hicon(custom);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sz_tip_copying_and_truncation() {
        let mut buf = [0u16; 128];
        copy_to_sz_tip(&mut buf, "MagicBridge Test Tooltip");

        let s = String::from_utf16_lossy(&buf);
        let trimmed = s.trim_end_matches('\0');
        assert_eq!(trimmed, "MagicBridge Test Tooltip");

        // Test buffer truncation at 127 chars + null terminator
        let long_str = "a".repeat(200);
        copy_to_sz_tip(&mut buf, &long_str);
        assert_eq!(buf[127], 0);
        let s2 = String::from_utf16_lossy(&buf);
        let trimmed2 = s2.trim_end_matches('\0');
        assert_eq!(trimmed2.len(), 127);
    }
}
