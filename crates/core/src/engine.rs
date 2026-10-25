//! Pure remapping engine and precomputed lookup table compiler.
//!
//! Compiles high-level [`NamedProfile`] configurations into flat, precomputed,
//! zero-allocation [`CompiledMappingTable`] structures suitable for O(1) evaluation
//! in performance-critical low-level hooks (`WH_KEYBOARD_LL`).
//!
//! Complies strictly with `NFR-Q1` (pure safe Rust, `#![forbid(unsafe_code)]`, zero Win32 crate dependencies)
//! and `NFR-P3` (zero allocations, zero locks, zero I/O in the evaluation path).

use serde::{Deserialize, Serialize};

use crate::config::{
    CustomAction, FnKeyChoice, KeyAction, ModifierPreset, NamedProfile, TopRowMode,
};
use crate::key::{
    Key, SC_ISO_ALTGR_SYNTHETIC_LCTRL, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_F3, VK_F4, VK_F7,
    VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F19, VK_HOME, VK_INSERT,
    VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
    VK_MEDIA_PREV_TRACK, VK_NEXT, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU,
    VK_RWIN, VK_SCROLL, VK_SNAPSHOT, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    parse_key_name,
};

/// Action resulting from evaluating an intercepted keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RemapAction {
    /// Pass the original keystroke through unmodified.
    #[default]
    PassThrough,
    /// Remap the keystroke to another virtual key code with specified scancode and extended flag.
    RemapKey {
        /// Destination Windows Virtual Key code (`0..=255`).
        target_vk: u16,
        /// Destination hardware make/break scancode.
        target_sc: u16,
        /// Whether the destination key requires `KEYEVENTF_EXTENDEDKEY` (`LLKHF_EXTENDED`).
        extended: bool,
    },
    /// Remaps the keystroke to a multi-key shortcut sequence (e.g. Win + Tab).
    Shortcut {
        /// Array of keys in the shortcut sequence (up to 4 keys).
        keys: [Key; 4],
        /// Number of valid keys in `keys` (`1..=4`).
        len: u8,
    },
    /// Triggers an off-thread custom action (e.g. launch program, URL, or key combination).
    ///
    /// The `u32` value represents an index into the table's registered `custom_actions` list.
    CustomAction(u32),
    /// Silently swallow the keystroke (e.g. Fn modifier key, or synthetic `sc=0x21D` Left Control).
    Swallow,
}

impl RemapAction {
    /// Creates a remapped key action targeting a specific virtual key, scancode, and extended flag.
    #[must_use]
    pub const fn key(target_vk: u16, target_sc: u16, extended: bool) -> Self {
        Self::RemapKey {
            target_vk,
            target_sc,
            extended,
        }
    }

    /// Creates a remapped key action from a high-level [`Key`].
    #[must_use]
    pub fn from_key(key: Key) -> Self {
        Self::RemapKey {
            target_vk: key.vk,
            target_sc: key.scan_code,
            extended: key.extended,
        }
    }

    /// Creates a remapped key action targeting the standard default scancode and extended flag for `vk`.
    #[must_use]
    pub fn from_vk(vk: u16) -> Self {
        Self::from_key(Key::from_vk(vk))
    }

    /// Creates a shortcut action from a slice of [`Key`]s (up to 4 keys).
    #[must_use]
    pub const fn shortcut(keys: &[Key]) -> Self {
        let mut arr = [Key::new(0, 0, false); 4];
        let len = if keys.len() > 4 { 4 } else { keys.len() } as u8;
        let mut i = 0;
        while i < len as usize {
            arr[i] = keys[i];
            i += 1;
        }
        Self::Shortcut { keys: arr, len }
    }

    /// Creates a shortcut action from a slice of virtual key codes.
    #[must_use]
    pub fn shortcut_from_vks(vks: &[u16]) -> Self {
        let mut keys = [Key::new(0, 0, false); 4];
        let len = vks.len().min(4) as u8;
        for (i, &vk) in vks.iter().take(4).enumerate() {
            keys[i] = Key::from_vk(vk);
        }
        Self::Shortcut { keys, len }
    }

    /// Returns `true` if this action is [`RemapAction::PassThrough`].
    #[must_use]
    pub const fn is_passthrough(&self) -> bool {
        matches!(self, Self::PassThrough)
    }

    /// Returns `true` if this action is [`RemapAction::Swallow`].
    #[must_use]
    pub const fn is_swallow(&self) -> bool {
        matches!(self, Self::Swallow)
    }

    /// Returns `true` if this action is [`RemapAction::Shortcut`].
    #[must_use]
    pub const fn is_shortcut(&self) -> bool {
        matches!(self, Self::Shortcut { .. })
    }

    /// Returns the target [`Key`] representation if this action is [`RemapAction::RemapKey`].
    #[must_use]
    pub const fn target_key(&self) -> Option<Key> {
        match *self {
            Self::RemapKey {
                target_vk,
                target_sc,
                extended,
            } => Some(Key::new(target_vk, target_sc, extended)),
            _ => None,
        }
    }

    /// Returns the slice of keys if this action is a [`RemapAction::Shortcut`].
    #[must_use]
    pub fn shortcut_keys(&self) -> Option<&[Key]> {
        match self {
            Self::Shortcut { keys, len } => Some(&keys[..*len as usize]),
            _ => None,
        }
    }

    /// Returns the registered custom action index if this action is [`RemapAction::CustomAction`].
    #[must_use]
    pub const fn custom_action_id(&self) -> Option<u32> {
        match *self {
            Self::CustomAction(id) => Some(id),
            _ => None,
        }
    }
}

/// Precomputed flat mapping table for ultra-low latency, zero-allocation hook evaluation.
///
/// Holds two 256-element direct lookup arrays indexed by Windows Virtual Key code:
/// - `normal_layer`: evaluated when Fn key is NOT held down.
/// - `fn_layer`: evaluated when Fn key IS held down.
///
/// Designed to be atomically swapped via `Arc<CompiledMappingTable>` in the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMappingTable {
    /// Normal layer lookup array indexed by virtual key code (`0..=255`).
    pub normal_layer: [RemapAction; 256],
    /// Fn layer lookup array indexed by virtual key code (`0..=255`).
    pub fn_layer: [RemapAction; 256],
    /// Virtual key code configured as the Fn trigger key, if enabled.
    pub fn_vk: Option<u16>,
    /// Whether synthetic `sc=0x21D` Left Control must be swallowed (active on ISO Italian when
    /// Right Option is remapped or used as Fn).
    pub swallow_iso_altgr: bool,
    /// Registered custom actions indexed by the `u32` payload of [`RemapAction::CustomAction`].
    pub custom_actions: Vec<CustomAction>,
}

impl Default for CompiledMappingTable {
    fn default() -> Self {
        Self {
            normal_layer: [RemapAction::PassThrough; 256],
            fn_layer: [RemapAction::PassThrough; 256],
            fn_vk: None,
            swallow_iso_altgr: false,
            custom_actions: Vec::new(),
        }
    }
}

impl CompiledMappingTable {
    /// O(1) table lookup for a virtual key code in the normal or Fn layer.
    ///
    /// Performs direct array indexing without any heap allocation or locking (`NFR-P3`).
    /// If `vk > 255`, passes through unmodified.
    #[must_use]
    pub const fn lookup(&self, vk: u16, fn_down: bool) -> RemapAction {
        if vk > 255 {
            return RemapAction::PassThrough;
        }
        let idx = vk as usize;
        if fn_down {
            self.fn_layer[idx]
        } else {
            self.normal_layer[idx]
        }
    }

    /// Complete evaluation of an incoming keyboard event (`vk`, `scan_code`, `fn_down`).
    ///
    /// Evaluates in order:
    /// 1. ISO AltGr synthetic Left Control check (`vk == VK_LCONTROL && scan_code == 0x21D`).
    /// 2. Fn trigger key check (swallows the keypress to avoid emitting character output).
    /// 3. Direct O(1) lookup in the active layer (`normal_layer` or `fn_layer`).
    #[must_use]
    pub const fn eval(&self, vk: u16, scan_code: u32, fn_down: bool) -> RemapAction {
        if self.should_swallow_synthetic_lctrl(vk, scan_code) {
            return RemapAction::Swallow;
        }
        if self.is_fn_key(vk) {
            return RemapAction::Swallow;
        }
        self.lookup(vk, fn_down)
    }

    /// Returns `true` if `vk` is configured as the active Fn trigger key.
    #[must_use]
    pub const fn is_fn_key(&self, vk: u16) -> bool {
        match self.fn_vk {
            Some(fn_code) => fn_code == vk,
            None => false,
        }
    }

    /// Returns `true` if an incoming key event matches the ISO Italian/AltGr
    /// synthetic Left Control (`vk == VK_LCONTROL`, `scanCode == 0x21D`) that must be suppressed.
    #[must_use]
    pub const fn should_swallow_synthetic_lctrl(&self, vk: u16, scan_code: u32) -> bool {
        self.swallow_iso_altgr
            && vk == VK_LCONTROL
            && scan_code == (SC_ISO_ALTGR_SYNTHETIC_LCTRL as u32)
    }

    /// Retrieves a registered custom action by index.
    #[must_use]
    pub fn get_custom_action(&self, id: u32) -> Option<&CustomAction> {
        self.custom_actions.get(id as usize)
    }

    /// Returns the virtual key code of the configured Fn trigger key, if any.
    #[must_use]
    pub const fn fn_vk(&self) -> Option<u16> {
        self.fn_vk
    }

    /// Returns `true` if ISO AltGr synthetic Left Control suppression is active.
    #[must_use]
    pub const fn swallow_iso_altgr(&self) -> bool {
        self.swallow_iso_altgr
    }
}

/// Compiles a high-level [`NamedProfile`] into a flat precomputed [`CompiledMappingTable`].
///
/// Compilation applies:
/// 1. **Modifier Presets (F3.1, F3.2):**
///    - `WindowsLayout`: swaps Left Cmd <-> Left Alt (`VK_LWIN` <-> `VK_LMENU`) and
///      Right Cmd <-> Right Alt (`VK_RWIN` <-> `VK_RMENU`).
///    - `MacLayout`: leaves modifiers unswapped.
///    - `Custom`: leaves modifiers unswapped (custom mappings define exact behavior).
/// 2. **Default Missing Keys (F4.1):**
///    - `F13` -> `PrintScreen` (`VK_SNAPSHOT`)
///    - `F14` -> `ScrollLock` (`VK_SCROLL`)
///    - `F15` -> `Pause` (`VK_PAUSE`)
/// 3. **Top Row Mode (F2.1, F2.3):**
///    - `MediaKeysFirst`: F7..F12 map to media keys in normal layer, and F-keys in Fn layer.
///    - `FKeysFirst`: F7..F12 stay F-keys in normal layer, and map to media keys in Fn layer.
/// 4. **Default Fn Combos (F1.2):**
///    - `Fn + Left/Right/Up/Down` -> `Home/End/PageUp/PageDown`
///    - `Fn + Backspace` -> `Forward Delete`
///    - `Fn + Enter` -> `Insert`
/// 5. **Custom Mappings & Fn Mappings:**
///    - Explicit mappings in `profile.mappings` and `profile.fn_mappings` override presets/defaults.
/// 6. **Fn Key Setup & ISO AltGr Suppression:**
///    - Configures `fn_vk` according to `profile.fn_key`.
///    - Sets `swallow_iso_altgr = true` whenever Right Option (`VK_RMENU`) is remapped or acts as Fn.
#[must_use]
pub fn compile_profile(profile: &NamedProfile) -> CompiledMappingTable {
    let mut normal_layer = [RemapAction::PassThrough; 256];
    let mut fn_layer = [RemapAction::PassThrough; 256];
    let mut custom_actions = Vec::new();

    // 1. Modifier Presets (F3.1, F3.2)
    match profile.modifier_preset {
        ModifierPreset::WindowsLayout => {
            let lmenu = RemapAction::from_vk(VK_LMENU);
            let lwin = RemapAction::from_vk(VK_LWIN);
            let rmenu = RemapAction::from_vk(VK_RMENU);
            let rwin = RemapAction::from_vk(VK_RWIN);

            // Left side swap (Cmd -> Alt, Option -> Win)
            normal_layer[VK_LWIN as usize] = lmenu;
            normal_layer[VK_LMENU as usize] = lwin;
            fn_layer[VK_LWIN as usize] = lmenu;
            fn_layer[VK_LMENU as usize] = lwin;

            // Right side swap (Cmd -> Alt, Option -> Win)
            normal_layer[VK_RWIN as usize] = rmenu;
            normal_layer[VK_RMENU as usize] = rwin;
            fn_layer[VK_RWIN as usize] = rmenu;
            fn_layer[VK_RMENU as usize] = rwin;
        }
        ModifierPreset::MacLayout | ModifierPreset::Custom => {
            // Unswapped / defined by explicit mappings
        }
    }

    // 2. Default Missing Keys (F4.1)
    let prt_sc = RemapAction::from_vk(VK_SNAPSHOT);
    let scr_lk = RemapAction::from_vk(VK_SCROLL);
    let pause = RemapAction::from_vk(VK_PAUSE);

    normal_layer[VK_F13 as usize] = prt_sc;
    normal_layer[VK_F14 as usize] = scr_lk;
    normal_layer[VK_F15 as usize] = pause;

    fn_layer[VK_F13 as usize] = prt_sc;
    fn_layer[VK_F14 as usize] = scr_lk;
    fn_layer[VK_F15 as usize] = pause;

    // 3. Top Row Mode (F2.1..F2.5)
    // F3: Mission Control -> Windows Task View (Win + Tab)
    let task_view = RemapAction::shortcut(&[Key::from_vk(VK_LWIN), Key::from_vk(VK_TAB)]);
    // F4: Launchpad -> Windows Start Menu (Win key)
    let start_menu = RemapAction::from_vk(VK_LWIN);

    let media_prev = RemapAction::from_vk(VK_MEDIA_PREV_TRACK);
    let media_play = RemapAction::from_vk(VK_MEDIA_PLAY_PAUSE);
    let media_next = RemapAction::from_vk(VK_MEDIA_NEXT_TRACK);
    let media_mute = RemapAction::from_vk(VK_VOLUME_MUTE);
    let media_vol_down = RemapAction::from_vk(VK_VOLUME_DOWN);
    let media_vol_up = RemapAction::from_vk(VK_VOLUME_UP);

    match profile.top_row_mode {
        TopRowMode::MediaKeysFirst => {
            // Standard press: special/media actions
            normal_layer[VK_F3 as usize] = task_view;
            normal_layer[VK_F4 as usize] = start_menu;
            normal_layer[VK_F7 as usize] = media_prev;
            normal_layer[VK_F8 as usize] = media_play;
            normal_layer[VK_F9 as usize] = media_next;
            normal_layer[VK_F10 as usize] = media_mute;
            normal_layer[VK_F11 as usize] = media_vol_down;
            normal_layer[VK_F12 as usize] = media_vol_up;

            // Fn + press: standard function keys
            fn_layer[VK_F3 as usize] = RemapAction::from_vk(VK_F3);
            fn_layer[VK_F4 as usize] = RemapAction::from_vk(VK_F4);
            fn_layer[VK_F7 as usize] = RemapAction::from_vk(VK_F7);
            fn_layer[VK_F8 as usize] = RemapAction::from_vk(VK_F8);
            fn_layer[VK_F9 as usize] = RemapAction::from_vk(VK_F9);
            fn_layer[VK_F10 as usize] = RemapAction::from_vk(VK_F10);
            fn_layer[VK_F11 as usize] = RemapAction::from_vk(VK_F11);
            fn_layer[VK_F12 as usize] = RemapAction::from_vk(VK_F12);
        }
        TopRowMode::FKeysFirst => {
            // Standard press: standard function keys (pass through)
            // Fn + press: special/media actions
            fn_layer[VK_F3 as usize] = task_view;
            fn_layer[VK_F4 as usize] = start_menu;
            fn_layer[VK_F7 as usize] = media_prev;
            fn_layer[VK_F8 as usize] = media_play;
            fn_layer[VK_F9 as usize] = media_next;
            fn_layer[VK_F10 as usize] = media_mute;
            fn_layer[VK_F11 as usize] = media_vol_down;
            fn_layer[VK_F12 as usize] = media_vol_up;
        }
    }

    // 4. Default Fn Combos (F1.2)
    fn_layer[VK_LEFT as usize] = RemapAction::from_vk(VK_HOME);
    fn_layer[VK_RIGHT as usize] = RemapAction::from_vk(VK_END);
    fn_layer[VK_UP as usize] = RemapAction::from_vk(VK_PRIOR);
    fn_layer[VK_DOWN as usize] = RemapAction::from_vk(VK_NEXT);
    fn_layer[VK_BACK as usize] = RemapAction::from_vk(VK_DELETE);
    fn_layer[VK_RETURN as usize] = RemapAction::from_vk(VK_INSERT);

    // 5. Apply Profile Normal Layer Mappings (overrides defaults)
    for (src_name, action) in &profile.mappings {
        if let Some(src_vk) = parse_key_name(src_name)
            && src_vk <= 255
        {
            normal_layer[src_vk as usize] = compile_key_action(action, &mut custom_actions);
        }
    }

    // Apply Profile Fn Layer Mappings (overrides defaults)
    for (src_name, action) in &profile.fn_mappings {
        if let Some(src_vk) = parse_key_name(src_name)
            && src_vk <= 255
        {
            fn_layer[src_vk as usize] = compile_key_action(action, &mut custom_actions);
        }
    }

    // 6. Fn Trigger Key Setup (F1.1, F1.4)
    let fn_vk = match profile.fn_key {
        FnKeyChoice::RightOption => Some(VK_RMENU),
        FnKeyChoice::RightControl => Some(VK_RCONTROL),
        FnKeyChoice::F19 => Some(VK_F19),
        FnKeyChoice::Disabled => None,
        FnKeyChoice::Other(vk) => Some(vk),
    };

    if let Some(code) = fn_vk
        && code <= 255
    {
        normal_layer[code as usize] = RemapAction::Swallow;
        fn_layer[code as usize] = RemapAction::Swallow;
    }

    // 7. Critical ISO Italian AltGr Quirk (sc=0x21D suppression)
    // Whenever Right Option is used as Fn or remapped to another key (e.g. Windows key),
    // the synthesized Left Control (sc=0x21D) must be swallowed.
    let swallow_iso_altgr =
        fn_vk == Some(VK_RMENU) || normal_layer[VK_RMENU as usize] != RemapAction::PassThrough;

    CompiledMappingTable {
        normal_layer,
        fn_layer,
        fn_vk,
        swallow_iso_altgr,
        custom_actions,
    }
}

/// Helper to compile a single [`KeyAction`] into a [`RemapAction`].
fn compile_key_action(action: &KeyAction, custom_actions: &mut Vec<CustomAction>) -> RemapAction {
    match action {
        KeyAction::Key(target_name) => {
            let clean = target_name.trim().to_ascii_lowercase();
            if clean == "passthrough" || clean == "none" {
                RemapAction::PassThrough
            } else if clean == "swallow" || clean == "drop" {
                RemapAction::Swallow
            } else if let Some(target_vk) = parse_key_name(target_name) {
                RemapAction::from_vk(target_vk)
            } else {
                RemapAction::PassThrough
            }
        }
        KeyAction::Custom(custom) => {
            if let CustomAction::Shortcut { keys } = custom
                && !keys.is_empty()
                && keys.len() <= 4
            {
                let mut parsed = [Key::new(0, 0, false); 4];
                let mut all_valid = true;
                for (i, k_str) in keys.iter().enumerate() {
                    if let Some(vk) = parse_key_name(k_str) {
                        parsed[i] = Key::from_vk(vk);
                    } else {
                        all_valid = false;
                        break;
                    }
                }
                if all_valid {
                    return RemapAction::Shortcut {
                        keys: parsed,
                        len: keys.len() as u8,
                    };
                }
            }
            let id = custom_actions.len() as u32;
            custom_actions.push(custom.clone());
            RemapAction::CustomAction(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeyboardLayout, ModifierPreset, NamedProfile, TopRowMode};
    use crate::key::{
        VK_A, VK_DELETE, VK_END, VK_F13, VK_F14, VK_F15, VK_F16, VK_HOME, VK_INSERT, VK_LCONTROL,
        VK_LMENU, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_NEXT,
        VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RMENU, VK_RWIN, VK_SCROLL, VK_SNAPSHOT,
        VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    };

    #[test]
    fn test_default_profile_compilation_missing_keys() {
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        // F4.1 missing Windows keys defaults
        assert_eq!(
            table.lookup(VK_F13, false),
            RemapAction::RemapKey {
                target_vk: VK_SNAPSHOT,
                target_sc: 0x37,
                extended: true,
            }
        );
        assert_eq!(
            table.lookup(VK_F14, false),
            RemapAction::RemapKey {
                target_vk: VK_SCROLL,
                target_sc: 0x46,
                extended: false,
            }
        );
        assert_eq!(
            table.lookup(VK_F15, false),
            RemapAction::RemapKey {
                target_vk: VK_PAUSE,
                target_sc: 0x45,
                extended: false,
            }
        );

        // Also active when Fn is held
        assert_eq!(
            table.lookup(VK_F13, true),
            RemapAction::RemapKey {
                target_vk: VK_SNAPSHOT,
                target_sc: 0x37,
                extended: true,
            }
        );
    }

    #[test]
    fn test_windows_layout_modifier_swap_compilation() {
        // Windows layout with Fn disabled to verify full 4-modifier swap
        let mut profile = NamedProfile::new("WinLayout", KeyboardLayout::IsoItalian);
        profile.modifier_preset = ModifierPreset::WindowsLayout;
        profile.fn_key = FnKeyChoice::Disabled;
        let table = compile_profile(&profile);

        // Left Cmd (VK_LWIN) -> Left Alt (VK_LMENU)
        assert_eq!(
            table.lookup(VK_LWIN, false),
            RemapAction::RemapKey {
                target_vk: VK_LMENU,
                target_sc: 0x38,
                extended: false,
            }
        );

        // Left Option (VK_LMENU) -> Left Windows (VK_LWIN)
        assert_eq!(
            table.lookup(VK_LMENU, false),
            RemapAction::RemapKey {
                target_vk: VK_LWIN,
                target_sc: 0x5B,
                extended: true,
            }
        );

        // Right Cmd (VK_RWIN) -> Right Alt (VK_RMENU)
        assert_eq!(
            table.lookup(VK_RWIN, false),
            RemapAction::RemapKey {
                target_vk: VK_RMENU,
                target_sc: 0x38,
                extended: true,
            }
        );

        // Right Option (VK_RMENU) -> Right Windows (VK_RWIN)
        assert_eq!(
            table.lookup(VK_RMENU, false),
            RemapAction::RemapKey {
                target_vk: VK_RWIN,
                target_sc: 0x5C,
                extended: true,
            }
        );

        // Because Right Option is remapped to Win, ISO AltGr suppression must be true
        assert!(table.swallow_iso_altgr);
    }

    #[test]
    fn test_mac_layout_preset_compilation_no_swap() {
        let mut profile = NamedProfile::new("MacLayout", KeyboardLayout::IsoItalian);
        profile.modifier_preset = ModifierPreset::MacLayout;
        profile.fn_key = FnKeyChoice::Disabled;
        let table = compile_profile(&profile);

        // Modifiers remain PassThrough
        assert_eq!(table.lookup(VK_LWIN, false), RemapAction::PassThrough);
        assert_eq!(table.lookup(VK_LMENU, false), RemapAction::PassThrough);
        assert_eq!(table.lookup(VK_RWIN, false), RemapAction::PassThrough);
        assert_eq!(table.lookup(VK_RMENU, false), RemapAction::PassThrough);

        // No ISO AltGr swallow when unswapped and Fn disabled
        assert!(!table.swallow_iso_altgr);
        assert_eq!(table.fn_vk, None);
    }

    #[test]
    fn test_custom_key_remapping_overrides() {
        let mut profile = NamedProfile::default_it();
        // Override F13 to Insert instead of PrintScreen
        profile
            .mappings
            .insert("F13".to_string(), KeyAction::key("Insert"));
        // Override F14 to Launch custom action
        profile.mappings.insert(
            "F14".to_string(),
            KeyAction::Custom(CustomAction::Launch {
                command: "calc.exe".to_string(),
            }),
        );
        // Override Fn+Enter to Delete instead of Insert
        profile
            .fn_mappings
            .insert("Enter".to_string(), KeyAction::key("Delete"));

        let table = compile_profile(&profile);

        assert_eq!(
            table.lookup(VK_F13, false),
            RemapAction::RemapKey {
                target_vk: VK_INSERT,
                target_sc: 0x52,
                extended: true,
            }
        );

        match table.lookup(VK_F14, false) {
            RemapAction::CustomAction(id) => {
                let action = table.get_custom_action(id);
                assert_eq!(
                    action,
                    Some(&CustomAction::Launch {
                        command: "calc.exe".to_string(),
                    })
                );
            }
            other => panic!("Expected CustomAction, got {other:?}"),
        }

        assert_eq!(
            table.lookup(VK_RETURN, true),
            RemapAction::RemapKey {
                target_vk: VK_DELETE,
                target_sc: 0x53,
                extended: true,
            }
        );
    }

    #[test]
    fn test_o1_lookup_normal_and_fn_layers() {
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        // Normal layer: Left Arrow is PassThrough
        assert_eq!(table.lookup(VK_LEFT, false), RemapAction::PassThrough);

        // Fn layer: Left Arrow is Home
        assert_eq!(
            table.lookup(VK_LEFT, true),
            RemapAction::RemapKey {
                target_vk: VK_HOME,
                target_sc: 0x47,
                extended: true,
            }
        );

        // Fn layer: Right -> End, Up -> PgUp, Down -> PgDn
        assert_eq!(
            table.lookup(VK_RIGHT, true),
            RemapAction::RemapKey {
                target_vk: VK_END,
                target_sc: 0x4F,
                extended: true,
            }
        );
        assert_eq!(
            table.lookup(VK_UP, true),
            RemapAction::RemapKey {
                target_vk: VK_PRIOR,
                target_sc: 0x49,
                extended: true,
            }
        );
        assert_eq!(
            table.lookup(VK_DOWN, true),
            RemapAction::RemapKey {
                target_vk: VK_NEXT,
                target_sc: 0x51,
                extended: true,
            }
        );

        // Standard letter 'A' is PassThrough in both layers
        assert_eq!(table.lookup(VK_A, false), RemapAction::PassThrough);
        assert_eq!(table.lookup(VK_A, true), RemapAction::PassThrough);

        // Key > 255 passes through safely
        assert_eq!(table.lookup(999, false), RemapAction::PassThrough);
    }

    #[test]
    fn test_fn_trigger_key_and_iso_altgr_eval() {
        let profile = NamedProfile::default_it();
        let table = compile_profile(&profile);

        assert_eq!(table.fn_vk(), Some(VK_RMENU));
        assert!(table.is_fn_key(VK_RMENU));
        assert!(!table.is_fn_key(VK_RCONTROL));
        assert!(table.swallow_iso_altgr());

        // Right Option itself must be swallowed during evaluation
        assert_eq!(table.eval(VK_RMENU, 0x38, false), RemapAction::Swallow);

        // Synthetic Left Control (sc=0x21D) must be swallowed
        assert_eq!(table.eval(VK_LCONTROL, 0x021D, false), RemapAction::Swallow);

        // Physical Left Control (sc=0x1D) must NOT be swallowed
        assert_eq!(
            table.eval(VK_LCONTROL, 0x1D, false),
            RemapAction::PassThrough
        );
    }

    #[test]
    fn test_top_row_modes_media_first_and_fkeys_first() {
        let mut profile = NamedProfile::default_it();
        profile.top_row_mode = TopRowMode::MediaKeysFirst;
        let table_media = compile_profile(&profile);

        // Media first: F3 -> Task View (Win + Tab), F4 -> Start Menu (Win)
        let expected_task_view =
            RemapAction::shortcut(&[Key::from_vk(VK_LWIN), Key::from_vk(VK_TAB)]);
        assert_eq!(table_media.lookup(VK_F3, false), expected_task_view);
        assert_eq!(
            table_media.lookup(VK_F4, false),
            RemapAction::from_vk(VK_LWIN)
        );
        assert_eq!(table_media.lookup(VK_F3, true), RemapAction::from_vk(VK_F3));
        assert_eq!(table_media.lookup(VK_F4, true), RemapAction::from_vk(VK_F4));

        // Media first: normal layer has media keys, Fn layer has F-keys
        assert_eq!(
            table_media.lookup(VK_F7, false),
            RemapAction::from_vk(VK_MEDIA_PREV_TRACK)
        );
        assert_eq!(
            table_media.lookup(VK_F8, false),
            RemapAction::from_vk(VK_MEDIA_PLAY_PAUSE)
        );
        assert_eq!(
            table_media.lookup(VK_F9, false),
            RemapAction::from_vk(VK_MEDIA_NEXT_TRACK)
        );
        assert_eq!(
            table_media.lookup(VK_F10, false),
            RemapAction::from_vk(VK_VOLUME_MUTE)
        );
        assert_eq!(
            table_media.lookup(VK_F11, false),
            RemapAction::from_vk(VK_VOLUME_DOWN)
        );
        assert_eq!(
            table_media.lookup(VK_F12, false),
            RemapAction::from_vk(VK_VOLUME_UP)
        );

        assert_eq!(table_media.lookup(VK_F7, true), RemapAction::from_vk(VK_F7));
        assert_eq!(
            table_media.lookup(VK_F12, true),
            RemapAction::from_vk(VK_F12)
        );

        // Switch to F-keys first
        profile.top_row_mode = TopRowMode::FKeysFirst;
        let table_fkeys = compile_profile(&profile);

        // F-keys first: standard press passes through as standard F-keys
        assert_eq!(table_fkeys.lookup(VK_F3, false), RemapAction::PassThrough);
        assert_eq!(table_fkeys.lookup(VK_F4, false), RemapAction::PassThrough);
        assert_eq!(table_fkeys.lookup(VK_F7, false), RemapAction::PassThrough);

        // Fn layer has special/media actions
        assert_eq!(table_fkeys.lookup(VK_F3, true), expected_task_view);
        assert_eq!(
            table_fkeys.lookup(VK_F4, true),
            RemapAction::from_vk(VK_LWIN)
        );
        assert_eq!(
            table_fkeys.lookup(VK_F7, true),
            RemapAction::from_vk(VK_MEDIA_PREV_TRACK)
        );
        assert_eq!(
            table_fkeys.lookup(VK_F12, true),
            RemapAction::from_vk(VK_VOLUME_UP)
        );
    }

    #[test]
    fn test_shortcut_custom_action_compilation() {
        let mut profile = NamedProfile::default_it();
        profile.mappings.insert(
            "F16".to_string(),
            KeyAction::Custom(CustomAction::Shortcut {
                keys: vec!["Win".to_string(), "Tab".to_string()],
            }),
        );
        let table = compile_profile(&profile);
        let action = table.lookup(VK_F16, false);
        assert!(action.is_shortcut());
        let keys = action.shortcut_keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], Key::from_vk(VK_LWIN));
        assert_eq!(keys[1], Key::from_vk(VK_TAB));
    }

    #[test]
    fn test_fn_key_choice_alternatives() {
        let mut profile = NamedProfile::default_it();
        profile.fn_key = FnKeyChoice::RightControl;
        let table = compile_profile(&profile);

        assert_eq!(table.fn_vk(), Some(VK_RCONTROL));
        assert!(table.is_fn_key(VK_RCONTROL));
        assert!(!table.is_fn_key(VK_RMENU));
        assert_eq!(table.lookup(VK_RCONTROL, false), RemapAction::Swallow);

        // Since profile is Windows layout, Right Option is mapped to RWIN
        assert_eq!(
            table.lookup(VK_RMENU, false),
            RemapAction::RemapKey {
                target_vk: VK_RWIN,
                target_sc: 0x5C,
                extended: true,
            }
        );
    }
}
