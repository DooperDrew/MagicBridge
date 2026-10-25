//! `hid-probe list`: enumerate HID collections and Raw Input devices (M0.2).

use magicbridge_core::device::{HidPathInfo, parse_hid_path};
use magicbridge_core::is_apple_vendor;
use windows::Win32::Devices::HumanInterfaceDevice::{
    HIDP_CAPS, HIDP_REPORT_TYPE, HidP_Feature, HidP_Input, HidP_Output,
};

use crate::hid::{self, HidDevice, usage_name};
use crate::outln;
use crate::rawinput;

/// One HID top-level collection with everything we could learn about it.
pub struct Collection {
    pub path: String,
    pub info: HidPathInfo,
    pub vid: u16,
    pub pid: u16,
    pub version: u16,
    pub product: Option<String>,
    pub manufacturer: Option<String>,
    pub caps: Option<HIDP_CAPS>,
    pub fields: Vec<(&'static str, Vec<hid::Field>)>,
}

impl Collection {
    pub fn usage(&self) -> (u16, u16) {
        self.caps.map_or((0, 0), |c| (c.UsagePage, c.Usage))
    }

    /// Distinct report IDs for a report type (`"input"`, `"output"`, `"feature"`).
    pub fn report_ids(&self, kind: &str) -> Vec<u8> {
        let mut ids: Vec<u8> = self
            .fields
            .iter()
            .filter(|(k, _)| *k == kind)
            .flat_map(|(_, f)| f.iter().map(|x| x.report_id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

/// Enumerates HID collections; only Apple ones unless `all` is set.
pub fn collections(all: bool) -> Vec<Collection> {
    let mut out = Vec::new();
    for path in hid::enumerate_paths() {
        let info = parse_hid_path(&path);
        if !all && !info.vid.is_some_and(is_apple_vendor) {
            continue;
        }
        let dev = HidDevice::open_query(&path).ok();
        let attrs = dev.as_ref().and_then(HidDevice::attributes);
        let pre = dev.as_ref().and_then(HidDevice::preparsed);
        let caps = pre.as_ref().and_then(hid::Preparsed::caps);
        let mut fields = Vec::new();
        if let (Some(p), Some(c)) = (&pre, caps) {
            let kinds: [(&str, HIDP_REPORT_TYPE, u16, u16); 3] = [
                (
                    "input",
                    HidP_Input,
                    c.NumberInputButtonCaps,
                    c.NumberInputValueCaps,
                ),
                (
                    "output",
                    HidP_Output,
                    c.NumberOutputButtonCaps,
                    c.NumberOutputValueCaps,
                ),
                (
                    "feature",
                    HidP_Feature,
                    c.NumberFeatureButtonCaps,
                    c.NumberFeatureValueCaps,
                ),
            ];
            for (name, kind, nb, nv) in kinds {
                fields.push((name, p.fields(kind, nb, nv)));
            }
        }
        let vid = attrs.map_or(info.vid.unwrap_or(0), |a| a.VendorID);
        if !all && !is_apple_vendor(vid) {
            continue;
        }
        out.push(Collection {
            info,
            vid,
            pid: attrs.map_or(info.pid.unwrap_or(0), |a| a.ProductID),
            version: attrs.map_or(0, |a| a.VersionNumber),
            product: dev.as_ref().and_then(HidDevice::product),
            manufacturer: dev.as_ref().and_then(HidDevice::manufacturer),
            caps,
            fields,
            path,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

pub fn run(all: bool) {
    let cols = collections(all);
    outln!(
        "== HID collections ({}) ==",
        if all {
            "all vendors"
        } else {
            "Apple VID 0x05AC"
        }
    );
    if cols.is_empty() {
        outln!("  none found. Is the keyboard connected (USB cable or paired over Bluetooth)?");
    }
    for (i, c) in cols.iter().enumerate() {
        print_collection(i, c);
    }

    outln!();
    outln!("== Raw Input devices ==");
    for d in rawinput::list_devices() {
        let apple = d.vid_pid.is_some_and(|(v, _)| is_apple_vendor(v));
        if !all && !apple {
            continue;
        }
        let transport = parse_hid_path(&d.name).transport;
        outln!(
            "  handle={:#010x} type={:<8} usage={} transport={} vid:pid={}",
            d.handle.0 as usize,
            d.kind,
            usage_name(d.usage.0, d.usage.1),
            transport,
            d.vid_pid
                .map_or("?".into(), |(v, p)| format!("{v:04x}:{p:04x}"))
        );
        outln!("      {}", d.name);
    }
}

fn print_collection(i: usize, c: &Collection) {
    outln!();
    outln!("[{i}] {}", c.path);
    outln!(
        "    transport={} vid={:#06x} pid={:#06x} version={:#06x} interface={} collection={}",
        c.info.transport,
        c.vid,
        c.pid,
        c.version,
        c.info.interface.map_or("-".into(), |v| v.to_string()),
        c.info.collection.map_or("-".into(), |v| v.to_string()),
    );
    outln!(
        "    product={:?} manufacturer={:?} direct_read={}",
        c.product.as_deref().unwrap_or("?"),
        c.manufacturer.as_deref().unwrap_or("?"),
        match HidDevice::open_read(&c.path) {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("denied ({})", e.code()),
        }
    );
    let Some(caps) = c.caps else {
        outln!("    (could not read capabilities)");
        return;
    };
    outln!(
        "    top-level usage={} report bytes: input={} output={} feature={}",
        usage_name(caps.UsagePage, caps.Usage),
        caps.InputReportByteLength,
        caps.OutputReportByteLength,
        caps.FeatureReportByteLength
    );
    for (kind, fields) in &c.fields {
        for f in fields {
            let usages = if f.usage_min == f.usage_max {
                usage_name(f.usage_page, f.usage_min)
            } else {
                format!(
                    "{}..0x{:04X}",
                    usage_name(f.usage_page, f.usage_min),
                    f.usage_max
                )
            };
            let shape = match f.bit_size {
                None => format!("button x{}", f.report_count.max(1)),
                Some(b) => format!(
                    "value {b}bit x{} logical {}..{}",
                    f.report_count, f.logical_min, f.logical_max
                ),
            };
            outln!(
                "      {kind:<7} id={:#04x} {usages} [{shape}] link={}",
                f.report_id,
                usage_name(f.link_usage_page, f.link_usage)
            );
        }
    }
}
