# 02 — Requirements

> Status: **APPROVED (M0 spike resolved)**. Priorities use MoSCoW: **M**ust / **S**hould / **C**ould.
> See [hardware-findings.md](hardware-findings.md) for the Milestone 0 hardware spike results.

## Functional requirements

### F1 — Fn key & Fn layer (M)
- F1.1 Because Windows' inbox `kbdhid.sys` driver locks the A1843 keyboard collection exclusively and strips the vendor `0x00FF:0x0003` (`Fn`) and `0x000C:0x00B8` (`Eject`) usages in kernel mode (see [hardware-findings.md](hardware-findings.md) §3), the Fn layer is triggered by a configurable **Fn key** in user mode (default: **Right Option / `VK_RMENU`**, configurable to any key such as Right Control or `F19`).
- F1.2 Default Fn combos:

  | Combo | Result |
  |---|---|
  | Fn + ← / → | Home / End |
  | Fn + ↑ / ↓ | Page Up / Page Down |
  | Fn + Backspace | Delete (forward) |
  | Fn + Enter | Insert *(default, remappable)* |
  | Fn + F1…F12 | the "other" function of the top row (see F2) |

- F1.3 The user can edit Fn combos in the GUI.
- F1.4 The user can pick which key acts as Fn (e.g. Right Option, Right Control, `F19`, or Disabled) in the GUI.

### F2 — Top row: media keys vs F-keys (M)
- F2.1 Two modes: **Media first** (press = media, Fn+press = F-key) and **F-keys first** (the reverse).
- F2.2 The mode can be switched from the tray menu, the GUI, and an optional hotkey.
- F2.3 Media actions: volume up/down/mute, play/pause, previous, next.
- F2.4 Brightness (F1/F2): **Could**. Best effort via DDC/CI for external monitors or WMI for laptops;
  otherwise the key can be remapped.
- F2.5 Mission Control / Launchpad keys (F3/F4) map to Windows equivalents by default
  (Task View / Start), and can be remapped.

### F3 — Modifier layout (M)
- F3.1 One-click preset **"Windows layout"**: Cmd → Alt, Option → Win (left and right independently).
- F3.2 Preset **"Mac layout"**: no swap.
- F3.3 Each modifier can be remapped individually.

### F4 — Missing Windows keys (M)
- F4.1 Defaults: **F13 → Print Screen**, **F14 → Scroll Lock**, **F15 → Pause**. Insert is reachable via F1.2.
  *(Note: physical `Eject` is stripped by `kbdhid.sys` in kernel mode and does not reach user mode; see [hardware-findings.md](hardware-findings.md) §3.)*
- F4.2 All remappable keys (`F13`–`F19`, `Clear`) can be remapped in the GUI.

### F5 — Custom actions for F13–F19, Clear (S)
- F5.1 Any of these keys (`F13`–`F19`, `Clear` / `VK_NUMLOCK`) can be mapped to: another key, a key combo, launching a program/file/URL,
  or a media action.
- F5.2 Custom actions run without blocking the input path (see NFR-P3).

### F6 — Battery (S)
- F6.1 The tray icon tooltip shows the battery % (via Apple HID report `0x90` on `0xFF00:0x0014` and/or the Windows PnP Bluetooth battery property).
- F6.2 A Windows notification appears once when battery < threshold (default 15%).
- F6.3 Battery is polled infrequently (default every 10 min, configurable); no busy polling.

### F7 — Settings GUI (M)
- F7.1 Separate small window, opened from the tray. See [04-ux.md](04-ux.md).
- F7.2 **Visual keyboard** (A1843 layout) where clicking a key shows/edits its mapping
  (normal layer and Fn layer).
- F7.3 **UI language** selector: English and Italian at launch, with an extensible translation file format.
- F7.4 **Physical & language layout** selector: supports multiple language layouts (`ANSI (US)`, `ISO (Italian)`, `ISO (UK)`, `ISO (German)`, `ISO (French)`, plus extensible/custom layout labels) so the visual keyboard matches the active layout.
- F7.5 Changes apply **instantly** without restarting the background agent.
- F7.6 Import/export the configuration (or individual profiles) as a file.
- F7.7 **Named profiles (M):** Users can create, rename, duplicate, and delete multiple **profiles** (e.g. per language layout, work vs. gaming, or custom keymaps). Each profile holds its language/physical layout, modifier preset, top-row mode, Fn key choice, and normal/Fn key mappings. Switching profiles is instant from the GUI, tray menu, or an optional hotkey.

### F8 — System integration (M)
- F8.1 Autostart with Windows (per-user, no admin), toggleable.
- F8.2 Tray icon menu: Enable/Disable all remapping, Profile submenu, Media/F-keys mode, Open settings, Quit.
- F8.3 Only one instance runs at a time.
- F8.4 Keeps working after sleep/resume, Bluetooth disconnect/reconnect, and USB ↔ Bluetooth switching.

### F9 — Game safety (S)
- F9.1 A global **pause hotkey** and tray toggle turn all remapping on/off instantly.
- F9.2 *(Could)* **Game mode:** remapping pauses automatically while a fullscreen exclusive app or a
  user-listed process is in the foreground.

## Non-functional requirements

### Performance (hard budget, verified in Milestone 6)
| ID | Metric | Budget |
|---|---|---|
| NFR-P1 | Background agent RAM (working set) | **≤ 10 MB**, target ≤ 5 MB |
| NFR-P2 | Background agent CPU when idle | **0%**: event-driven, no polling loops (except battery F6.3) |
| NFR-P3 | Added latency per keystroke in the hook | **< 0.1 ms** typical; no allocation, I/O or locks in the hook |
| NFR-P4 | Agent binary size | ≤ 3 MB |
| NFR-P5 | Settings GUI | Runs **only while open**; zero footprint when closed |

### Safety & compatibility
- NFR-S1 No DLL injection, no kernel drivers, no admin rights needed for normal use.
- NFR-S2 Mappings are active while an Apple keyboard (`VID 0x05AC`) is connected and automatically pause when none is connected
  (see [03-architecture.md](03-architecture.md) §Device filtering and [hardware-findings.md](hardware-findings.md) §6).
- NFR-S3 If the agent crashes or is killed, the keyboard falls back to normal Windows behaviour.
  No stuck keys, no leftover system changes.
- NFR-S4 No network access. No telemetry.

### Quality
- NFR-Q1 The mapping engine is a pure, unit-tested module (no Win32 calls).
- NFR-Q2 Logging is off by default; a debug log can be turned on from the GUI.
