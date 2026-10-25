# 05 — Roadmap

> Each task should fit in **one development conversation**. Tick the boxes as tasks are completed
> and add a one-line note (date + commit) if useful. Don't start a milestone before the previous
> milestone's exit criteria are met.

## M0 — Hardware spike 🔬 (decision gate)
Goal: learn exactly what the A1843 sends on Windows, over USB and Bluetooth.
- [x] M0.1 Set up the Cargo workspace skeleton (`core`, `agent`, `settings`, `hid-probe`), CI-free local build, release profile. _(2026-10-25)_
- [ ] M0.2 `hid-probe`: list all HID devices with VID `0x05AC` (PID, usage page/usage, report descriptors, transport).
- [ ] M0.3 `hid-probe`: live-dump Raw Input and HID reports with timestamps; also log `WH_KEYBOARD_LL` events (vk, scancode, flags) side by side.
- [ ] M0.4 Owner runs the probe on **USB** and **Bluetooth** and presses: Fn, Fn+each key, every top-row key, Eject, F13–F19, Clear, all modifiers.
- [ ] M0.5 Investigate battery reporting (HID feature report / BT device property).
- [ ] M0.6 Investigate matching Raw Input device handles with hook events (device filtering).
- [ ] M0.7 Write `docs/hardware-findings.md` and update requirements/architecture where 🔬 items are resolved.

**Exit:** Fn detection approach decided (native or fallback), battery approach decided, device-filtering approach decided.

## M1 — Agent skeleton
- [ ] M1.1 Tray icon + menu (Enable, Open settings, Quit), single-instance mutex, hidden message window.
- [ ] M1.2 Config schema in `core` + TOML load/save with defaults; config path in `%APPDATA%`.
- [ ] M1.3 Autostart toggle (HKCU Run key).
- [ ] M1.4 Device arrival/removal and sleep/resume handling.
**Exit:** agent idles at 0% CPU and ≤ 5 MB RAM with tray working.

## M2 — Remapping engine
- [ ] M2.1 `core` mapping engine: key model, layers, precomputed lookup table, unit tests.
- [ ] M2.2 `WH_KEYBOARD_LL` hook + `SendInput` output with self-injection marker; pass-through fast path.
- [ ] M2.3 Modifier presets (Windows/Mac layout) and per-key remap.
- [ ] M2.4 Missing-key defaults (F13→PrtSc, F14→ScrLk, F15→Pause).
- [ ] M2.5 Pause hotkey + tray Enable toggle; atomic config hot-swap.
**Exit:** remaps work in Notepad, browser, and one game; no stuck keys after rapid typing or killing the agent.

## M3 — Fn layer & top row
- [ ] M3.1 Fn detection per `hardware-findings.md` (or fallback key).
- [ ] M3.2 Fn combos (arrows, Backspace, Enter).
- [ ] M3.3 Top-row Media-first / F-keys-first modes + tray toggle.
- [ ] M3.4 Task View / Start for F3/F4; brightness best effort (Could).

## M4 — Custom actions & battery
- [ ] M4.1 Actions: key combo, launch program/URL, media action. Executed off the hook thread.
- [ ] M4.2 Battery reading + tray tooltip + low-battery notification.

## M5 — Settings GUI
- [ ] M5.1 `settings` app shell (eframe), tabs, named-pipe client, "agent not running" banner.
- [ ] M5.2 Visual A1843 keyboard widget (ANSI + ISO), Normal/Fn layers, highlight remapped keys.
- [ ] M5.3 Key editor side panel + presets + "press a key to select".
- [ ] M5.4 Top row / Battery / General / About tabs.
- [ ] M5.5 i18n (English, Italian) + language switch.
- [ ] M5.6 Import/export config.

## M6 — Hardening & performance verification
- [ ] M6.1 Measure and document NFR-P1…P5 (RAM, idle CPU, hook latency, binary size).
- [ ] M6.2 Soak test: BT reconnect, USB↔BT switching, sleep/resume, long uptime.
- [ ] M6.3 Game mode (Could): auto-pause for fullscreen / listed processes.
- [ ] M6.4 Debug logging toggle; crash safety review (no stuck keys).

## M7 — Packaging
- [ ] M7.1 Release build script producing a portable zip (both exes + default config + translations).
- [ ] M7.2 Optional: simple installer, and an uninstall that removes the autostart entry.
- [ ] M7.3 User README: install, features, known limitations (elevated windows, anti-cheat note).
