//! Parsing of Windows HID device interface paths / Raw Input device names.
//!
//! Windows encodes bus, vendor, product, interface and top-level collection in
//! the device path, for example:
//!
//! - USB: `\\?\HID#VID_05AC&PID_026C&MI_01&Col01#8&2fa5df45&0&0000#{4d1e55b2-...}`
//! - Bluetooth Classic: `\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&000205ac_PID&026c&Col01#...`
//! - Bluetooth LE: `\\?\HID#{00001812-0000-1000-8000-00805f9b34fb}_Dev_VID&02045e_PID&0b13_REV&0509_...`
//!
//! This module is pure string logic so it can be unit-tested and reused by the
//! agent (device filtering) and by `hid-probe`.

use core::fmt;

/// How a HID device is attached to the PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// USB (for the A1843: Lightning cable).
    Usb,
    /// Bluetooth Classic HID profile (service UUID 0x1124).
    Bluetooth,
    /// Bluetooth Low Energy HID-over-GATT (service UUID 0x1812).
    BluetoothLe,
    /// Anything else (I2C, virtual, unknown format).
    Unknown,
}

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Transport::Usb => "USB",
            Transport::Bluetooth => "Bluetooth",
            Transport::BluetoothLe => "Bluetooth LE",
            Transport::Unknown => "unknown",
        })
    }
}

/// Information extracted from a HID device path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HidPathInfo {
    pub transport: Transport,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    /// USB interface number (`MI_xx`), if present.
    pub interface: Option<u8>,
    /// Top-level collection index (`Colxx`), if present.
    pub collection: Option<u8>,
}

const BT_CLASSIC_HID_UUID: &str = "{00001124-0000-1000-8000-00805f9b34fb}";
const BT_LE_HID_UUID: &str = "{00001812-0000-1000-8000-00805f9b34fb}";

/// Parses a HID device interface path or Raw Input device name.
///
/// Parsing is case-insensitive and never fails: unknown parts are `None`.
#[must_use]
pub fn parse_hid_path(path: &str) -> HidPathInfo {
    let lower = path.to_ascii_lowercase();
    // Only look at the hardware-id segment: `hid#<this part>#<instance>#{class guid}`.
    let hw = hardware_segment(&lower);

    let transport = if hw.starts_with(BT_CLASSIC_HID_UUID) {
        Transport::Bluetooth
    } else if hw.starts_with(BT_LE_HID_UUID) {
        Transport::BluetoothLe
    } else if hw.starts_with("vid_") {
        Transport::Usb
    } else {
        Transport::Unknown
    };

    HidPathInfo {
        transport,
        vid: hex_after(hw, &["vid_", "vid&"]).map(|v| (v & 0xFFFF) as u16),
        pid: hex_after(hw, &["pid_", "pid&"]).map(|v| (v & 0xFFFF) as u16),
        interface: hex_after(hw, &["&mi_"]).and_then(|v| u8::try_from(v).ok()),
        collection: hex_after(hw, &["&col"]).and_then(|v| u8::try_from(v).ok()),
    }
}

/// Returns the hardware-id segment (between the first and second `#`) of a
/// lower-cased path, or the whole string if it has no `#`.
fn hardware_segment(lower: &str) -> &str {
    let mut parts = lower.split('#');
    match (parts.next(), parts.next()) {
        (Some(_), Some(hw)) => hw,
        _ => lower,
    }
}

/// Finds the first marker and parses the hex digits right after it.
/// Bluetooth VIDs carry a vendor-source prefix (`000205ac`, `02045e`), so the
/// caller masks the value to 16 bits.
fn hex_after(s: &str, markers: &[&str]) -> Option<u32> {
    markers.iter().find_map(|m| {
        let start = s.find(m)? + m.len();
        let digits: &str = &s[start..];
        let end = digits
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(digits.len());
        if end == 0 || end > 8 {
            return None;
        }
        u32::from_str_radix(&digits[..end], 16).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_path() {
        let p = parse_hid_path(
            r"\\?\hid#vid_05ac&pid_026c&mi_01&col02#8&2fa5df45&0&0001#{4d1e55b2-f16f-11cf-88cb-001111000030}",
        );
        assert_eq!(p.transport, Transport::Usb);
        assert_eq!(p.vid, Some(0x05AC));
        assert_eq!(p.pid, Some(0x026C));
        assert_eq!(p.interface, Some(1));
        assert_eq!(p.collection, Some(2));
    }

    #[test]
    fn usb_raw_input_name_uppercase_no_collection() {
        let p = parse_hid_path(
            r"\\?\HID#VID_05AC&PID_026C&MI_00#7&abc&0&0000#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}",
        );
        assert_eq!(p.transport, Transport::Usb);
        assert_eq!(p.vid, Some(0x05AC));
        assert_eq!(p.interface, Some(0));
        assert_eq!(p.collection, None);
    }

    #[test]
    fn bluetooth_classic_path() {
        let p = parse_hid_path(
            r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&000205ac_PID&026c&Col01#9&1234&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}",
        );
        assert_eq!(p.transport, Transport::Bluetooth);
        assert_eq!(p.vid, Some(0x05AC));
        assert_eq!(p.pid, Some(0x026C));
        assert_eq!(p.interface, None);
        assert_eq!(p.collection, Some(1));
    }

    #[test]
    fn bluetooth_le_path() {
        let p = parse_hid_path(
            r"\\?\HID#{00001812-0000-1000-8000-00805f9b34fb}_Dev_VID&02045e_PID&0b13_REV&0509_aabbccddeeff&Col03#a&1&0&0002#{4d1e55b2-f16f-11cf-88cb-001111000030}",
        );
        assert_eq!(p.transport, Transport::BluetoothLe);
        assert_eq!(p.vid, Some(0x045E));
        assert_eq!(p.pid, Some(0x0B13));
        assert_eq!(p.collection, Some(3));
    }

    #[test]
    fn garbage_is_unknown() {
        let p = parse_hid_path("not a path");
        assert_eq!(p.transport, Transport::Unknown);
        assert_eq!(p.vid, None);
        assert_eq!(p.pid, None);
    }
}
