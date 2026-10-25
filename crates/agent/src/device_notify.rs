//! Device arrival/removal notifications and sleep/resume handling (F8.4, NFR-S2).
//!
//! Provides RAII registration for Windows device interface change notifications
//! via `RegisterDeviceNotificationW` and `UnregisterDeviceNotification`, parsing of
//! `WM_DEVICECHANGE` broadcast data, and probing of currently present Apple keyboards.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows::Win32::Devices::HumanInterfaceDevice::GUID_DEVINTERFACE_HID;
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DBT_DEVTYP_DEVICEINTERFACE, DEV_BROADCAST_DEVICEINTERFACE_W, DEV_BROADCAST_HDR,
    DEVICE_NOTIFY_WINDOW_HANDLE, HDEVNOTIFY, RegisterDeviceNotificationW,
    UnregisterDeviceNotification,
};
use windows::core::{GUID, PCWSTR};

use magicbridge_core::device::parse_hid_path;
use magicbridge_core::is_apple_vendor;

/// RAII wrapper for a Windows device notification registration handle (`HDEVNOTIFY`).
///
/// Automatically calls `UnregisterDeviceNotification` on drop.
#[derive(Debug)]
pub struct DeviceNotificationHandle {
    handle: HDEVNOTIFY,
}

// SAFETY: HDEVNOTIFY can be dropped from any thread.
unsafe impl Send for DeviceNotificationHandle {}

impl DeviceNotificationHandle {
    /// Registers the given window to receive device interface notifications for `class_guid`.
    pub fn register_for_device_interface(
        hwnd: HWND,
        class_guid: &GUID,
    ) -> windows::core::Result<Self> {
        let filter = DEV_BROADCAST_DEVICEINTERFACE_W {
            dbcc_size: size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>() as u32,
            dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE.0,
            dbcc_reserved: 0,
            dbcc_classguid: *class_guid,
            dbcc_name: [0],
        };

        // SAFETY:
        // - `hwnd` is a valid window handle passed as a HANDLE recipient.
        // - `filter` points to a properly initialized DEV_BROADCAST_DEVICEINTERFACE_W structure
        //   with dbcc_size set to sizeof(DEV_BROADCAST_DEVICEINTERFACE_W) and dbcc_devicetype
        //   set to DBT_DEVTYP_DEVICEINTERFACE.
        // - DEVICE_NOTIFY_WINDOW_HANDLE specifies that recipient is a window handle.
        let handle = unsafe {
            RegisterDeviceNotificationW(
                HANDLE(hwnd.0),
                std::ptr::addr_of!(filter).cast::<c_void>(),
                DEVICE_NOTIFY_WINDOW_HANDLE,
            )?
        };

        Ok(Self { handle })
    }

    /// Wraps an existing raw handle (useful for testing or custom registration).
    #[must_use]
    pub fn from_raw(handle: HDEVNOTIFY) -> Self {
        Self { handle }
    }

    /// Returns `true` if the underlying notification handle is invalid.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        self.handle.is_invalid()
    }
}

impl Drop for DeviceNotificationHandle {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY:
            // `self.handle` is a non-null, valid HDEVNOTIFY handle previously returned
            // by RegisterDeviceNotificationW, and is unregistered exactly once on drop.
            unsafe {
                let _ = UnregisterDeviceNotification(self.handle);
            }
        }
    }
}

/// Parses a device interface path from `LPARAM` in a `WM_DEVICECHANGE` message.
///
/// Returns `Some(device_path)` if `lparam` points to a valid `DEV_BROADCAST_DEVICEINTERFACE_W`
/// containing a non-empty UTF-16 device path.
/// Returns `None` if `lparam` is null, too small, or has a different device type.
pub fn parse_device_interface_path(lparam: LPARAM) -> Option<String> {
    if lparam.0 == 0 {
        return None;
    }

    let min_hdr_size = size_of::<DEV_BROADCAST_HDR>();
    let min_iface_size = size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>();

    // SAFETY:
    // `lparam.0` is non-null. In WM_DEVICECHANGE, Windows passes a pointer to a DEV_BROADCAST_*
    // structure where the first field is dbch_size (u32). We verify that dbch_size is at least
    // min_iface_size and dbch_devicetype matches DBT_DEVTYP_DEVICEINTERFACE before reading
    // interface fields or the variable-length dbcc_name array.
    unsafe {
        let hdr = lparam.0 as *const DEV_BROADCAST_HDR;
        let total_size = (*hdr).dbch_size as usize;
        if total_size < min_hdr_size || (*hdr).dbch_devicetype != DBT_DEVTYP_DEVICEINTERFACE {
            return None;
        }

        if total_size < min_iface_size {
            return None;
        }

        let iface = lparam.0 as *const DEV_BROADCAST_DEVICEINTERFACE_W;
        let name_offset = core::mem::offset_of!(DEV_BROADCAST_DEVICEINTERFACE_W, dbcc_name);
        if total_size <= name_offset {
            return None;
        }

        let max_chars = (total_size - name_offset) / size_of::<u16>();
        let name_slice = std::slice::from_raw_parts((*iface).dbcc_name.as_ptr(), max_chars);

        // Find the null-terminator.
        let len = name_slice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(name_slice.len());

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&name_slice[..len]))
    }
}

/// Returns `true` if the device interface path corresponds to an Apple keyboard
/// (VID `0x05AC` on USB or `0x004C` over Bluetooth Classic).
#[must_use]
pub fn is_apple_keyboard_path(path: &str) -> bool {
    let info = parse_hid_path(path);
    info.vid.map(is_apple_vendor).unwrap_or(false)
}

/// Probes currently present HID devices to detect any connected Apple keyboards.
///
/// Returns a vector of `(device_path, transport_string)` for each present Apple keyboard interface.
pub fn probe_connected_apple_keyboards() -> Vec<(String, String)> {
    let mut results = Vec::new();
    let guid = GUID_DEVINTERFACE_HID;

    // SAFETY:
    // SetupDiGetClassDevsW is called with valid GUID pointer and DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
    // to enumerate currently connected HID devices.
    let dev_info = match unsafe {
        SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    } {
        Ok(set) => set,
        Err(_) => return results,
    };

    for index in 0.. {
        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        // SAFETY:
        // `dev_info` is a valid HDEVINFO handle, and `iface_data.cbSize` is initialized.
        let enum_ok =
            unsafe { SetupDiEnumDeviceInterfaces(dev_info, None, &guid, index, &mut iface_data) };
        if enum_ok.is_err() {
            break;
        }

        let mut required_size = 0u32;
        // SAFETY:
        // First call with null buffer queries the required buffer size in `required_size`.
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &iface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            )
        };
        if required_size == 0 {
            continue;
        }

        // Buffer must be properly aligned for SP_DEVICE_INTERFACE_DETAIL_DATA_W.
        let mut buf = vec![0u64; (required_size as usize).div_ceil(8)];
        let detail_ptr = buf.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();

        // SAFETY:
        // As documented by Win32 SDK, cbSize must be set to the size of the fixed
        // structure (8 bytes on x64).
        unsafe {
            (*detail_ptr).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }

        // SAFETY:
        // `detail_ptr` points to an aligned buffer with at least `required_size` bytes.
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &iface_data,
                Some(detail_ptr),
                required_size,
                None,
                None,
            )
        }
        .is_ok()
        {
            // SAFETY:
            // On success, DevicePath is a null-terminated UTF-16 string inside `buf`.
            let path_str = unsafe { PCWSTR((*detail_ptr).DevicePath.as_ptr()).to_string() };
            if let Ok(path) = path_str {
                let info = parse_hid_path(&path);
                if info.vid.map(is_apple_vendor).unwrap_or(false) {
                    results.push((path, info.transport.to_string()));
                }
            }
        }
    }

    // SAFETY:
    // `dev_info` was allocated by SetupDiGetClassDevsW and must be destroyed.
    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(dev_info);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_null_lparam_returns_none() {
        assert_eq!(parse_device_interface_path(LPARAM(0)), None);
    }

    #[test]
    fn parse_too_small_buffer_returns_none() {
        let mut buf = [0u8; 16];
        // Set dbch_size to 16 (< 28)
        buf[0..4].copy_from_slice(&16u32.to_ne_bytes());
        buf[4..8].copy_from_slice(&DBT_DEVTYP_DEVICEINTERFACE.0.to_ne_bytes());
        let lparam = LPARAM(buf.as_ptr() as isize);
        assert_eq!(parse_device_interface_path(lparam), None);
    }

    #[test]
    fn parse_wrong_devtype_returns_none() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&64u32.to_ne_bytes());
        buf[4..8].copy_from_slice(&0u32.to_ne_bytes()); // OEM devicetype
        let lparam = LPARAM(buf.as_ptr() as isize);
        assert_eq!(parse_device_interface_path(lparam), None);
    }

    #[test]
    fn parse_valid_usb_device_interface_path() {
        let path = r"\\?\hid#vid_05ac&pid_026c&mi_01&col01#8&2fa5df45&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let name_offset = core::mem::offset_of!(DEV_BROADCAST_DEVICEINTERFACE_W, dbcc_name);
        let total_size = name_offset + wide.len() * size_of::<u16>();

        let mut buf = vec![0u64; total_size.div_ceil(8)];
        let byte_ptr = buf.as_mut_ptr().cast::<u8>();

        // SAFETY: `buf` is allocated with sufficient 8-byte aligned memory to hold
        // the DEV_BROADCAST_DEVICEINTERFACE_W header and wide string `wide`.
        unsafe {
            // dbcc_size
            (byte_ptr as *mut u32).write(total_size as u32);
            // dbcc_devicetype
            (byte_ptr.add(4) as *mut u32).write(DBT_DEVTYP_DEVICEINTERFACE.0);
            // dbcc_reserved
            (byte_ptr.add(8) as *mut u32).write(0);
            // dbcc_classguid
            (byte_ptr.add(12) as *mut GUID).write(GUID_DEVINTERFACE_HID);
            // dbcc_name
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                byte_ptr.add(name_offset) as *mut u16,
                wide.len(),
            );
        }

        let lparam = LPARAM(buf.as_ptr() as isize);
        let extracted = parse_device_interface_path(lparam).expect("should extract path");
        assert_eq!(extracted, path);
        assert!(is_apple_keyboard_path(&extracted));
    }

    #[test]
    fn parse_valid_bluetooth_device_interface_path() {
        let path = r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&0001004c_PID&026c&Col01#8&eb3c0b3&0&0000#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}";
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let name_offset = core::mem::offset_of!(DEV_BROADCAST_DEVICEINTERFACE_W, dbcc_name);
        let total_size = name_offset + wide.len() * size_of::<u16>();

        let mut buf = vec![0u64; total_size.div_ceil(8)];
        let byte_ptr = buf.as_mut_ptr().cast::<u8>();

        // SAFETY: `buf` is allocated with sufficient 8-byte aligned memory to hold
        // the DEV_BROADCAST_DEVICEINTERFACE_W header and wide string `wide`.
        unsafe {
            (byte_ptr as *mut u32).write(total_size as u32);
            (byte_ptr.add(4) as *mut u32).write(DBT_DEVTYP_DEVICEINTERFACE.0);
            (byte_ptr.add(8) as *mut u32).write(0);
            (byte_ptr.add(12) as *mut GUID).write(GUID_DEVINTERFACE_HID);
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                byte_ptr.add(name_offset) as *mut u16,
                wide.len(),
            );
        }

        let lparam = LPARAM(buf.as_ptr() as isize);
        let extracted = parse_device_interface_path(lparam).expect("should extract path");
        assert_eq!(extracted, path);
        assert!(is_apple_keyboard_path(&extracted));
    }

    #[test]
    fn non_apple_path_is_rejected_by_is_apple_keyboard_path() {
        let non_apple =
            r"\\?\hid#vid_046d&pid_c52b&mi_00#7&abc&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert!(!is_apple_keyboard_path(non_apple));
    }

    #[test]
    fn invalid_notification_handle_does_not_panic_on_drop() {
        let handle = DeviceNotificationHandle::from_raw(HDEVNOTIFY(core::ptr::null_mut()));
        assert!(handle.is_invalid());
        drop(handle);
    }
}
