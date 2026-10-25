//! Thin, safe-ish wrappers over SetupAPI and HID.dll (`HidD_*` / `HidP_*`).
//!
//! All `unsafe` for HID access in the probe lives here.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HIDD_ATTRIBUTES, HIDP_BUTTON_CAPS, HIDP_CAPS, HIDP_REPORT_TYPE, HIDP_STATUS_SUCCESS,
    HIDP_VALUE_CAPS, HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetFeature, HidD_GetHidGuid,
    HidD_GetInputReport, HidD_GetManufacturerString, HidD_GetPreparsedData, HidD_GetProductString,
    HidP_GetButtonCaps, HidP_GetCaps, HidP_GetValueCaps, PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    ReadFile,
};
use windows::core::{HSTRING, PCWSTR};

/// Returns the interface paths of all present HID collections.
pub fn enumerate_paths() -> Vec<String> {
    let mut paths = Vec::new();
    // SAFETY: HidD_GetHidGuid has no preconditions.
    let guid = unsafe { HidD_GetHidGuid() };
    // SAFETY: valid GUID pointer, no enumerator / parent window.
    let Ok(set) = (unsafe {
        SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }) else {
        return paths;
    };

    for index in 0.. {
        let mut iface = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: `set` is a live device-info set and `iface.cbSize` is initialised.
        if unsafe { SetupDiEnumDeviceInterfaces(set, None, &guid, index, &mut iface) }.is_err() {
            break;
        }
        let mut required = 0u32;
        // SAFETY: size query: null buffer with size 0 is the documented way to get `required`.
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(set, &iface, None, 0, Some(&mut required), None)
        };
        if required == 0 {
            continue;
        }
        // u64 backing keeps the struct suitably aligned.
        let mut buf = vec![0u64; (required as usize).div_ceil(8)];
        let detail = buf.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: `buf` is at least `required` bytes and 8-byte aligned; cbSize must be the
        // size of the fixed header (8 on x64), as documented.
        unsafe { (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 };
        // SAFETY: `detail` points to a buffer of `required` bytes.
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(set, &iface, Some(detail), required, None, None)
        }
        .is_ok()
        {
            // SAFETY: on success DevicePath is a NUL-terminated UTF-16 string inside `buf`.
            let path = unsafe { PCWSTR((*detail).DevicePath.as_ptr()).to_string() };
            if let Ok(p) = path {
                paths.push(p);
            }
        }
    }
    // SAFETY: `set` was returned by SetupDiGetClassDevsW and is destroyed exactly once.
    let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
    paths
}

/// An open handle to a HID collection. Closed on drop.
pub struct HidDevice {
    handle: HANDLE,
}

// SAFETY: a Win32 file handle may be used from any thread.
unsafe impl Send for HidDevice {}

impl Drop for HidDevice {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by this struct and closed exactly once.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

fn open_with(path: &str, access: u32) -> windows::core::Result<HANDLE> {
    // SAFETY: `HSTRING` provides a valid NUL-terminated path for the duration of the call.
    unsafe {
        CreateFileW(
            &HSTRING::from(path),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
}

impl HidDevice {
    /// Opens with zero access: enough for attributes, caps, strings and usually feature
    /// reports. Works even for keyboards/mice that Windows opens exclusively.
    pub fn open_query(path: &str) -> windows::core::Result<Self> {
        open_with(path, 0).map(|handle| Self { handle })
    }

    /// Opens for reading (and writing if allowed). Fails for system-owned keyboards/mice.
    pub fn open_read(path: &str) -> windows::core::Result<Self> {
        open_with(path, GENERIC_READ.0 | GENERIC_WRITE.0)
            .or_else(|_| open_with(path, GENERIC_READ.0))
            .map(|handle| Self { handle })
    }

    pub fn attributes(&self) -> Option<HIDD_ATTRIBUTES> {
        let mut a = HIDD_ATTRIBUTES {
            Size: size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        // SAFETY: valid handle and correctly sized out-struct.
        unsafe { HidD_GetAttributes(self.handle, &mut a) }.then_some(a)
    }

    pub fn product(&self) -> Option<String> {
        self.string(|h, b, l| {
            // SAFETY: forwarded valid handle/buffer/length.
            unsafe { HidD_GetProductString(h, b, l) }
        })
    }

    pub fn manufacturer(&self) -> Option<String> {
        self.string(|h, b, l| {
            // SAFETY: forwarded valid handle/buffer/length.
            unsafe { HidD_GetManufacturerString(h, b, l) }
        })
    }

    fn string(&self, f: impl Fn(HANDLE, *mut c_void, u32) -> bool) -> Option<String> {
        let mut buf = [0u16; 256];
        if !f(self.handle, buf.as_mut_ptr().cast(), (buf.len() * 2) as u32) {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let s = String::from_utf16_lossy(&buf[..len]);
        (!s.is_empty()).then_some(s)
    }

    pub fn preparsed(&self) -> Option<Preparsed> {
        let mut p = PHIDP_PREPARSED_DATA::default();
        // SAFETY: valid handle; on success `p` must be freed with HidD_FreePreparsedData.
        unsafe { HidD_GetPreparsedData(self.handle, &mut p) }.then_some(Preparsed(p))
    }

    /// `HidD_GetFeature`. `buf[0]` must hold the report ID. Returns the Win32 error on failure.
    pub fn get_feature(&self, buf: &mut [u8]) -> Result<(), u32> {
        // SAFETY: buffer pointer and length describe a valid writable region.
        let ok = unsafe { HidD_GetFeature(self.handle, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if ok { Ok(()) } else { Err(last_error()) }
    }

    /// `HidD_GetInputReport`. `buf[0]` must hold the report ID.
    pub fn get_input_report(&self, buf: &mut [u8]) -> Result<(), u32> {
        // SAFETY: buffer pointer and length describe a valid writable region.
        let ok =
            unsafe { HidD_GetInputReport(self.handle, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if ok { Ok(()) } else { Err(last_error()) }
    }

    /// Blocking read of the next input report. Returns the number of bytes read.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, u32> {
        let mut n = 0u32;
        // SAFETY: synchronous handle (no OVERLAPPED), valid buffer and out-count.
        match unsafe { ReadFile(self.handle, Some(buf), Some(&mut n), None) } {
            Ok(()) => Ok(n as usize),
            Err(_) => Err(last_error()),
        }
    }
}

fn last_error() -> u32 {
    // SAFETY: GetLastError has no preconditions.
    unsafe { GetLastError() }.0
}

/// Preparsed report-descriptor data. Freed on drop.
pub struct Preparsed(PHIDP_PREPARSED_DATA);

impl Drop for Preparsed {
    fn drop(&mut self) {
        // SAFETY: obtained from HidD_GetPreparsedData, freed exactly once.
        let _ = unsafe { HidD_FreePreparsedData(self.0) };
    }
}

/// One field of a report, flattened from button caps or value caps.
#[derive(Debug, Clone)]
pub struct Field {
    pub report_id: u8,
    pub usage_page: u16,
    pub usage_min: u16,
    pub usage_max: u16,
    pub link_usage_page: u16,
    pub link_usage: u16,
    /// `None` for buttons (1 bit each), `Some(bits)` for values.
    pub bit_size: Option<u16>,
    pub report_count: u16,
    pub logical_min: i32,
    pub logical_max: i32,
}

impl Preparsed {
    pub fn caps(&self) -> Option<HIDP_CAPS> {
        let mut caps = HIDP_CAPS::default();
        // SAFETY: valid preparsed data and out-struct.
        (unsafe { HidP_GetCaps(self.0, &mut caps) } == HIDP_STATUS_SUCCESS).then_some(caps)
    }

    /// All button and value fields for one report type.
    pub fn fields(&self, kind: HIDP_REPORT_TYPE, n_buttons: u16, n_values: u16) -> Vec<Field> {
        let mut out = Vec::new();

        if n_buttons > 0 {
            let mut caps = vec![HIDP_BUTTON_CAPS::default(); n_buttons as usize];
            let mut len = n_buttons;
            // SAFETY: `caps` has room for `len` entries; `len` is updated to the count written.
            if unsafe { HidP_GetButtonCaps(kind, caps.as_mut_ptr(), &mut len, self.0) }
                == HIDP_STATUS_SUCCESS
            {
                for c in &caps[..len as usize] {
                    // SAFETY: `IsRange` selects which union member Windows filled in.
                    let (lo, hi) = unsafe {
                        if c.IsRange {
                            (c.Anonymous.Range.UsageMin, c.Anonymous.Range.UsageMax)
                        } else {
                            (c.Anonymous.NotRange.Usage, c.Anonymous.NotRange.Usage)
                        }
                    };
                    out.push(Field {
                        report_id: c.ReportID,
                        usage_page: c.UsagePage,
                        usage_min: lo,
                        usage_max: hi,
                        link_usage_page: c.LinkUsagePage,
                        link_usage: c.LinkUsage,
                        bit_size: None,
                        report_count: c.ReportCount,
                        logical_min: 0,
                        logical_max: 1,
                    });
                }
            }
        }

        if n_values > 0 {
            let mut caps = vec![HIDP_VALUE_CAPS::default(); n_values as usize];
            let mut len = n_values;
            // SAFETY: `caps` has room for `len` entries; `len` is updated to the count written.
            if unsafe { HidP_GetValueCaps(kind, caps.as_mut_ptr(), &mut len, self.0) }
                == HIDP_STATUS_SUCCESS
            {
                for c in &caps[..len as usize] {
                    // SAFETY: `IsRange` selects which union member Windows filled in.
                    let (lo, hi) = unsafe {
                        if c.IsRange {
                            (c.Anonymous.Range.UsageMin, c.Anonymous.Range.UsageMax)
                        } else {
                            (c.Anonymous.NotRange.Usage, c.Anonymous.NotRange.Usage)
                        }
                    };
                    out.push(Field {
                        report_id: c.ReportID,
                        usage_page: c.UsagePage,
                        usage_min: lo,
                        usage_max: hi,
                        link_usage_page: c.LinkUsagePage,
                        link_usage: c.LinkUsage,
                        bit_size: Some(c.BitSize),
                        report_count: c.ReportCount,
                        logical_min: c.LogicalMin,
                        logical_max: c.LogicalMax,
                    });
                }
            }
        }

        out.sort_by_key(|f| (f.report_id, f.usage_page, f.usage_min));
        out
    }
}

/// Human-readable name for common HID usage pages / usages (diagnostics only).
pub fn usage_name(page: u16, usage: u16) -> String {
    let page_name = match page {
        0x01 => "Generic Desktop",
        0x06 => "Generic Device Controls",
        0x07 => "Keyboard/Keypad",
        0x08 => "LED",
        0x09 => "Button",
        0x0C => "Consumer",
        0x0D => "Digitizer",
        0x84 => "Power Device",
        0x85 => "Battery System",
        0xFF00..=0xFFFF => "Vendor",
        _ => "?",
    };
    let usage_name = match (page, usage) {
        (0x01, 0x02) => "Mouse",
        (0x01, 0x06) => "Keyboard",
        (0x01, 0x07) => "Keypad",
        (0x01, 0x80) => "System Control",
        (0x06, 0x20) => "Battery Strength",
        (0x0C, 0x01) => "Consumer Control",
        (0x0C, 0xB5) => "Next Track",
        (0x0C, 0xB6) => "Previous Track",
        (0x0C, 0xB8) => "Eject",
        (0x0C, 0xCD) => "Play/Pause",
        (0x0C, 0xE2) => "Mute",
        (0x0C, 0xE9) => "Volume Up",
        (0x0C, 0xEA) => "Volume Down",
        (0x0C, 0x6F) => "Brightness Up",
        (0x0C, 0x70) => "Brightness Down",
        (0x0C, 0xB3) => "Fast Forward",
        (0x0C, 0xB4) => "Rewind",
        (0x85, 0x44) => "Charging",
        (0x85, 0x46) => "Discharging",
        (0x85, 0x61) => "AC Present",
        (0x85, 0x65) => "AbsoluteStateOfCharge (%)",
        (0xFF, 0x03) | (0xFF00, 0x03) => "Apple Fn (0x00FF:0x0003)",
        _ => "",
    };
    if usage_name.is_empty() {
        format!("0x{page:04X}:0x{usage:04X} ({page_name})")
    } else {
        format!("0x{page:04X}:0x{usage:04X} ({page_name} / {usage_name})")
    }
}
