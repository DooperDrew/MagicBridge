# Development & Knowledge Log (`CHANGELOG.md`)

All development steps, hardware discoveries, architectural decisions, and commits are logged here in chronological order for future reference.

---

## 2026-10-25 — QA Framework, UX Specifications & GitHub Issue Tracking

### QA Test Suite & UX Backlog Formalization
- **Dedicated QA Specification (`docs/06-qa-and-validation.md`):**
  - Level 1-4 validation architecture spanning automated unit invariants, soak/hardening gates, empirical hardware tests, and security edge cases.
  - Complete 12-point empirical hardware test matrix for physical Apple Magic Keyboard (Model A1843) across USB (`VID 0x05AC`) and Bluetooth Classic (`VID 0x004C`).
  - Validation procedures for the Italian/ISO AltGr quirk (`sc=0x21D`), Modern Standby/sleep recovery, UIPI admin window isolation, and Game Mode auto-pause.
- **Dedicated UX Roadmap (`docs/07-ux-and-improvements.md`):**
  - Specification for Guided Windows Installer Wizard (`magicbridge-setup.exe` / Inno Setup) with zero UAC elevation requirement.
  - Architecture for First-Run Initial Configuration Onboarding Wizard in `magicbridge-settings`: device auto-detect, layout picker, modifier preset switcher, Fn trigger key choice, and autostart toggle.
  - Technical design for Fluid Responsive Layout in `magicbridge-settings` (`egui`): dynamic keyboard scale clamping (`ui.available_width() / 25.5`) and adaptive navigation rail for compact and high-DPI screens.
  - Feature backlog: dynamic battery tray badge, DDC/CI monitor brightness integration, per-app profiles, and GitHub release checker.
- **GitHub Issue Tracking & Milestone Alignment:**
  - Created GitHub Milestones **M9 — Quality Assurance & Empirical Validation** and **M10 — UX & Onboarding Improvements**.
  - Logged tracking issues: #10 (Hardware QA Matrix), #11 (Power States & Edge Cases), #12 (Installer Wizard), #13 (Onboarding Wizard), #14 (Responsive Settings Layout), and #15 (Battery Tray Badge).
  - Integrated direct links to QA Matrix and UX Roadmap in the Settings Architecture Tab and `README.md`.

---

## 2026-10-25 — Milestone 8: Multi-Agent Methodology & Release Parity (Completed ✅)

### M8.1..M8.3 — Didactic Multi-Agent Guide, GUI Architecture Tab & Release Parity
- Created comprehensive didactic documentation [`docs/multi-agent-methodology.md`](docs/multi-agent-methodology.md):
  - In-depth explanation of the Hub-and-Spoke Lead/Worker model, dynamic model selection tiers (T1/T2/T3), Sole Committer protocol, and the 4-step Lead Verification Gate.
  - Complete Mermaid sequence and dataflow architecture diagrams for all 5 parallel execution phases.
  - Subagent taxonomy, reporting contract templates, and instructions on how to extend the workflow for future tasks.
- Added dedicated **"🤖 Architecture"** tab in `magicbridge-settings` (`crates/settings/src/views/architecture_view.rs`):
  - Interactive multi-section view covering Overview, Hub-and-Spoke model, Complexity Tiers, Parallel Phases, and Extending Over Time.
  - Added direct browser links to GitHub documentation and localization keys in English (`en.toml`) and Italian (`it.toml`).
- Release & Distribution Parity (M8.3):
  - Created official **GitHub Release `v0.1.0`** attaching the distribution archive `MagicBridge-v0.1.0-windows-x64.zip` directly to the release page.
  - Re-synced and confirmed 100% parity between local repository, git remotes, and GitHub release assets.

---

## 2026-10-25 — Milestone 7: Packaging & Release Automation (Completed ✅)

### M7.1..M7.3 — Portable ZIP Script, User-Space Installer & User Guide
- Implemented `scripts/package-release.ps1`:
  - Compiles release binaries with optimizations.
  - Assembles staging bundle with `magicbridge-agent.exe`, `magicbridge-settings.exe`, `locales/` (`en.toml`, `it.toml`), documentation, and installer scripts.
  - Generates portable ZIP archive `dist/MagicBridge-v<version>-windows-x64.zip` (~2.1 MB).
- Implemented user-space installer and uninstaller scripts:
  - `scripts/install.ps1`: Deploys to `%LOCALAPPDATA%\Programs\MagicBridge`, creates Start Menu shortcuts, and registers per-user autostart in `HKCU\...\Run` (zero admin rights required).
  - `scripts/uninstall.ps1`: Gracefully terminates running processes, removes shortcuts, and cleans autostart keys.
- Comprehensive End-User Documentation:
  - Created `docs/user-guide.md` covering setup, pairing, Fn navigation, media key controls, named profiles, battery telemetry, and Windows UIPI limitations.
  - Added official `LICENSE` file (MIT).
- Subagent: `T1-Package-M7` (flash).

---

## 2026-10-25 — Milestone 6: Hardening, Performance Verification & Game Mode (Completed ✅)

### M6.1..M6.5 — Game Mode Auto-Pause, Debug Logger, Soak Tests & Latency Budget
- Implemented `crates/agent/src/game_mode.rs`:
  - Event-driven foreground tracker using `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND` (0% idle CPU per NFR-P2).
  - Auto-pauses input hook and releases synthetic keys when a game from `config.game_mode_processes` or fullscreen exclusive application is focused.
  - Restores active hook state upon return to desktop.
- Implemented `crates/agent/src/logger.rs`:
  - Zero-overhead atomic flag check when disabled.
  - Timestamped diagnostic log writing to `%APPDATA%\MagicBridge\debug.log` and in-memory ring buffer (up to 200 entries) when enabled.
- Hardening & Soak Tests (`crates/agent/tests/`):
  - Benchmarked hook latency: average latency measured < 1 µs (< 1,000 ns), outperforming NFR-P3's 100 µs (0.1 ms) budget by >100x.
  - Verified emergency key release and rapid 500-iteration typing/pause toggling with zero stuck keys (`NFR-S3`).
  - Simulated 500 consecutive sleep/wake (`WM_POWERBROADCAST`) and device arrival/removal loops.
- Model Fallback Audit (M6.5):
  - Audited `docs/model-fallback-log.md`: zero fallback occurrences recorded; all tasks were executed using required target tiers.
- Subagent: `T3-Hardening-M6` (pro).

---

## 2026-10-25 — Milestone 5: Settings GUI (Completed ✅)

### M5.1..M5.6 — Standalone `eframe`/`egui` Settings GUI with Visual Keyboard & Profiles
- Implemented `crates/settings` (`magicbridge-settings.exe`):
  - Standalone native binary opening a `~920x540` window; exits completely upon close with zero resident memory (`NFR-P5`).
  - Top banner with one-click "Start Agent" button when `magicbridge-agent.exe` is not running.
  - Named Pipe IPC client connecting to `\\.\pipe\MagicBridge_Agent_IPC` for instant live status queries and atomic table reloading (`F7.5`).
- Visual A1843 Keyboard Widget:
  - Drawn to scale with all 109 keys across 3 distinct clusters (Main, Navigation, and Numpad).
  - Physical & Language layout switching (`ISO Italian`, `ANSI US`, `ISO UK`, `ISO German`, `ISO French`, and custom labels).
  - Normal vs. Fn layer toggling with visual highlighting for remapped keys.
  - Interactive Key Editor side panel and "Press a key to select" mode.
  - Modifier presets dropdown (Windows layout, Mac layout, Reset All).
- Complete Tab Navigation:
  - **Keyboard:** Profile bar (Create, Duplicate, Rename, Delete), visual map, layout selector, and key editor.
  - **Top Row & Fn:** Mode toggle (Media-first vs. F-keys first), toggle hotkey, and Fn fallback key selector.
  - **Battery:** Live gauge/percentage, charging indicator, alert threshold, and poll interval.
  - **General:** UI language selector, autostart toggle (`HKCU\...\Run`), pause hotkey, game mode process list, and configuration import/export.
  - **About:** Diagnostic daemon status, version, open-source license, and GitHub repository link.
- Zero Hardcoded UI Text (i18n):
  - English (`locales/en.toml`) and Italian (`locales/it.toml`) translation dictionaries with dynamic runtime switching.
- Subagent: `T2-UI-M5` (Claude 3.7 Sonnet).

---

## 2026-10-25 — Milestone 4: Custom Actions & Battery (Completed ✅)

### M4.1 & M4.2 — Off-Thread Action Dispatcher & Direct Apple HID Battery Reader
- Implemented `crates/agent/src/action_worker.rs`:
  - Off-thread asynchronous action dispatcher using a bounded channel (`sync_channel(64)`).
  - Guarantees zero blocking on the `WH_KEYBOARD_LL` input hook (`NFR-P3`).
  - Supports `Launch { program, args }`, `OpenUrl { url }` via `ShellExecuteW`, `Shortcut` (multi-key sequences), and `Media` actions.
- Implemented `crates/agent/src/battery.rs`:
  - Direct Apple HID Input Report `0x90` reader on collection `0xFF00:0x0014` via `HidD_GetInputReport`.
  - Works identically on both USB (`MI_00&Col02`) and Bluetooth (`Col02`).
  - Decodes charging/discharging/AC present bitflags and battery percentage (`0..100%`).
  - Fallback support for Bluetooth PnP battery property (`PKEY_BLUETOOTH_BATTERY`).
- Low-frequency polling & toast alerts in `crates/agent/src/window.rs`:
  - Win32 periodic timer (`SetTimer` `IDT_BATTERY_TIMER`) based on `config.battery_poll_minutes` (default 10 min, zero busy loops).
  - Immediate poll on device arrival (`DBT_DEVICEARRIVAL`) and wake from sleep (`WM_POWERBROADCAST`).
  - Windows notification balloon/toast alert when battery drops below `low_battery_threshold_percent` (15%).
- Subagent: `T3-Battery-M4` (inherit/pro).

---

## 2026-10-25 — Milestone 3: Fn Layer & Top Row Modes (Completed ✅)

### M3.1..M3.4 — Top Row Special Keys, Media Switching & Navigation Combos
- Implemented top row shortcuts and mode switching:
  - F3 (Mission Control) -> Windows Task View (`Win + Tab`) via atomic multi-key shortcut synthesis.
  - F4 (Launchpad) -> Windows Start Menu (`VK_LWIN`).
  - F7..F12 -> Media keys (Rewind, Play/Pause, Fast Forward, Mute, Volume Down, Volume Up).
  - `TopRowMode::MediaKeysFirst` vs. `TopRowMode::FKeysFirst` toggles smoothly between direct press and Fn+press.
  - Built-in navigation combos: `Fn + Arrows` -> `Home/End/PgUp/PgDn`, `Fn + Backspace` -> `Forward Delete (⌦)`, `Fn + Enter` -> `Insert`.
- Subagent: `T2-TopRow-M3` (flash/pro).

---

## 2026-10-25 — Milestone 2: Remapping Engine (Completed ✅)

### M2.2 & M2.5 — Low-Level Hook, SendInput Injector, AltGr Suppression & Named Pipe IPC
- Implemented `crates/agent/src/hook.rs`:
  - `WH_KEYBOARD_LL` low-level hook procedure operating with zero heap allocation, zero disk/file I/O, and zero mutex locks (`NFR-P3`).
  - Fast-path for non-remapped keys passing directly to `CallNextHookEx`.
  - Evaluates `dwExtraInfo == MAGICBRIDGE_EXTRA_INFO` (`0x4D424745`) to prevent self-injection loops.
  - Automatically swallows fake Windows `VK_LCONTROL` with `scanCode == 0x21D` when Right Option is remapped or acts as Fn (`hardware-findings §4.1`).
  - Lock-free `[AtomicU32; 256]` active key state tracker; `release_all_active_keys` releases any active synthetic keys on pause/disable/sleep/exit to guarantee zero stuck keys (`NFR-S3`).
- Implemented `crates/agent/src/injector.rs`:
  - Thin `SendInput` wrapper tagging synthetic keystrokes with `MAGICBRIDGE_EXTRA_INFO` sentinel.
- Implemented `crates/agent/src/ipc.rs`:
  - `AtomicMappingTable`: Lock-free table swapping built on `arc_swap::ArcSwap<CompiledMappingTable>`.
  - Named Pipe IPC server listening on `\\.\pipe\MagicBridge_Agent_IPC`.
  - Request/response protocol supporting `Reload`, `SwitchProfile`, `GetStatus`, and `Pause`.
  - Lightweight `IpcClient` with end-to-end integration tests.
- Verified modifier presets (Windows layout Cmd<->Alt, Option<->Win) and missing key defaults (`F13`->`PrintScreen`, `F14`->`ScrollLock`, `F15`->`Pause`) across the hook pipeline (M2.3, M2.4).
- Subagents: `T3-Hook-M2.2` (inherit/pro) and `T2-IPC-M2.5` (flash/sonnet) running in parallel.

### M2.1 — Pure Remapping Engine in `crates/core`
- Implemented core key models, layer definitions, and zero-allocation precomputed lookup table in `crates/core`:
  - `key`: Windows Virtual Key constants (`VK_LWIN`, `VK_RMENU`, `VK_SNAPSHOT`, `VK_SCROLL`, `VK_PAUSE`, etc.), default scancodes, and `MAGICBRIDGE_EXTRA_INFO: usize = 0x4D424745` ("MBGE") self-injection prevention sentinel.
  - `engine`: `CompiledMappingTable` with flat 256-element arrays for normal and Fn layers, supporting `PassThrough`, `RemapKey`, `CustomAction`, and `Swallow`.
  - O(1) table lookup with zero heap allocations, zero locks, and zero I/O (`NFR-P3`).
  - `compile_profile`: Compiles `NamedProfile` applying modifier presets (Windows layout 4-key swap vs. Mac layout), default missing keys (`F13`->`PrintScreen`, `F14`->`ScrollLock`, `F15`->`Pause`), top-row mode, and Fn combos (`Home`/`End`/`PgUp`/`PgDn`/`Del`/`Ins`).
  - Automatic ISO Italian `AltGr` synthetic `LCtrl` (`sc=0x21D`) suppression whenever Right Option (`VK_RMENU`) is remapped or acts as the Fn key.
  - Subagent: `T1-Engine-M2.1` running `flash`.

---

## 2026-10-25 — Milestone 1: Agent Skeleton (Completed ✅)

### M1.4 — Device Arrival/Removal & Power Management
- Implemented Windows hardware event and power management lifecycle in `crates/agent`:
  - `device_notify`: RAII `DeviceNotificationHandle` registering `RegisterDeviceNotificationW` for `GUID_DEVINTERFACE_HID`.
  - Filtered arrival (`DBT_DEVICEARRIVAL`) and removal (`DBT_DEVICEREMOVECOMPLETE`) notifications using `magicbridge_core::is_apple_vendor` (`0x05AC` on USB and `0x004C` over Bluetooth Classic).
  - Tracked connected Apple keyboards with dynamic tray tooltip updates.
  - Handled `WM_POWERBROADCAST` (`PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC`, `PBT_APMRESUMESUSPEND`) to re-probe connected keyboards after system sleep/resume.
  - Subagent: `T2-Device-M1.4` running `flash`.

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
