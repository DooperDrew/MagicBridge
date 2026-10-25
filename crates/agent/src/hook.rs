//! Low-level Windows keyboard hook (`WH_KEYBOARD_LL`) and remapping dispatch.
//!
//! Conforms strictly to:
//! - `NFR-P3`: <0.1ms latency, zero heap allocations, zero locks, zero file/console I/O in hook path.
//! - `NFR-S2`: presence-based auto-pausing when no Apple keyboard is connected.
//! - `NFR-S3`: stuck-key resilience via active key tracking and cleanup on unload/disable.
//! - `docs/hardware-findings.md` §4.1: ISO Italian `AltGr` synthetic `sc=0x21D` Left Control suppression.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use magicbridge_core::CompiledMappingTable;
use magicbridge_core::engine::RemapAction;
use magicbridge_core::key::{
    Key, MAGICBRIDGE_EXTRA_INFO, SC_ISO_ALTGR_SYNTHETIC_LCTRL, VK_LCONTROL,
};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::injector;

/// Bit flags for packing tracked active remap state into a single `AtomicU32`.
const BIT_REMAPPED: u32 = 1 << 31;
const BIT_EXTENDED: u32 = 1 << 24;

/// Tracked state of a physical virtual key code currently down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// Key is not active or passed through unmodified.
    None,
    /// Key was swallowed.
    Swallowed,
    /// Key was remapped to the target [`Key`].
    Remapped(Key),
}

/// Action decided for an intercepted keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookAction {
    /// Pass original event along to the next hook in the chain.
    CallNextHook,
    /// Swallow the event completely (return non-zero without calling next hook).
    Swallow,
    /// Swallow original event and inject synthetic key-down.
    InjectKeyDown(Key),
    /// Swallow original event and inject synthetic key-up.
    InjectKeyUp(Key),
    /// Swallow original event and inject synthetic shortcut combo.
    InjectShortcut {
        /// Keys in the shortcut combo.
        keys: [Key; 4],
        /// Number of valid keys in the combo.
        len: u8,
    },
    /// Swallow original event and trigger off-thread custom action by ID (NFR-P3).
    TriggerCustomAction(u32),
}

/// Active state tracking for virtual keys `0..=255`.
///
/// Each slot holds an encoded `KeyState` as a lock-free `AtomicU32`.
static ACTIVE_REMAPS: [AtomicU32; 256] = [const { AtomicU32::new(0) }; 256];

/// Whether the Fn modifier key is currently held down.
static FN_DOWN: AtomicBool = AtomicBool::new(false);

/// Global hook active flag (set to false when paused or disconnected).
static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Raw atomic pointer to the current [`CompiledMappingTable`].
///
/// Evaluated in the hook callback with zero locks or heap allocation.
static ACTIVE_TABLE: AtomicPtr<CompiledMappingTable> = AtomicPtr::new(std::ptr::null_mut());

/// Safe container keeping heap ownership of the active mapping table.
static TABLE_HOLDER: Mutex<Option<Box<CompiledMappingTable>>> = Mutex::new(None);

/// Encodes a target [`Key`] into a packed `u32` for atomic storage.
#[inline]
pub fn encode_remap(target: Key) -> u32 {
    let mut val =
        BIT_REMAPPED | ((target.scan_code as u32 & 0xFFFF) << 8) | (target.vk as u32 & 0xFF);
    if target.extended {
        val |= BIT_EXTENDED;
    }
    val
}

/// Decodes a packed `u32` back into a [`KeyState`].
#[inline]
pub fn decode_remap(val: u32) -> KeyState {
    if val == 0 {
        KeyState::None
    } else if val == 1 {
        KeyState::Swallowed
    } else if (val & BIT_REMAPPED) != 0 {
        let vk = (val & 0xFF) as u16;
        let scan_code = ((val >> 8) & 0xFFFF) as u16;
        let extended = (val & BIT_EXTENDED) != 0;
        KeyState::Remapped(Key::new(vk, scan_code, extended))
    } else {
        KeyState::None
    }
}

/// Sets the active compiled mapping table.
///
/// Updates the atomic pointer so the hook hot path can read it without locking.
pub fn set_active_table(table: CompiledMappingTable) {
    let mut guard = TABLE_HOLDER.lock().unwrap();
    let boxed = Box::new(table);
    let ptr = &*boxed as *const CompiledMappingTable as *mut CompiledMappingTable;
    *guard = Some(boxed);
    ACTIVE_TABLE.store(ptr, Ordering::Release);
}

/// Clears the active mapping table.
pub fn clear_active_table() {
    let mut guard = TABLE_HOLDER.lock().unwrap();
    ACTIVE_TABLE.store(std::ptr::null_mut(), Ordering::Release);
    *guard = None;
}

/// Returns whether the hook currently has an active compiled mapping table.
#[must_use]
pub fn has_active_table() -> bool {
    !ACTIVE_TABLE.load(Ordering::Acquire).is_null()
}

/// Retrieves a cloned custom action from the active table for off-thread execution.
#[must_use]
pub fn get_active_custom_action(id: u32) -> Option<magicbridge_core::CustomAction> {
    let guard = TABLE_HOLDER.lock().unwrap();
    guard
        .as_ref()
        .and_then(|t| t.get_custom_action(id).cloned())
}

/// Returns whether the Fn modifier key is currently held down.
#[must_use]
pub fn is_fn_down() -> bool {
    FN_DOWN.load(Ordering::Relaxed)
}

/// Returns whether the hook is currently enabled and active.
#[must_use]
pub fn is_hook_active() -> bool {
    IS_ACTIVE.load(Ordering::Relaxed)
}

/// Sets whether the hook is active.
///
/// When deactivating, releases all synthetic keys currently held down (NFR-S3).
pub fn set_hook_active(active: bool) {
    if !active {
        release_all_active_keys();
    }
    IS_ACTIVE.store(active, Ordering::Release);
}

/// Releases all synthetic keys currently held down and resets Fn state (NFR-S3).
pub fn release_all_active_keys() {
    for entry in &ACTIVE_REMAPS {
        let val = entry.swap(0, Ordering::AcqRel);
        if let KeyState::Remapped(target) = decode_remap(val) {
            injector::send_key_up(target.vk, target.scan_code, target.extended);
        }
    }
    FN_DOWN.store(false, Ordering::Release);
}

/// Evaluates an incoming keyboard event and decides the remapping action.
///
/// Pure logic function with zero heap allocations, zero locks, and zero I/O.
#[inline]
pub fn process_keyboard_event(
    ncode: i32,
    msg: u32,
    vk: u16,
    scan_code: u32,
    dw_extra_info: usize,
    table: Option<&CompiledMappingTable>,
    is_active: bool,
) -> HookAction {
    // 1. If ncode < 0, Windows documentation mandates immediate forwarding.
    if ncode < 0 {
        return HookAction::CallNextHook;
    }

    // 2. Self-injection prevention (Requirement 2):
    // Ignore events generated by MagicBridge via SendInput to prevent feedback loops.
    if dw_extra_info == MAGICBRIDGE_EXTRA_INFO {
        return HookAction::CallNextHook;
    }

    // 3. Fast-path if hook is inactive or paused.
    if !is_active {
        return HookAction::CallNextHook;
    }

    // 4. Fast-path if no mapping table is configured.
    let table = match table {
        Some(t) => t,
        None => return HookAction::CallNextHook,
    };

    // 5. Check if event is a key-down or key-up message.
    let is_key_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_key_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !is_key_down && !is_key_up {
        return HookAction::CallNextHook;
    }

    // 6. ISO Italian AltGr Quirk (Requirement 3, hardware-findings §4.1):
    // Swallow Windows-synthesized fake Left Control (vk=0xA2, sc=0x21D) on both down and up.
    if table.swallow_iso_altgr
        && vk == VK_LCONTROL
        && scan_code == (SC_ISO_ALTGR_SYNTHETIC_LCTRL as u32)
    {
        return HookAction::Swallow;
    }

    // 7. Fn Trigger Key tracking (Requirement 4):
    // If vk matches the configured Fn key, update fn_down and swallow the keypress.
    if table.is_fn_key(vk) {
        if is_key_down {
            FN_DOWN.store(true, Ordering::Relaxed);
        } else {
            FN_DOWN.store(false, Ordering::Relaxed);
        }
        return HookAction::Swallow;
    }

    // Keys outside 0..=255 cannot be indexed in the table.
    if vk > 255 {
        return HookAction::CallNextHook;
    }
    let idx = vk as usize;

    if is_key_down {
        let active_val = ACTIVE_REMAPS[idx].load(Ordering::Relaxed);
        let active_state = decode_remap(active_val);

        match active_state {
            KeyState::Remapped(target) => {
                // Key auto-repeat while held down
                HookAction::InjectKeyDown(target)
            }
            KeyState::Swallowed => HookAction::Swallow,
            KeyState::None => {
                let fn_down = FN_DOWN.load(Ordering::Relaxed);
                let action = table.lookup(vk, fn_down);
                match action {
                    RemapAction::PassThrough => HookAction::CallNextHook,
                    RemapAction::Swallow => {
                        ACTIVE_REMAPS[idx].store(1, Ordering::Relaxed);
                        HookAction::Swallow
                    }
                    RemapAction::RemapKey {
                        target_vk,
                        target_sc,
                        extended,
                    } => {
                        let target = Key::new(target_vk, target_sc, extended);
                        ACTIVE_REMAPS[idx].store(encode_remap(target), Ordering::Relaxed);
                        HookAction::InjectKeyDown(target)
                    }
                    RemapAction::Shortcut { keys, len } => {
                        ACTIVE_REMAPS[idx].store(1, Ordering::Relaxed);
                        HookAction::InjectShortcut { keys, len }
                    }
                    RemapAction::CustomAction(id) => {
                        ACTIVE_REMAPS[idx].store(1, Ordering::Relaxed);
                        HookAction::TriggerCustomAction(id)
                    }
                }
            }
        }
    } else {
        // Key-up: inspect and clear the active remap slot
        let active_val = ACTIVE_REMAPS[idx].swap(0, Ordering::Relaxed);
        let active_state = decode_remap(active_val);

        match active_state {
            KeyState::Remapped(target) => HookAction::InjectKeyUp(target),
            KeyState::Swallowed => HookAction::Swallow,
            KeyState::None => HookAction::CallNextHook,
        }
    }
}

/// The low-level Windows keyboard hook callback procedure (`WH_KEYBOARD_LL`).
///
/// Adheres strictly to NFR-P3: zero heap allocation, zero locks, zero file/disk I/O.
///
/// # Safety
///
/// This callback is invoked directly by Windows as a `WH_KEYBOARD_LL` hook procedure.
/// `lparam` must point to a valid `KBDLLHOOKSTRUCT` as guaranteed by Windows when `ncode >= 0`.
pub unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode < 0 {
        // SAFETY: If ncode < 0, Windows requires forwarding to CallNextHookEx without processing.
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let info_ptr = lparam.0 as *const KBDLLHOOKSTRUCT;
    if info_ptr.is_null() {
        // SAFETY: Forward unprocessable null message to CallNextHookEx.
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    // SAFETY: Windows guarantees that `info_ptr` is a valid pointer to KBDLLHOOKSTRUCT when ncode >= 0.
    let info = unsafe { &*info_ptr };

    let table_ptr = ACTIVE_TABLE.load(Ordering::Acquire);
    let table = if table_ptr.is_null() {
        None
    } else {
        // SAFETY: `table_ptr` is maintained by `TABLE_HOLDER` and points to a valid CompiledMappingTable.
        Some(unsafe { &*table_ptr })
    };

    let is_active = IS_ACTIVE.load(Ordering::Relaxed);
    let action = process_keyboard_event(
        ncode,
        wparam.0 as u32,
        info.vkCode as u16,
        info.scanCode,
        info.dwExtraInfo,
        table,
        is_active,
    );

    match action {
        HookAction::CallNextHook => {
            // SAFETY: CallNextHookEx passes unmodified keyboard input down the Windows hook chain.
            unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
        }
        HookAction::Swallow => LRESULT(1),
        HookAction::InjectKeyDown(target) => {
            injector::send_key_down(target.vk, target.scan_code, target.extended);
            LRESULT(1)
        }
        HookAction::InjectKeyUp(target) => {
            injector::send_key_up(target.vk, target.scan_code, target.extended);
            LRESULT(1)
        }
        HookAction::InjectShortcut { keys, len } => {
            injector::send_shortcut(&keys[..len as usize]);
            LRESULT(1)
        }
        HookAction::TriggerCustomAction(id) => {
            crate::action_worker::dispatch_action(id);
            LRESULT(1)
        }
    }
}

/// RAII handle managing installation and removal of the `WH_KEYBOARD_LL` hook.
#[derive(Debug)]
pub struct HookHandle {
    hook: HHOOK,
}

impl HookHandle {
    /// Installs the global low-level keyboard hook on the current thread's message loop.
    pub fn install() -> Result<Self, windows::core::Error> {
        // SAFETY: SetWindowsHookExW installs low_level_keyboard_proc for WH_KEYBOARD_LL.
        // dwThreadId is 0 for desktop-wide low-level hook.
        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0)? };
        IS_ACTIVE.store(true, Ordering::Release);
        Ok(Self { hook })
    }

    /// Sets whether the hook is actively remapping.
    pub fn set_active(&self, active: bool) {
        set_hook_active(active);
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        set_hook_active(false);
        // SAFETY: UnhookWindowsHookEx unregisters the hook registered by SetWindowsHookExW.
        unsafe {
            let _ = UnhookWindowsHookEx(self.hook);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::compile_profile;
    use magicbridge_core::config::NamedProfile;
    use magicbridge_core::key::{VK_DELETE, VK_F13, VK_HOME, VK_LEFT, VK_RMENU, VK_SNAPSHOT};
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn reset_test_state() {
        for entry in &ACTIVE_REMAPS {
            entry.store(0, Ordering::Relaxed);
        }
        FN_DOWN.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_encode_decode_remap() {
        let _guard = TEST_MUTEX.lock().unwrap();
        assert_eq!(decode_remap(0), KeyState::None);
        assert_eq!(decode_remap(1), KeyState::Swallowed);

        let k1 = Key::new(VK_SNAPSHOT, 0x37, false);
        let enc1 = encode_remap(k1);
        assert_eq!(decode_remap(enc1), KeyState::Remapped(k1));

        let k2 = Key::new(VK_DELETE, 0x53, true);
        let enc2 = encode_remap(k2);
        assert_eq!(decode_remap(enc2), KeyState::Remapped(k2));
    }

    #[test]
    fn test_self_injection_prevention() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        let res = process_keyboard_event(
            0,
            WM_KEYDOWN,
            VK_SNAPSHOT,
            0x37,
            MAGICBRIDGE_EXTRA_INFO,
            Some(&table),
            true,
        );
        assert_eq!(res, HookAction::CallNextHook);
    }

    #[test]
    fn test_hook_inactive_passes_through() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        let res = process_keyboard_event(
            0,
            WM_KEYDOWN,
            VK_F13,
            0x64,
            0,
            Some(&table),
            false, // is_active = false
        );
        assert_eq!(res, HookAction::CallNextHook);
    }

    #[test]
    fn test_negative_ncode_passes_through() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        let res = process_keyboard_event(-1, WM_KEYDOWN, VK_F13, 0x64, 0, Some(&table), true);
        assert_eq!(res, HookAction::CallNextHook);
    }

    #[test]
    fn test_iso_altgr_quirk_suppression() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);
        assert!(table.swallow_iso_altgr);

        // Synthetic LCtrl (sc=0x21D) down -> Swallow
        let res_down = process_keyboard_event(
            0,
            WM_KEYDOWN,
            VK_LCONTROL,
            SC_ISO_ALTGR_SYNTHETIC_LCTRL as u32,
            0,
            Some(&table),
            true,
        );
        assert_eq!(res_down, HookAction::Swallow);

        // Synthetic LCtrl (sc=0x21D) up -> Swallow
        let res_up = process_keyboard_event(
            0,
            WM_KEYUP,
            VK_LCONTROL,
            SC_ISO_ALTGR_SYNTHETIC_LCTRL as u32,
            0,
            Some(&table),
            true,
        );
        assert_eq!(res_up, HookAction::Swallow);

        // Physical normal Left Ctrl (sc=0x1D) -> CallNextHook (not swallowed)
        let res_normal =
            process_keyboard_event(0, WM_KEYDOWN, VK_LCONTROL, 0x1D, 0, Some(&table), true);
        assert_eq!(res_normal, HookAction::CallNextHook);
    }

    #[test]
    fn test_fn_trigger_key_tracking() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);
        assert_eq!(table.fn_vk, Some(VK_RMENU));

        // Right Option down -> sets FN_DOWN = true, swallowed
        let res_down =
            process_keyboard_event(0, WM_SYSKEYDOWN, VK_RMENU, 0x38, 0, Some(&table), true);
        assert_eq!(res_down, HookAction::Swallow);
        assert!(FN_DOWN.load(Ordering::Relaxed));

        // Right Option up -> sets FN_DOWN = false, swallowed
        let res_up = process_keyboard_event(0, WM_SYSKEYUP, VK_RMENU, 0x38, 0, Some(&table), true);
        assert_eq!(res_up, HookAction::Swallow);
        assert!(!FN_DOWN.load(Ordering::Relaxed));
    }

    #[test]
    fn test_remapped_key_flow() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        // F13 down (mapped to PrintScreen, extended=true)
        let res_down = process_keyboard_event(0, WM_KEYDOWN, VK_F13, 0x64, 0, Some(&table), true);
        assert_eq!(
            res_down,
            HookAction::InjectKeyDown(Key::new(VK_SNAPSHOT, 0x37, true))
        );

        // F13 repeat down
        let res_repeat = process_keyboard_event(0, WM_KEYDOWN, VK_F13, 0x64, 0, Some(&table), true);
        assert_eq!(
            res_repeat,
            HookAction::InjectKeyDown(Key::new(VK_SNAPSHOT, 0x37, true))
        );

        // F13 up
        let res_up = process_keyboard_event(0, WM_KEYUP, VK_F13, 0x64, 0, Some(&table), true);
        assert_eq!(
            res_up,
            HookAction::InjectKeyUp(Key::new(VK_SNAPSHOT, 0x37, true))
        );

        // Subsequent F13 up when already cleared -> CallNextHook
        let res_up2 = process_keyboard_event(0, WM_KEYUP, VK_F13, 0x64, 0, Some(&table), true);
        assert_eq!(res_up2, HookAction::CallNextHook);
    }

    #[test]
    fn test_fn_combo_with_fn_release_first() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        // 1. Press Fn (Right Option)
        let fn_down =
            process_keyboard_event(0, WM_SYSKEYDOWN, VK_RMENU, 0x38, 0, Some(&table), true);
        assert_eq!(fn_down, HookAction::Swallow);
        assert!(is_fn_down());

        // 2. Press Left Arrow while Fn is down (Fn + Left -> Home)
        let left_down = process_keyboard_event(0, WM_KEYDOWN, VK_LEFT, 0x4B, 0, Some(&table), true);
        assert_eq!(
            left_down,
            HookAction::InjectKeyDown(Key::new(VK_HOME, 0x47, true))
        );

        // 3. Release Fn first (Right Option released before Left)
        let fn_up = process_keyboard_event(0, WM_SYSKEYUP, VK_RMENU, 0x38, 0, Some(&table), true);
        assert_eq!(fn_up, HookAction::Swallow);
        assert!(!is_fn_down());

        // 4. Release Left Arrow (even though Fn is no longer down, must release Home!)
        let left_up = process_keyboard_event(0, WM_KEYUP, VK_LEFT, 0x4B, 0, Some(&table), true);
        assert_eq!(
            left_up,
            HookAction::InjectKeyUp(Key::new(VK_HOME, 0x47, true))
        );
    }

    #[test]
    fn test_passthrough_unmapped_keys() {
        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        // Key 'A' (VK_A = 0x41)
        let res_down = process_keyboard_event(0, WM_KEYDOWN, 0x41, 0x1E, 0, Some(&table), true);
        assert_eq!(res_down, HookAction::CallNextHook);

        let res_up = process_keyboard_event(0, WM_KEYUP, 0x41, 0x1E, 0, Some(&table), true);
        assert_eq!(res_up, HookAction::CallNextHook);
    }

    #[test]
    fn test_active_table_management() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        set_active_table(table);
        assert!(has_active_table());

        clear_active_table();
        assert!(!has_active_table());
    }

    #[test]
    fn test_custom_action_event_flow() {
        use magicbridge_core::config::{CustomAction, KeyAction};
        use magicbridge_core::key::VK_F16;

        let _guard = TEST_MUTEX.lock().unwrap();
        reset_test_state();
        let mut profile = NamedProfile::default_it();
        profile.mappings.insert(
            "F16".to_string(),
            KeyAction::Custom(CustomAction::Launch {
                command: "calc.exe".to_string(),
            }),
        );
        let table = compile_profile(&profile);

        // F16 down -> TriggerCustomAction(0)
        let res_down = process_keyboard_event(0, WM_KEYDOWN, VK_F16, 0x67, 0, Some(&table), true);
        assert_eq!(res_down, HookAction::TriggerCustomAction(0));

        // F16 repeat down -> Swallow (do not retrigger on repeat)
        let res_repeat = process_keyboard_event(0, WM_KEYDOWN, VK_F16, 0x67, 0, Some(&table), true);
        assert_eq!(res_repeat, HookAction::Swallow);

        // F16 up -> Swallow
        let res_up = process_keyboard_event(0, WM_KEYUP, VK_F16, 0x67, 0, Some(&table), true);
        assert_eq!(res_up, HookAction::Swallow);

        // Subsequent up when already cleared -> CallNextHook
        let res_up2 = process_keyboard_event(0, WM_KEYUP, VK_F16, 0x67, 0, Some(&table), true);
        assert_eq!(res_up2, HookAction::CallNextHook);
    }

    #[test]
    fn test_hook_handle_install_and_drop() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let hook = HookHandle::install();
        if let Ok(handle) = hook {
            assert!(is_hook_active());
            handle.set_active(false);
            assert!(!is_hook_active());
            handle.set_active(true);
            assert!(is_hook_active());
            drop(handle);
            assert!(!is_hook_active());
        }
    }
}
