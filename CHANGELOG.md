# Development & Knowledge Log (`CHANGELOG.md`)

All development steps, hardware discoveries, architectural decisions, and commits are logged here in chronological order for future reference.

---

## 2026-10-25 — Milestone 1: Agent Skeleton (In Progress)

### M1.1 — Daemon Shell, Single-Instance Mutex, Hidden Window & Tray Icon
- Implemented background daemon runtime shell in `crates/agent`:
  - `single_instance`: Named Win32 Mutex `Local\MagicBridge_SingleInstance_Mutex` to ensure single instance (F8.3), exiting cleanly with code 0 on duplicate.
  - `window`: Hidden message window (`MagicBridge_MsgWndClass`) with clean Win32 message pump (`GetMessageW` / `TranslateMessage` / `DispatchMessageW`). Handles `WM_CREATE`, `WM_COMMAND`, `WM_TRAY_CALLBACK`, and `TaskbarCreated` broadcast message to restore tray icon if Windows Explorer restarts.
  - `tray`: System tray icon (`Shell_NotifyIconW`) with tooltip showing keyboard connection status, battery, active profile, and top-row mode.
  - Context menu: Enabled/Paused toggle, Profile submenu, Top Row mode toggle, Open Settings... (launches `magicbridge-settings.exe`), Start with Windows toggle, and Quit.
  - Subagent: `T2-Agent-M1.1` running `flash`.

### M1.3 — Windows Autostart Toggle (HKCU Run Key)
- Implemented per-user autostart manager in `crates/agent/src/autostart.rs`:
  - Interacts with `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` value `"MagicBridge"` (`REG_SZ`).
  - Pure user-space, zero administrator or UAC elevation required (NFR-S1).
  - Handles command line path quoting, RAII `RegCloseKey` handles, and idempotent toggling (`enable_autostart`, `disable_autostart`, `is_autostart_enabled`).
  - Includes 8 tests verifying quoting, spacing, and isolated registry CRUD operations.
  - Subagent: `T1-Autostart-M1.3` running `flash`.

### M1.2 — Config Schema in `crates/core`
- Implemented complete configuration schema, named profiles, multi-language layout models, and TOML persistence in `magicbridge_core::config`:
  - `Config`: global settings (`active_profile`, `autostart`, `battery_poll_minutes`, `low_battery_threshold_percent`, `ui_language`, `pause_hotkey`, `debug_logging`, and `profiles`).
  - `NamedProfile`: encapsulates `name`, `layout: KeyboardLayout`, `modifier_preset: ModifierPreset`, `top_row_mode: TopRowMode`, `fn_key: FnKeyChoice`, `mappings: BTreeMap<String, KeyAction>`, and `fn_mappings: BTreeMap<String, KeyAction>`.
  - Enums: `KeyboardLayout` (`IsoItalian`, `AnsiUs`, `IsoUk`, `IsoGerman`, `IsoFrench`, `Custom`), `ModifierPreset` (`WindowsLayout`, `MacLayout`, `Custom`), `TopRowMode` (`MediaKeysFirst`, `FKeysFirst`), and `FnKeyChoice` (`RightOption`, `RightControl`, `F19`, `Disabled`, `Other`).
  - Sensible out-of-the-box defaults: generates a "Default (IT)" profile matching an Italian ISO keyboard with Windows layout modifiers (`Cmd`->`Alt`, `Option`->`Win`), Right Option as Fn trigger, Media keys first, default missing keys (`F13`->`PrintScreen`, `F14`->`ScrollLock`, `F15`->`Pause`), and standard Fn combos (`Left/Right`->`Home/End`, `Up/Down`->`PageUp/PageDown`, `Backspace`->`Delete`, `Enter`->`Insert`).
  - Safe atomic file persistence via temporary file creation, data flushing (`sync_all()`), and atomic replacement.
  - Resolved `%APPDATA%\MagicBridge\config.toml` path helpers.
- Added 10 comprehensive unit tests covering serialization, round-trip parsing, custom profiles, duplication, deletion safety guards, and atomic writes.
- Subagent: `T1-Core-M1` running token-efficient `flash` model. All local verification gates passed.

---

## 2026-10-25 — Milestone 0: Hardware Spike 🔬 (Completed)

### M0.1 — Cargo Workspace Skeleton (`4935fbf`)
- Created Cargo workspace (`resolver = "3"`, Rust 2024 edition) with 4 crates:
  - `crates/core` (`magicbridge-core`): pure business logic, `#![forbid(unsafe_code)]`, zero Win32 dependencies.
  - `crates/agent` (`magicbridge-agent`): background tray/hook/HID agent executable.
  - `crates/settings` (`magicbridge-settings`): `egui`/`eframe` settings GUI executable.
  - `crates/hid-probe` (`hid-probe`): diagnostic CLI for Milestone 0 hardware probing.
- Configured size/latency-tuned release profile (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`) and workspace lints (`unsafe_op_in_unsafe_fn = "deny"`, `clippy::undocumented_unsafe_blocks = "deny"`).

### M0.2 & M0.3 — `hid-probe` Implementation (`fb1cc69`)
- Added `magicbridge_core::device::parse_hid_path` to extract transport (`USB`, `Bluetooth`, `BluetoothLe`), `VID`, `PID`, USB interface (`MI_xx`), and top-level collection (`Colxx`) from Windows HID paths, with unit tests.
- Implemented `hid-probe list`, `hid-probe dump` (simultaneous `WH_KEYBOARD_LL`, `WM_INPUT` `RIM_TYPEKEYBOARD` / `RIM_TYPEHID`, and direct `ReadFile` threads with microsecond QPC timestamps and Hook↔Raw Input pairing), and `hid-probe battery`.

### M0.4, M0.5, M0.6, M0.7 — Physical A1843 Hardware Findings (`ae17257`, `ac95648`)
- **Dual Vendor IDs (`0x05AC` on USB, `0x004C` on Bluetooth):**
  - Over USB, the A1843 reports `VID = 0x05AC` (USB-IF Apple Inc.), `PID = 0x026C`, rev `0x0852`.
  - Over Bluetooth Classic (`{00001124-0000-1000-8000-00805f9b34fb}`), the A1843 reports **`VID = 0x004C`** (Bluetooth SIG Apple Inc., `VID&0001004c`), `PID = 0x026C`.
  - Updated `magicbridge_core::is_apple_vendor` to match both `0x05AC` and `0x004C`.
- **Why Physical `Fn` (`0x00FF:0x0003`) and `Eject` (`0x000C:0x00B8`) Are Blocked in User Mode:**
  - Both usages are declared inside Input Report `0x01` (10B) of the main `0x0001:0x0006` (Generic Desktop / Keyboard) collection (`MI_01&Col01` on USB, `Col01` on Bluetooth).
  - Windows `kbdhid.sys` opens `0x0001:0x0006` collections exclusively (`ERROR_ACCESS_DENIED`) and strips non-`0x07` usages in kernel mode before `WH_KEYBOARD_LL` or `WM_INPUT`.
  - **Decision:** Use the configurable Fn key approach (`F1.1` / `F1.4`, default Right Option / `VK_RMENU`) in user mode rather than a kernel filter driver (`NFR-S1`).
- **ISO / Italian `AltGr` Quirk (`scanCode = 0x21D`):**
  - Pressing Right Option (`VK_RMENU`, `vk=0xA5`, `sc=0x38`, `ext=1`) on an Italian/ISO layout causes Windows to synthesize a fake `VK_LCONTROL` (`vk=0xA2`, `sc=0x21D`, `ext=0`) ~0.8 ms earlier. The M2 hook must swallow `sc=0x21D` whenever `VK_RMENU` is remapped.
- **`Clear` Key:**
  - Sends `VK_NUMLOCK` (`vk=0x90`, `sc=0x45`, `ext=1`).
- **Battery Reporting (`0xFF00:0x0014`, Report `0x90`):**
  - Works identically on both USB (`MI_00&Col02`) and Bluetooth (`Col02`) via `HidD_GetInputReport(0x90)` (3 bytes: `[0x90, status_flags, percentage]`). Observed `90 01 64` (100%, charging) on USB and `90 00 64` (100%, on battery) on Bluetooth.
- **Device Filtering Timing:**
  - `WH_KEYBOARD_LL` always fires **0.28–2.41 ms before** `WM_INPUT` (`RAWKB`). Per-keystroke device filtering inside `WH_KEYBOARD_LL` is therefore impossible without blocking; presence-based activation (active while an Apple `0x05AC`/`0x004C` keyboard is connected) is used.

### Scope & Workflow Alignment — Profiles, Multi-Language Layouts & GitHub Tracking
- Added requirement **F7.7** for **named profiles** and **multiple language/physical keyboard layouts** (`ISO Italian`, `ANSI US`, `ISO UK`, `ISO German`, `ISO French`, plus custom labels), switchable from the tray menu, GUI, or hotkey.
- Made the GitHub repository (`DooperDrew/test-repo-antigravity`) public, added `CHANGELOG.md`, and created GitHub Issues for all milestones (`M0`–`M7`).
