//! `hid-probe dump`: live, timestamped dump of Raw Input, direct HID reads and
//! `WH_KEYBOARD_LL` events side by side (M0.3), with hook↔Raw Input pairing (M0.6).
//!
//! Everything runs on one thread with a message-only window, except one blocking
//! `ReadFile` thread per directly readable HID collection. NOTE: this is a
//! diagnostic tool. Its hook posts a message per event, which the agent's hook
//! must never do (AGENTS.md hard rules).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;

use magicbridge_core::device::parse_hid_path;
use magicbridge_core::is_apple_vendor;
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyNameTextW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HC_ACTION,
    HWND_MESSAGE, KBDLLHOOKSTRUCT, MSG, PostMessageW, RegisterClassW, SetWindowsHookExW,
    WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_INPUT, WM_INPUT_DEVICE_CHANGE,
    WNDCLASSW,
};
use windows::core::w;

use crate::hid::{HidDevice, usage_name};
use crate::list::{self, Collection};
use crate::out::{hex, stamp, ticks};
use crate::outln;
use crate::rawinput::{self, RawEvent};

const WM_APP_HOOK: u32 = WM_APP + 1;
/// Hook and Raw Input events closer than this are considered the same keystroke.
const PAIR_WINDOW_MS: f64 = 100.0;

static HOOK_TARGET: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub struct Options {
    pub all: bool,
    pub hook: bool,
    pub hid_read: bool,
}

pub fn run(opts: &Options) -> windows::core::Result<()> {
    let cols = list::collections(opts.all);
    outln!("== hid-probe dump ==  (Ctrl+C to stop)");
    outln!("Found {} matching HID collection(s).", cols.len());

    // SAFETY: GetModuleHandleW(None) returns the current exe's module; no preconditions.
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None) }?.into();
    let class = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance,
        lpszClassName: w!("MagicBridgeHidProbe"),
        ..Default::default()
    };
    // SAFETY: `class` is fully initialised and its strings are static.
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(windows::core::Error::from_thread());
    }
    // SAFETY: message-only window of the class registered above.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("MagicBridgeHidProbe"),
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        )
    }?;

    // Raw Input: generic keyboard, consumer control, system control + every collection
    // of the target devices (vendor pages included).
    let mut usages = vec![(0x01, 0x06), (0x0C, 0x01), (0x01, 0x80)];
    for c in &cols {
        let u = c.usage();
        if u != (0, 0) && u != (0x01, 0x02) && !usages.contains(&u) {
            usages.push(u);
        }
    }
    match rawinput::register(hwnd, &usages) {
        Ok(()) => outln!(
            "Raw Input registered for: {}",
            usages
                .iter()
                .map(|&(p, u)| format!("{p:#06x}:{u:#06x}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Err(e) => outln!("Raw Input registration FAILED: {e}"),
    }

    if opts.hook {
        HOOK_TARGET.store(hwnd.0, Ordering::Release);
        // SAFETY: valid hook procedure; LL hooks need no DLL and run on this thread's loop.
        match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_proc), Some(hinstance), 0) } {
            Ok(_) => outln!("WH_KEYBOARD_LL hook installed."),
            Err(e) => outln!("WH_KEYBOARD_LL hook FAILED: {e}"),
        }
    }

    if opts.hid_read {
        start_hid_readers(&cols);
    }

    outln!();
    outln!("Columns: [ms since start] SOURCE details");
    outln!(
        "  HOOK   = WH_KEYBOARD_LL      RAWKB = Raw Input keyboard   RAWHID = Raw Input HID report"
    );
    outln!(
        "  HIDRD  = direct ReadFile on a HID collection             PAIR  = hook/raw match (M0.6)"
    );
    outln!();

    let mut msg = MSG::default();
    // SAFETY: standard message loop on the thread that owns `hwnd` and the hook.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        // SAFETY: `msg` was filled in by GetMessageW.
        unsafe { DispatchMessageW(&msg) };
    }
    Ok(())
}

fn start_hid_readers(cols: &[Collection]) {
    for (i, c) in cols.iter().enumerate() {
        let (page, usage) = c.usage();
        let label = format!("[{i}] {} {}", c.info.transport, usage_name(page, usage));
        let len = c.caps.map_or(0, |caps| caps.InputReportByteLength as usize);
        if len == 0 {
            outln!("HIDRD  {label}: no input reports, not reading");
            continue;
        }
        match HidDevice::open_read(&c.path) {
            Ok(dev) => {
                outln!("HIDRD  {label}: reading ({len}-byte reports)");
                thread::spawn(move || {
                    let mut buf = vec![0u8; len];
                    loop {
                        match dev.read(&mut buf) {
                            Ok(n) => {
                                outln!("{} HIDRD  {label}: {}", stamp(ticks()), hex(&buf[..n]))
                            }
                            Err(e) => {
                                outln!(
                                    "{} HIDRD  {label}: read stopped (Win32 error {e})",
                                    stamp(ticks())
                                );
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => outln!(
                "HIDRD  {label}: cannot open for reading ({}) — expected for keyboards owned by Windows",
                e.message()
            ),
        }
    }
}

/// Low-level keyboard hook: packs the event into a posted message, nothing else.
unsafe extern "system" fn ll_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for HC_ACTION, lParam points to a KBDLLHOOKSTRUCT owned by the system.
        let k = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let packed = (k.vkCode as usize & 0xFFFF)
            | ((k.scanCode as usize & 0xFFFF) << 16)
            | ((k.flags.0 as usize & 0xFF) << 32)
            | ((wparam.0 & 0xFFFF) << 40)
            | (usize::from(k.dwExtraInfo != 0) << 56);
        let hwnd = HWND(HOOK_TARGET.load(Ordering::Acquire));
        // SAFETY: posting to our own message-only window; failure is harmless.
        let _ = unsafe {
            PostMessageW(
                Some(hwnd),
                WM_APP_HOOK,
                WPARAM(packed),
                LPARAM(ticks() as isize),
            )
        };
    }
    // SAFETY: always pass the event on unchanged.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[derive(Clone, Copy)]
struct Key {
    scan: u16,
    ext: bool,
    up: bool,
    t: i64,
    matched: bool,
}

#[derive(Default)]
struct State {
    labels: HashMap<usize, String>,
    hooks: VecDeque<Key>,
    raws: VecDeque<(Key, String)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn device_label(state: &mut State, h: HANDLE) -> String {
    let key = h.0 as usize;
    if key == 0 {
        return "dev=none(injected)".into();
    }
    let n = state.labels.len();
    state
        .labels
        .entry(key)
        .or_insert_with(|| {
            let d = rawinput::describe(h);
            let apple = d.vid_pid.is_some_and(|(v, _)| is_apple_vendor(v));
            let label = format!(
                "dev{n}{}({} {} {})",
                if apple { "*APPLE*" } else { "" },
                d.kind,
                parse_hid_path(&d.name).transport,
                d.vid_pid
                    .map_or("?".into(), |(v, p)| format!("{v:04x}:{p:04x}"))
            );
            outln!("       new device {label} handle={key:#x} {}", d.name);
            label
        })
        .clone()
}

fn key_name(scan: u16, ext: bool) -> String {
    let mut buf = [0u16; 64];
    let lp = (i32::from(scan) << 16) | (i32::from(ext) << 24);
    // SAFETY: valid output buffer.
    let n = unsafe { GetKeyNameTextW(lp, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

fn vk_name(vk: u16) -> String {
    let n = match vk {
        0x10 => "Shift",
        0x11 => "Ctrl",
        0x12 => "Alt",
        0x14 => "CapsLock",
        0x5B => "LWin",
        0x5C => "RWin",
        0x5D => "Apps",
        0x70..=0x87 => return format!("F{}", vk - 0x6F),
        0xA0 => "LShift",
        0xA1 => "RShift",
        0xA2 => "LCtrl",
        0xA3 => "RCtrl",
        0xA4 => "LAlt",
        0xA5 => "RAlt",
        0xAD => "VolumeMute",
        0xAE => "VolumeDown",
        0xAF => "VolumeUp",
        0xB0 => "MediaNext",
        0xB1 => "MediaPrev",
        0xB2 => "MediaStop",
        0xB3 => "MediaPlayPause",
        0x0C => "Clear",
        0xFF => "0xFF(none)",
        _ => "",
    };
    n.to_string()
}

fn pair(state: &mut State, from_hook: bool, k: Key) -> Option<String> {
    let close = |o: &Key| {
        !o.matched
            && o.scan == k.scan
            && o.ext == k.ext
            && o.up == k.up
            && ((k.t - o.t) as f64).abs() * 1000.0 / freq() <= PAIR_WINDOW_MS
    };
    if from_hook {
        let (o, dev) = state.raws.iter_mut().find(|(o, _)| close(o))?;
        o.matched = true;
        Some(format!(
            "PAIR   hook after raw by {:+.3} ms  -> {dev}",
            (k.t - o.t) as f64 * 1000.0 / freq()
        ))
    } else {
        let o = state.hooks.iter_mut().find(|o| close(o))?;
        o.matched = true;
        Some(format!(
            "PAIR   raw after hook by {:+.3} ms",
            (k.t - o.t) as f64 * 1000.0 / freq()
        ))
    }
}

fn freq() -> f64 {
    let mut f = 0i64;
    // SAFETY: valid out pointer.
    let _ = unsafe { windows::Win32::System::Performance::QueryPerformanceFrequency(&mut f) };
    f.max(1) as f64
}

fn push<T>(q: &mut VecDeque<T>, v: T) {
    if q.len() >= 64 {
        q.pop_front();
    }
    q.push_back(v);
}

fn on_hook(wparam: WPARAM, lparam: LPARAM) {
    let p = wparam.0;
    let vk = (p & 0xFFFF) as u16;
    let scan = ((p >> 16) & 0xFFFF) as u16;
    let flags = ((p >> 32) & 0xFF) as u32;
    let msg = ((p >> 40) & 0xFFFF) as u32;
    let extra = (p >> 56) & 1;
    let t = lparam.0 as i64;
    let ext = flags & 0x01 != 0;
    let up = flags & 0x80 != 0;
    outln!(
        "{} HOOK   {:<4} vk={vk:#04x} {:<14} sc={scan:#04x} ext={} inj={} lowil={} alt={} sys={} extra={} key={:?}",
        stamp(t),
        if up { "up" } else { "down" },
        vk_name(vk),
        u8::from(ext),
        u8::from(flags & 0x10 != 0),
        u8::from(flags & 0x02 != 0),
        u8::from(flags & 0x20 != 0),
        u8::from(msg == 0x104 || msg == 0x105),
        extra,
        key_name(scan, ext),
    );
    let k = Key {
        scan,
        ext,
        up,
        t,
        matched: false,
    };
    STATE.with_borrow_mut(|s| {
        if let Some(line) = pair(s, true, k) {
            outln!("{} {line}", stamp(t));
        } else {
            push(&mut s.hooks, k);
        }
    });
}

fn on_raw(lparam: LPARAM) {
    let t = ticks();
    let Some(ev) = rawinput::read_event(lparam) else {
        return;
    };
    STATE.with_borrow_mut(|s| match ev {
        RawEvent::Keyboard { device, make_code, flags, vkey, message, extra } => {
            let dev = device_label(s, device);
            let up = flags & 0x01 != 0;
            let e0 = flags & 0x02 != 0;
            outln!(
                "{} RAWKB  {:<4} vk={vkey:#04x} {:<14} sc={make_code:#04x} e0={} e1={} msg={message:#06x} extra={extra:#x} {dev}",
                stamp(t),
                if up { "up" } else { "down" },
                vk_name(vkey),
                u8::from(e0),
                u8::from(flags & 0x04 != 0),
            );
            let k = Key { scan: make_code, ext: e0, up, t, matched: false };
            if let Some(line) = pair(s, false, k) {
                outln!("{} {line} -> {dev}", stamp(t));
            } else {
                push(&mut s.raws, (k, dev));
            }
        }
        RawEvent::Hid { device, report_size, reports } => {
            let dev = device_label(s, device);
            for r in reports {
                outln!("{} RAWHID {dev} len={report_size}: {}", stamp(t), hex(&r));
            }
        }
        RawEvent::Other => {}
    });
}

fn on_device_change(wparam: WPARAM, lparam: LPARAM) {
    let h = HANDLE(lparam.0 as *mut c_void);
    let what = if wparam.0 == 1 { "arrival" } else { "removal" };
    STATE.with_borrow_mut(|s| {
        let label = if wparam.0 == 1 {
            device_label(s, h)
        } else {
            s.labels
                .get(&(h.0 as usize))
                .cloned()
                .unwrap_or_else(|| format!("handle={:#x}", h.0 as usize))
        };
        outln!("{} DEVICE {what} {label}", stamp(ticks()));
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_APP_HOOK => {
            on_hook(wparam, lparam);
            LRESULT(0)
        }
        WM_INPUT => {
            on_raw(lparam);
            // SAFETY: WM_INPUT must still reach DefWindowProc for cleanup.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_INPUT_DEVICE_CHANGE => {
            on_device_change(wparam, lparam);
            LRESULT(0)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
