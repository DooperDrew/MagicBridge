//! Direct HID battery reading and status decoding for Apple Magic Keyboard (F6.1..F6.3).
//!
//! On both USB (`MI_00&Col02`) and Bluetooth (`Col02`), Apple Magic Keyboard (model A1843)
//! exposes top-level collection `0xFF00:0x0014` with Input Report `0x90` (3 bytes):
//! - `buf[0] = 0x90` (Report ID)
//! - `buf[1] = status_flags`:
//!   - bit 0 (`0x01`): Charging (`0x85:0x44`)
//!   - bit 1 (`0x02`): Discharging (`0x85:0x46`)
//!   - bit 2 (`0x04`): AC Present (`0x85:0x61`)
//! - `buf[2] = percentage`: `0..=100` (`0x85:0x65` `AbsoluteStateOfCharge`).
//!
//! If HID report query is unsupported by a third-party Bluetooth stack, falls back
//! to the standard Windows Bluetooth PnP battery property (`PKEY_BLUETOOTH_BATTERY`).

use std::collections::HashSet;
use std::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HIDP_CAPS, HIDP_STATUS_SUCCESS, HidD_FreePreparsedData, HidD_GetInputReport,
    HidD_GetPreparsedData, HidP_GetCaps, PHIDP_PREPARSED_DATA,
};
use windows::Win32::Devices::Properties::{DEVPROP_TYPE_BYTE, DEVPROP_TYPE_UINT32, DEVPROPTYPE};
use windows::Win32::Foundation::{CloseHandle, DEVPROPKEY, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::core::{GUID, HSTRING, PCWSTR};

/// Apple Vendor Usage Page (`0xFF00`).
pub const APPLE_VENDOR_USAGE_PAGE: u16 = 0xFF00;

/// Apple Battery System Top-Level Usage (`0x0014`).
pub const APPLE_BATTERY_USAGE: u16 = 0x0014;

/// Apple Battery Input Report ID (`0x90`).
pub const APPLE_BATTERY_REPORT_ID: u8 = 0x90;

/// Standard Windows Bluetooth battery level device property key:
/// `{104ea319-6ee2-4701-bd47-8ddb4425bbe5}`, pid 2 (`DEVPROP_TYPE_BYTE`, 0..=100).
pub const PKEY_BLUETOOTH_BATTERY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddb4425bbe5),
    pid: 2,
};

/// Decoded battery state from Apple HID Input Report 0x90 or PnP Bluetooth properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    /// Battery level percentage (`0..=100`).
    pub percentage: u8,
    /// Whether the battery is currently charging (bit 0 of status byte).
    pub is_charging: bool,
    /// Whether the battery is currently discharging (bit 1 of status byte).
    pub is_discharging: bool,
    /// Whether external AC power is connected (bit 2 of status byte).
    pub is_ac_present: bool,
    /// Raw status bitmask from report 0x90.
    pub raw_status: u8,
}

impl BatteryStatus {
    /// Creates a battery status from a percentage with neutral charging flags.
    #[must_use]
    pub const fn from_percentage(percentage: u8) -> Self {
        Self {
            percentage: if percentage > 100 { 100 } else { percentage },
            is_charging: false,
            is_discharging: false,
            is_ac_present: false,
            raw_status: 0,
        }
    }

    /// Decodes Apple HID Input Report `0x90` buffer.
    ///
    /// Layout:
    /// - `buf[0]`: Report ID, must be `0x90`.
    /// - `buf[1]`: Status bitmask (bit 0 = charging, bit 1 = discharging, bit 2 = AC present).
    /// - `buf[2]`: Battery percentage (`0..=100`).
    #[must_use]
    pub fn from_report_0x90(buf: &[u8]) -> Option<Self> {
        if buf.len() < 3 || buf[0] != APPLE_BATTERY_REPORT_ID {
            return None;
        }

        let raw_status = buf[1];
        let raw_pct = buf[2];
        let percentage = if raw_pct > 100 { 100 } else { raw_pct };

        Some(Self {
            percentage,
            is_charging: (raw_status & 0x01) != 0,
            is_discharging: (raw_status & 0x02) != 0,
            is_ac_present: (raw_status & 0x04) != 0,
            raw_status,
        })
    }
}

/// Helper function to open a HID collection handle with read/query access.
fn open_hid_collection(path: &str) -> windows::core::Result<HANDLE> {
    let path_hstr = HSTRING::from(path);

    // Try zero query access first (sufficient for HidD_GetInputReport and works with exclusive devices).
    // SAFETY: CreateFileW called with valid path, sharing flags, and OPEN_EXISTING.
    let query_handle = unsafe {
        CreateFileW(
            &path_hstr,
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    };

    if let Ok(handle) = query_handle {
        return Ok(handle);
    }

    // Fallback: try with GENERIC_READ access.
    // SAFETY: CreateFileW called with valid path, GENERIC_READ, sharing flags, and OPEN_EXISTING.
    unsafe {
        CreateFileW(
            &path_hstr,
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
}

/// Retrieves preparsed HID caps for an open handle.
fn get_collection_caps(handle: HANDLE) -> Option<HIDP_CAPS> {
    let mut preparsed = PHIDP_PREPARSED_DATA::default();

    // SAFETY: HidD_GetPreparsedData allocates preparsed data for the valid collection handle.
    let ok = unsafe { HidD_GetPreparsedData(handle, &mut preparsed) };
    if !ok || preparsed.0 == 0 {
        return None;
    }

    let mut caps = HIDP_CAPS::default();
    // SAFETY: HidP_GetCaps retrieves HIDP_CAPS from the valid preparsed pointer.
    let status = unsafe { HidP_GetCaps(preparsed, &mut caps) };

    // SAFETY: Preparsed data must be freed with HidD_FreePreparsedData.
    unsafe {
        let _ = HidD_FreePreparsedData(preparsed);
    }

    if status == HIDP_STATUS_SUCCESS {
        Some(caps)
    } else {
        None
    }
}

/// Reads Apple battery report `0x90` from a specific HID collection path.
pub fn read_battery_from_hid_path(path: &str) -> Option<BatteryStatus> {
    let handle = open_hid_collection(path).ok()?;

    // Inspect caps if available. If caps are successfully read and usage page/usage
    // do not match Apple battery collection (0xFF00:0x0014), skip unless it's a candidate collection.
    if let Some(caps) = get_collection_caps(handle)
        && caps.UsagePage != APPLE_VENDOR_USAGE_PAGE
        && caps.Usage != APPLE_BATTERY_USAGE
    {
        // Main keyboard collection (0x0001:0x0006) does not expose report 0x90; avoid unnecessary call.
        if caps.UsagePage == 0x0001 && caps.Usage == 0x0006 {
            // SAFETY: CloseHandle closes the opened device handle.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return None;
        }
    }

    // Input Report 0x90 buffer: [report_id, status, pct].
    let mut buf = [APPLE_BATTERY_REPORT_ID, 0, 0];

    // SAFETY:
    // `handle` is an open Win32 HANDLE to the HID collection.
    // `buf` is a writable 3-byte array where buf[0] is set to requested Report ID 0x90.
    let success = unsafe { HidD_GetInputReport(handle, buf.as_mut_ptr().cast(), buf.len() as u32) };

    // SAFETY: CloseHandle closes the handle after query.
    unsafe {
        let _ = CloseHandle(handle);
    }

    if success {
        BatteryStatus::from_report_0x90(&buf)
    } else {
        None
    }
}

/// Reads the Windows Bluetooth PnP battery level property (`PKEY_BLUETOOTH_BATTERY`)
/// for any present Apple keyboard device node.
pub fn read_bluetooth_pnp_battery() -> Option<u8> {
    // SAFETY: SetupDiGetClassDevsW enumerates all present devices on the system.
    let set = match unsafe {
        SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
    } {
        Ok(s) => s,
        Err(_) => return None,
    };

    let mut found_battery = None;

    for idx in 0.. {
        let mut dev = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        // SAFETY: SetupDiEnumDeviceInfo enumerates each device in the set.
        if unsafe { SetupDiEnumDeviceInfo(set, idx, &mut dev) }.is_err() {
            break;
        }

        // Query device instance ID to verify Apple vendor ID.
        let mut id_buf = [0u16; 512];
        let mut req_len = 0u32;
        // SAFETY: SetupDiGetDeviceInstanceIdW retrieves the instance ID.
        if unsafe { SetupDiGetDeviceInstanceIdW(set, &dev, Some(&mut id_buf), Some(&mut req_len)) }
            .is_err()
        {
            continue;
        }

        let len = id_buf.iter().position(|&c| c == 0).unwrap_or(id_buf.len());
        let instance_id = String::from_utf16_lossy(&id_buf[..len]).to_ascii_uppercase();

        // Check if device matches Apple USB VID 0x05AC or Bluetooth SIG Company ID 0x004C.
        if !instance_id.contains("05AC") && !instance_id.contains("004C") {
            continue;
        }

        // Query PKEY_BLUETOOTH_BATTERY property.
        let mut ptype = DEVPROPTYPE::default();
        let mut prop_buf = [0u8; 8];
        let mut prop_req = 0u32;

        // SAFETY: SetupDiGetDevicePropertyW reads PKEY_BLUETOOTH_BATTERY into prop_buf.
        let ok = unsafe {
            SetupDiGetDevicePropertyW(
                set,
                &dev,
                &PKEY_BLUETOOTH_BATTERY,
                &mut ptype,
                Some(&mut prop_buf),
                Some(&mut prop_req),
                0,
            )
        };

        if ok.is_ok() {
            let pct = match ptype {
                DEVPROP_TYPE_BYTE => Some(prop_buf[0]),
                DEVPROP_TYPE_UINT32 if prop_req >= 4 => Some(prop_buf[0]),
                _ => None,
            };

            if let Some(p) = pct
                && p <= 100
            {
                found_battery = Some(p);
                break;
            }
        }
    }

    // SAFETY: SetupDiDestroyDeviceInfoList destroys the allocated HDEVINFO set.
    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(set);
    }

    found_battery
}

/// Queries the battery status across all currently connected Apple keyboards.
///
/// Priority:
/// 1. Direct HID Input Report 0x90 on `0xFF00:0x0014` Apple collections.
/// 2. Probed Apple HID collections if connected paths don't include the battery interface.
/// 3. Windows Bluetooth PnP battery property (`PKEY_BLUETOOTH_BATTERY`).
pub fn query_battery(connected_paths: &HashSet<String>) -> Option<BatteryStatus> {
    // 1. Try reading from each connected path tracked in state.
    for path in connected_paths {
        if let Some(status) = read_battery_from_hid_path(path) {
            return Some(status);
        }
    }

    // 2. If not found in tracked paths, probe all present Apple HID devices.
    for (path, _) in crate::device_notify::probe_connected_apple_keyboards() {
        if let Some(status) = read_battery_from_hid_path(&path) {
            return Some(status);
        }
    }

    // 3. Fallback to Bluetooth PnP property.
    if let Some(pct) = read_bluetooth_pnp_battery() {
        return Some(BatteryStatus::from_percentage(pct));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_status_from_report_0x90_usb_charging() {
        // Report 0x90 observed on USB in hardware findings §5:
        // [0x90, 0x01, 0x64] -> status=0x01 (charging=true, discharging=false, ac=false), 100%
        let buf = [0x90, 0x01, 100];
        let status = BatteryStatus::from_report_0x90(&buf).expect("should decode report 0x90");
        assert_eq!(status.percentage, 100);
        assert!(status.is_charging);
        assert!(!status.is_discharging);
        assert!(!status.is_ac_present);
        assert_eq!(status.raw_status, 0x01);
    }

    #[test]
    fn test_battery_status_from_report_0x90_bluetooth_discharging() {
        // Report 0x90 observed on Bluetooth in hardware findings §5:
        // [0x90, 0x00, 78] -> status=0x00, 78%
        let buf = [0x90, 0x00, 78];
        let status = BatteryStatus::from_report_0x90(&buf).expect("should decode report 0x90");
        assert_eq!(status.percentage, 78);
        assert!(!status.is_charging);
        assert!(!status.is_discharging);
        assert!(!status.is_ac_present);
        assert_eq!(status.raw_status, 0x00);
    }

    #[test]
    fn test_battery_status_flags_decoding() {
        // status = 0x07: charging (bit 0), discharging (bit 1), AC present (bit 2)
        let buf = [0x90, 0x07, 42];
        let status = BatteryStatus::from_report_0x90(&buf).expect("should decode");
        assert_eq!(status.percentage, 42);
        assert!(status.is_charging);
        assert!(status.is_discharging);
        assert!(status.is_ac_present);

        // status = 0x04: AC present only
        let buf2 = [0x90, 0x04, 99];
        let status2 = BatteryStatus::from_report_0x90(&buf2).expect("should decode");
        assert_eq!(status2.percentage, 99);
        assert!(!status2.is_charging);
        assert!(!status2.is_discharging);
        assert!(status2.is_ac_present);
    }

    #[test]
    fn test_battery_status_percentage_clamping() {
        // Values > 100 clamped to 100
        let buf = [0x90, 0x01, 150];
        let status = BatteryStatus::from_report_0x90(&buf).expect("should decode");
        assert_eq!(status.percentage, 100);

        let neutral = BatteryStatus::from_percentage(200);
        assert_eq!(neutral.percentage, 100);
    }

    #[test]
    fn test_battery_status_invalid_report() {
        // Wrong report ID
        assert!(BatteryStatus::from_report_0x90(&[0x80, 0x01, 50]).is_none());
        assert!(BatteryStatus::from_report_0x90(&[0x01, 0x00, 0x00]).is_none());

        // Too short
        assert!(BatteryStatus::from_report_0x90(&[]).is_none());
        assert!(BatteryStatus::from_report_0x90(&[0x90]).is_none());
        assert!(BatteryStatus::from_report_0x90(&[0x90, 0x01]).is_none());
    }

    #[test]
    fn test_read_battery_from_nonexistent_path_returns_none() {
        assert!(read_battery_from_hid_path(r"\\?\hid#nonexistent_device").is_none());
    }

    #[test]
    fn test_pnp_battery_does_not_panic() {
        // Probing PnP devices on the local machine should run safely and not panic.
        let _ = read_bluetooth_pnp_battery();
    }
}
