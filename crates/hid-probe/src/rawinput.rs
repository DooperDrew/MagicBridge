//! Raw Input wrappers: device list, device info, registration, WM_INPUT parsing.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, HRAWINPUT, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RID_DEVICE_INFO, RID_INPUT,
    RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICEINFO, RIDI_DEVICENAME, RIM_TYPEHID,
    RIM_TYPEKEYBOARD, RIM_TYPEMOUSE, RegisterRawInputDevices,
};

/// A Raw Input device as reported by `GetRawInputDeviceList`.
#[derive(Debug, Clone)]
pub struct RawDevice {
    pub handle: HANDLE,
    pub kind: &'static str,
    pub name: String,
    /// Usage page / usage for HID devices; (1,6) keyboard, (1,2) mouse.
    pub usage: (u16, u16),
    pub vid_pid: Option<(u16, u16)>,
}

// SAFETY: Raw Input device handles are plain identifiers, valid from any thread.
unsafe impl Send for RawDevice {}

pub fn list_devices() -> Vec<RawDevice> {
    let mut count = 0u32;
    let entry = size_of::<RAWINPUTDEVICELIST>() as u32;
    // SAFETY: size query with a null buffer.
    unsafe { GetRawInputDeviceList(None, &mut count, entry) };
    let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
    // SAFETY: `list` has room for `count` entries.
    let n = unsafe { GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, entry) };
    if n == u32::MAX {
        return Vec::new();
    }
    list.truncate(n as usize);
    list.iter().map(|d| describe(d.hDevice)).collect()
}

/// Full description of a Raw Input device handle.
pub fn describe(handle: HANDLE) -> RawDevice {
    let name = device_name(handle).unwrap_or_default();
    let mut info = RID_DEVICE_INFO {
        cbSize: size_of::<RID_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    let mut size = size_of::<RID_DEVICE_INFO>() as u32;
    // SAFETY: `info` is a correctly sized, cbSize-initialised RID_DEVICE_INFO.
    let r = unsafe {
        GetRawInputDeviceInfoW(
            Some(handle),
            RIDI_DEVICEINFO,
            Some((&raw mut info).cast::<c_void>()),
            &mut size,
        )
    };
    let (kind, usage, vid_pid) = if r == u32::MAX {
        ("?", (0, 0), None)
    } else {
        match info.dwType {
            // SAFETY: dwType tells which union member is valid.
            RIM_TYPEHID => unsafe {
                let h = info.Anonymous.hid;
                (
                    "hid",
                    (h.usUsagePage, h.usUsage),
                    Some((h.dwVendorId as u16, h.dwProductId as u16)),
                )
            },
            RIM_TYPEKEYBOARD => ("keyboard", (1, 6), None),
            RIM_TYPEMOUSE => ("mouse", (1, 2), None),
            _ => ("?", (0, 0), None),
        }
    };
    let vid_pid = vid_pid.or_else(|| {
        let p = magicbridge_core::device::parse_hid_path(&name);
        p.vid.zip(p.pid)
    });
    RawDevice {
        handle,
        kind,
        name,
        usage,
        vid_pid,
    }
}

fn device_name(handle: HANDLE) -> Option<String> {
    let mut chars = 0u32;
    // SAFETY: size query (in characters) with a null buffer.
    unsafe { GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut chars) };
    if chars == 0 {
        return None;
    }
    let mut buf = vec![0u16; chars as usize];
    // SAFETY: `buf` holds `chars` UTF-16 units.
    let r = unsafe {
        GetRawInputDeviceInfoW(
            Some(handle),
            RIDI_DEVICENAME,
            Some(buf.as_mut_ptr().cast()),
            &mut chars,
        )
    };
    if r == u32::MAX {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Registers for background (`INPUTSINK`) Raw Input plus arrival/removal notifications
/// on every given usage page / usage pair.
pub fn register(hwnd: HWND, usages: &[(u16, u16)]) -> windows::core::Result<()> {
    let devices: Vec<RAWINPUTDEVICE> = usages
        .iter()
        .map(|&(page, usage)| RAWINPUTDEVICE {
            usUsagePage: page,
            usUsage: usage,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        })
        .collect();
    // SAFETY: slice of valid RAWINPUTDEVICE entries targeting a window we own.
    unsafe { RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) }
}

/// A decoded WM_INPUT event.
pub enum RawEvent {
    Keyboard {
        device: HANDLE,
        make_code: u16,
        flags: u16,
        vkey: u16,
        message: u32,
        extra: u32,
    },
    Hid {
        device: HANDLE,
        report_size: usize,
        reports: Vec<Vec<u8>>,
    },
    Other,
}

/// Reads the RAWINPUT referenced by a WM_INPUT `lParam`.
pub fn read_event(lparam: LPARAM) -> Option<RawEvent> {
    let header = size_of::<RAWINPUTHEADER>() as u32;
    let hraw = HRAWINPUT(lparam.0 as *mut c_void);
    let mut size = 0u32;
    // SAFETY: size query with a null buffer for the handle passed in WM_INPUT.
    unsafe { GetRawInputData(hraw, RID_INPUT, None, &mut size, header) };
    if size == 0 {
        return None;
    }
    // u64 backing keeps RAWINPUT aligned.
    let mut buf = vec![0u64; (size as usize).div_ceil(8)];
    // SAFETY: `buf` has at least `size` bytes.
    let r = unsafe {
        GetRawInputData(
            hraw,
            RID_INPUT,
            Some(buf.as_mut_ptr().cast()),
            &mut size,
            header,
        )
    };
    if r == u32::MAX || (r as usize) < size_of::<RAWINPUTHEADER>() {
        return None;
    }
    let raw = buf.as_ptr().cast::<RAWINPUT>();
    // SAFETY: `buf` holds a complete RAWINPUT written by Windows; dwType selects the union
    // member, and for HID the variable-length data (dwSizeHid * dwCount bytes) follows
    // `bRawData` inside the same buffer of `size` bytes.
    unsafe {
        let hdr = (*raw).header;
        match hdr.dwType {
            t if t == RIM_TYPEKEYBOARD.0 => {
                let k = (*raw).data.keyboard;
                Some(RawEvent::Keyboard {
                    device: hdr.hDevice,
                    make_code: k.MakeCode,
                    flags: k.Flags,
                    vkey: k.VKey,
                    message: k.Message,
                    extra: k.ExtraInformation,
                })
            }
            t if t == RIM_TYPEHID.0 => {
                let h = &(*raw).data.hid;
                let each = h.dwSizeHid as usize;
                let count = h.dwCount as usize;
                let base = h.bRawData.as_ptr();
                let avail = size as usize - (base as usize - buf.as_ptr() as usize);
                let total = (each * count).min(avail);
                let bytes = std::slice::from_raw_parts(base, total);
                let reports = if each == 0 {
                    Vec::new()
                } else {
                    bytes.chunks(each).map(<[u8]>::to_vec).collect()
                };
                Some(RawEvent::Hid {
                    device: hdr.hDevice,
                    report_size: each,
                    reports,
                })
            }
            _ => Some(RawEvent::Other),
        }
    }
}
