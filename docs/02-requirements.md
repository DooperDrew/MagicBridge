# 02 — Requirements

> Status: **DRAFT**. Priorities use MoSCoW: **M**ust / **S**hould / **C**ould.
> Requirements marked 🔬 depend on the outcome of the hardware spike (Milestone 0).

## Functional requirements

### F1 — Fn key & Fn layer 🔬 (M)
- F1.1 The physical Fn key is detected on both USB and Bluetooth.
- F1.2 Default Fn combos:

  | Combo | Result |
  |---|---|
  | Fn + ← / → | Home / End |
  | Fn + ↑ / ↓ | Page Up / Page Down |
  | Fn + Backspace | Delete (forward) |
  | Fn + Enter | Insert *(default, remappable)* |
  | Fn + F1…F12 | the "other" function of the top row (see F2) |

- F1.3 The user can edit Fn combos in the GUI.
- F1.4 **Fallback:** if the hardware spike shows Fn can't be read reliably, the user can
  pick another key to act as Fn, e.g. Right Option.

### F2 — Top row: media keys vs F-keys 🔬 (M)
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
- F4.1 Defaults: **F13 → Print Screen**, **F14 → Scroll Lock**, **F15 → Pause**, **Eject → configurable**
  (default: none). Insert is reachable via F1.2.
- F4.2 All of these can be remapped in the GUI.

### F5 — Custom actions for F13–F19, Clear, Eject (S)
- F5.1 Any of these keys can be mapped to: another key, a key combo, launching a program/file/URL,
  or a media action.
- F5.2 Custom actions run without blocking the input path (see NFR-P3).

### F6 — Battery 🔬 (S)
- F6.1 The tray icon tooltip shows the battery % when connected over Bluetooth.
- F6.2 A Windows notification appears once when battery < threshold (default 15%).
- F6.3 Battery is polled infrequently (default every 10 min, configurable); no busy polling.

### F7 — Settings GUI (M)
- F7.1 Separate small window, opened from the tray. See [04-ux.md](04-ux.md).
- F7.2 **Visual keyboard** (A1843 layout) where clicking a key shows/edits its mapping
  (normal layer and Fn layer).
- F7.3 **UI language** selector: English and Italian at launch, with an extensible translation file format.
- F7.4 **Physical layout** selector: ANSI (US) / ISO (e.g. Italian), so the visual keyboard matches the real one.
- F7.5 Changes apply **instantly** without restarting the background agent.
- F7.6 Import/export the configuration as a file.

### F8 — System integration (M)
- F8.1 Autostart with Windows (per-user, no admin), toggleable.
- F8.2 Tray icon menu: Enable/Disable all remapping, Media/F-keys mode, Open settings, Quit.
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
- NFR-S2 Only keys from the Apple keyboard are changed, where technically possible 🔬
  (see [03-architecture.md](03-architecture.md) §Device filtering).
- NFR-S3 If the agent crashes or is killed, the keyboard falls back to normal Windows behaviour.
  No stuck keys, no leftover system changes.
- NFR-S4 No network access. No telemetry.

### Quality
- NFR-Q1 The mapping engine is a pure, unit-tested module (no Win32 calls).
- NFR-Q2 Logging is off by default; a debug log can be turned on from the GUI.
