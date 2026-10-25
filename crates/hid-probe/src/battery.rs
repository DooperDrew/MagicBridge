//! `hid-probe battery`: probe HID feature/input reports and PnP device properties
//! for battery reporting (M0.5).

use std::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_FriendlyName, DEVPKEY_Device_Parent,
    DEVPROP_TYPE_BYTE, DEVPROP_TYPE_STRING, DEVPROP_TYPE_UINT32, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{GUID, PCWSTR};

use crate::hid::{HidDevice, usage_name};
use crate::list;
use crate::out::hex;
use crate::outln;

/// Undocumented-but-standard Windows Bluetooth battery level property:
/// `{104ea319-6ee2-4701-bd47-8ddb4425bbe5}`, pid 2 (`DEVPROP_TYPE_BYTE`, 0..=100).
const PKEY_BLUETOOTH_BATTERY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddb4425bbe5),
    pid: 2,
};

/// Extra report IDs worth probing even if not listed in the descriptor
/// (0x90 = Apple Magic Keyboard battery report in Linux `hid-magicmouse`/`hid-apple`,
/// 0x47 = Apple wireless battery on older models, 0x20 = Generic HID battery).
const CANDIDATE_REPORT_IDS: &[u8] = &[0x90, 0x47, 0x20, 0x13, 0x01, 0x02];

pub fn run(all: bool) {
    outln!("== Battery probe: HID feature & input reports ==");
    let cols = list::collections(all);
    if cols.is_empty() {
        outln!("  no matching HID collections.");
    }
    for (i, c) in cols.iter().enumerate() {
        let (page, usage) = c.usage();
        outln!();
        outln!(
            "[{i}] {} {} (vid={:#06x} pid={:#06x} mi={} col={})",
            c.info.transport,
            usage_name(page, usage),
            c.vid,
            c.pid,
            c.info.interface.map_or("-".into(), |v| v.to_string()),
            c.info.collection.map_or("-".into(), |v| v.to_string()),
        );
        let dev = HidDevice::open_read(&c.path).or_else(|_| HidDevice::open_query(&c.path));
        let Ok(dev) = dev else {
            outln!("    could not open collection");
            continue;
        };
        let Some(caps) = c.caps else {
            outln!("    could not read caps");
            continue;
        };

        // 1. Declared feature reports + well-known candidate IDs.
        let mut feat_ids = c.report_ids("feature");
        for &id in CANDIDATE_REPORT_IDS {
            if !feat_ids.contains(&id) {
                feat_ids.push(id);
            }
        }
        let feat_len = if caps.FeatureReportByteLength > 0 {
            usize::from(caps.FeatureReportByteLength)
        } else {
            16
        };
        for id in feat_ids {
            let mut buf = vec![0u8; feat_len];
            buf[0] = id;
            match dev.get_feature(&mut buf) {
                Ok(()) => outln!(
                    "    GetFeature(id={id:#04x}, len={feat_len}): {}",
                    hex(&buf)
                ),
                Err(err) => {
                    if c.report_ids("feature").contains(&id) {
                        outln!("    GetFeature(id={id:#04x}, len={feat_len}): Win32 error {err}");
                    }
                }
            }
        }

        // 2. Declared input reports + candidate IDs via HidD_GetInputReport.
        let mut in_ids = c.report_ids("input");
        for &id in CANDIDATE_REPORT_IDS {
            if !in_ids.contains(&id) {
                in_ids.push(id);
            }
        }
        let in_len = if caps.InputReportByteLength > 0 {
            usize::from(caps.InputReportByteLength)
        } else {
            16
        };
        for id in in_ids {
            let mut buf = vec![0u8; in_len];
            buf[0] = id;
            match dev.get_input_report(&mut buf) {
                Ok(()) => {
                    outln!(
                        "    GetInputReport(id={id:#04x}, len={in_len}): {}",
                        hex(&buf)
                    );
                    if id == 0x90 && buf.len() >= 3 {
                        let status = buf[1];
                        let pct = buf[2];
                        outln!(
                            "      -> decoded Apple Battery report 0x90: level={pct}% status={status:#04x} (charging={} discharging={} ac_present={})",
                            u8::from(status & 0x01 != 0),
                            u8::from(status & 0x02 != 0),
                            u8::from(status & 0x04 != 0),
                        );
                    }
                }
                Err(err) => {
                    if c.report_ids("input").contains(&id) {
                        outln!("    GetInputReport(id={id:#04x}, len={in_len}): Win32 error {err}");
                    }
                }
            }
        }
    }

    outln!();
    outln!("== Battery probe: PnP / Bluetooth device properties ==");
    probe_pnp_devices(all);
}

fn probe_pnp_devices(all: bool) {
    // SAFETY: valid flags, enumerating all present device nodes.
    let Ok(set) = (unsafe {
        SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
    }) else {
        outln!("  SetupDiGetClassDevsW failed");
        return;
    };

    for idx in 0.. {
        let mut dev = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: valid device info set and initialized SP_DEVINFO_DATA.
        if unsafe { SetupDiEnumDeviceInfo(set, idx, &mut dev) }.is_err() {
            break;
        }
        let instance = instance_id(set, &dev).unwrap_or_default();
        let up = instance.to_ascii_uppercase();
        let is_apple = up.contains("05AC") || up.contains("004C");
        let batt = read_prop_byte(set, &dev, &PKEY_BLUETOOTH_BATTERY);
        if !is_apple && batt.is_none() && !(all && up.starts_with("BTH")) {
            continue;
        }
        let name = read_prop_string(set, &dev, &DEVPKEY_Device_FriendlyName)
            .or_else(|| read_prop_string(set, &dev, &DEVPKEY_Device_BusReportedDeviceDesc))
            .unwrap_or_default();
        let parent = read_prop_string(set, &dev, &DEVPKEY_Device_Parent).unwrap_or_default();
        outln!(
            "  {instance}  name={name:?}  bt_battery={}  parent={parent:?}",
            batt.map_or("-".into(), |b| format!("{b}%"))
        );
    }

    // SAFETY: `set` was returned by SetupDiGetClassDevsW and is freed once.
    let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
}

fn instance_id(
    set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    dev: &SP_DEVINFO_DATA,
) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut req = 0u32;
    // SAFETY: valid set, devinfo and UTF-16 buffer.
    unsafe { SetupDiGetDeviceInstanceIdW(set, dev, Some(&mut buf), Some(&mut req)) }.ok()?;
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn read_prop_byte(
    set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    dev: &SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<u8> {
    let mut ptype = DEVPROPTYPE::default();
    let mut buf = [0u8; 8];
    let mut req = 0u32;
    // SAFETY: valid set, devinfo, key and byte buffer.
    unsafe {
        SetupDiGetDevicePropertyW(set, dev, key, &mut ptype, Some(&mut buf), Some(&mut req), 0)
    }
    .ok()?;
    match ptype {
        DEVPROP_TYPE_BYTE => Some(buf[0]),
        DEVPROP_TYPE_UINT32 if req >= 4 => Some(buf[0]),
        _ => None,
    }
}

fn read_prop_string(
    set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    dev: &SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<String> {
    let mut ptype = DEVPROPTYPE::default();
    let mut buf = [0u8; 512];
    let mut req = 0u32;
    // SAFETY: valid set, devinfo, key and byte buffer.
    unsafe {
        SetupDiGetDevicePropertyW(set, dev, key, &mut ptype, Some(&mut buf), Some(&mut req), 0)
    }
    .ok()?;
    if ptype != DEVPROP_TYPE_STRING || req < 2 {
        return None;
    }
    let words: Vec<u16> = buf[..req as usize]
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(u16::from_le_bytes)
        .collect();
    let len = words.iter().position(|&c| c == 0).unwrap_or(words.len());
    Some(String::from_utf16_lossy(&words[..len]))
}
