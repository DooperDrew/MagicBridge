# 05 — Roadmap

> Each task should fit in **one development conversation**. Tick the boxes as tasks are completed
> and add a one-line note (date + commit) if useful. Don't start a milestone before the previous
> milestone's exit criteria are met.
>
> **Task Complexity Tiers:**
> - `[T1]` **Lean:** Pure logic, schemas, unit tests, i18n (`flash_lite` / `flash`).
> - `[T2]` **Moderate:** UI views, tray callbacks, named pipe IPC (`flash` / `pro`).
> - `[T3]` **Complex / Critical:** Low-level hooks, atomic table swaps, raw HID (`pro` / `inherit`).

## M0 — Hardware spike 🔬 (decision gate)
Goal: learn exactly what the A1843 sends on Windows, over USB and Bluetooth.
- [x] M0.1 Set up the Cargo workspace skeleton (`core`, `agent`, `settings`, `hid-probe`), CI-free local build, release profile. _(2026-10-25 — 4935fbf)_
- [x] M0.2 `hid-probe`: list all HID devices with VID `0x05AC` (PID, usage page/usage, report descriptors, transport). _(2026-10-25 — fb1cc69)_
- [x] M0.3 `hid-probe`: live-dump Raw Input and HID reports with timestamps; also log `WH_KEYBOARD_LL` events (vk, scancode, flags) side by side. _(2026-10-25 — fb1cc69)_
- [x] M0.4 Owner runs the probe on **USB** and **Bluetooth** and presses: Fn, Fn+each key, every top-row key, Eject, F13–F19, Clear, all modifiers. _(2026-10-25 — verified on both USB `05AC:026C` and Bluetooth `004C:026C`)_
- [x] M0.5 Investigate battery reporting (HID feature report / BT device property). _(2026-10-25 — HID Input Report `0x90` on `0xFF00:0x0014` returns `[0x90, status, pct]` on both USB `90 01 64` and Bluetooth `90 00 64`)_
- [x] M0.6 Investigate matching Raw Input device handles with hook events (device filtering). _(2026-10-25 — `WH_KEYBOARD_LL` fires 0.43–1.76 ms before `WM_INPUT`; presence-based device filtering selected)_
- [x] M0.7 Write `docs/hardware-findings.md` and update requirements/architecture where 🔬 items are resolved. _(2026-10-25 — ae17257, ac95648)_

**Exit:** Fn detection approach decided (native or fallback), battery approach decided, device-filtering approach decided.

## M1 — Agent skeleton
- [x] `[T2]` M1.1 Tray icon + menu (Enable, Profile submenu, Open settings, Quit), single-instance mutex, hidden message window. _(2026-10-25 — M1.1 completed)_
- [x] `[T1]` M1.2 Config schema in `core` (with named profiles & multiple language layouts) + TOML load/save with defaults; config path in `%APPDATA%`. _(2026-10-25 — M1.2 completed)_
- [x] `[T2]` M1.3 Autostart toggle (HKCU Run key). _(2026-10-25 — M1.3 completed)_
- [x] `[T2]` M1.4 Device arrival/removal and sleep/resume handling. _(2026-10-25 — M1.4 completed)_
**Exit:** agent idles at 0% CPU and ≤ 5 MB RAM with tray working. — **MILESTONE 1 COMPLETE ✅**

## M2 — Remapping engine
- [x] `[T1]` M2.1 `core` mapping engine: key model, layers, profile compilation to precomputed lookup table, unit tests. _(2026-10-25 — M2.1 completed)_
- [ ] `[T3]` M2.2 `WH_KEYBOARD_LL` hook + `SendInput` output with self-injection marker; pass-through fast path + ISO `AltGr` (`sc=0x21D`) suppression.
- [ ] `[T2]` M2.3 Modifier presets (Windows/Mac layout) and per-key remap.
- [ ] `[T1]` M2.4 Missing-key defaults (F13→PrtSc, F14→ScrLk, F15→Pause).
- [ ] `[T3]` M2.5 Pause hotkey + tray Enable/Profile toggle; atomic config/profile hot-swap.
**Exit:** remaps work in Notepad, browser, and one game; no stuck keys after rapid typing or killing the agent.

## M3 — Fn layer & top row
- [ ] `[T3]` M3.1 Configurable Fn key per `hardware-findings.md` (default Right Option / `VK_RMENU`).
- [ ] `[T2]` M3.2 Fn combos (arrows, Backspace, Enter).
- [ ] `[T2]` M3.3 Top-row Media-first / F-keys-first modes + tray toggle.
- [ ] `[T2]` M3.4 Task View / Start for F3/F4; brightness best effort (Could).

## M4 — Custom actions & battery
- [ ] `[T2]` M4.1 Actions: key combo, launch program/URL, media action. Executed off the hook thread.
- [ ] `[T3]` M4.2 Battery reading (`0xFF00:0x0014` report `0x90` + PnP fallback) + tray tooltip + low-battery notification.

## M5 — Settings GUI
- [ ] `[T2]` M5.1 `settings` app shell (eframe), tabs, named-pipe client, "agent not running" banner.
- [ ] `[T2]` M5.2 Visual A1843 keyboard widget (multiple language layouts: ANSI US, ISO IT/UK/DE/FR + custom labels), Normal/Fn layers, highlight remapped keys.
- [ ] `[T2]` M5.3 Key editor side panel + profile manager (create/duplicate/rename/delete) + presets + "press a key to select".
- [ ] `[T1]` M5.4 Top row / Battery / General / About tabs.
- [ ] `[T1]` M5.5 i18n (English, Italian) + language switch.
- [ ] `[T1]` M5.6 Import/export config and profiles.

## M6 — Hardening & performance verification
- [ ] `[T3]` M6.1 Measure and document NFR-P1…P5 (RAM, idle CPU, hook latency, binary size).
- [ ] `[T3]` M6.2 Soak test: BT reconnect, USB↔BT switching, sleep/resume, long uptime.
- [ ] `[T2]` M6.3 Game mode (Could): auto-pause for fullscreen / listed processes.
- [ ] `[T2]` M6.4 Debug logging toggle; crash safety review (no stuck keys).
- [ ] `[T3]` M6.5 Retrospective review of any model-fallback log items recorded in `docs/model-fallback-log.md`.

## M7 — Packaging
- [ ] `[T1]` M7.1 Release build script producing a portable zip (both exes + default config + translations).
- [ ] `[T1]` M7.2 Optional: simple installer, and an uninstall that removes the autostart entry.
- [ ] `[T1]` M7.3 User README: install, features, known limitations (elevated windows, anti-cheat note).
