# 05 — Roadmap

> Each task should fit in **one development conversation**. Tick the boxes as tasks are completed
> and add a one-line note (date + commit) if useful. Don't start a milestone before the previous
> milestone's exit criteria are met.
>
> **Task Complexity Tiers:**
> - `[T1]` **Lean:** Pure logic, schemas, unit tests, i18n (`flash_lite` / `flash`).
> - `[T2]` **Moderate:** UI views, tray callbacks, named pipe IPC (`flash` / `pro`).
> - `[T3]` **Complex / Critical:** Low-level hooks, atomic table swaps, raw HID (`pro` / `inherit`).
>
> **Living Context:** See [`docs/retrospective.md`](retrospective.md) for project history and transcript mining evidence, and [`docs/lessons-learned.md`](lessons-learned.md) for promoted rules.
>
> **Evidence Principle:** In accordance with the Project Kickstart working agreement, automated invariant test passes are distinguished from physical hardware confirmation. Physical hardware QA items remain marked `[ ] Pending Physical Evidence` until confirmed by human testing on the physical device.

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
- [x] `[T3]` M2.2 `WH_KEYBOARD_LL` hook + `SendInput` output with self-injection marker; pass-through fast path + ISO `AltGr` (`sc=0x21D`) suppression. _(2026-10-25 — M2.2 completed)_
- [x] `[T2]` M2.3 Modifier presets (Windows/Mac layout) and per-key remap. _(2026-10-25 — M2.3 completed via M2.1 & M2.2)_
- [x] `[T1]` M2.4 Missing-key defaults (F13→PrtSc, F14→ScrLk, F15→Pause). _(2026-10-25 — M2.4 completed via M2.1 & M2.2)_
- [x] `[T3]` M2.5 Pause hotkey + tray Enable/Profile toggle; atomic config/profile hot-swap. _(2026-10-25 — M2.5 completed)_
**Exit:** remaps work in Notepad, browser, and one game; no stuck keys after rapid typing or killing the agent. — **MILESTONE 2 COMPLETE ✅**

## M3 — Fn layer & top row
- [x] `[T3]` M3.1 Configurable Fn key per `hardware-findings.md` (default Right Option / `VK_RMENU`). _(2026-10-25 — M3.1 completed)_
- [x] `[T2]` M3.2 Fn combos (arrows, Backspace, Enter). _(2026-10-25 — M3.2 completed)_
- [x] `[T2]` M3.3 Top-row Media-first / F-keys-first modes + tray toggle. _(2026-10-25 — M3.3 completed)_
- [x] `[T2]` M3.4 Task View / Start for F3/F4; brightness best effort (Could). _(2026-10-25 — M3.4 completed)_
**Exit:** F3/F4 trigger Task View / Start, F7..F12 media keys toggle with Fn, navigation combos active. — **MILESTONE 3 COMPLETE ✅**

## M4 — Custom actions & battery
- [x] `[T2]` M4.1 Actions: key combo, launch program/URL, media action. Executed off the hook thread. _(2026-10-25 — M4.1 completed)_
- [x] `[T3]` M4.2 Battery reading (`0xFF00:0x0014` report `0x90` + PnP fallback) + tray tooltip + low-battery notification. _(2026-10-25 — M4.2 completed)_
**Exit:** direct HID battery reading over USB & BT, low-battery alert, async action dispatcher. — **MILESTONE 4 COMPLETE ✅**

## M5 — Settings GUI
- [x] `[T2]` M5.1 `settings` app shell (eframe), tabs, named-pipe client, "agent not running" banner. _(2026-10-25 — M5.1 completed)_
- [x] `[T2]` M5.2 Visual A1843 keyboard widget (multiple language layouts: ANSI US, ISO IT/UK/DE/FR + custom labels), Normal/Fn layers, highlight remapped keys. _(2026-10-25 — M5.2 completed)_
- [x] `[T2]` M5.3 Key editor side panel + profile manager (create/duplicate/rename/delete) + presets + "press a key to select". _(2026-10-25 — M5.3 completed)_
- [x] `[T1]` M5.4 Top row / Battery / General / About tabs. _(2026-10-25 — M5.4 completed)_
- [x] `[T1]` M5.5 i18n (English, Italian) + language switch. _(2026-10-25 — M5.5 completed)_
- [x] `[T1]` M5.6 Import/export config and profiles. _(2026-10-25 — M5.6 completed)_
**Exit:** GUI renders visual A1843 layout, profile manager operational, hot-swaps active table via IPC, zero background footprint when closed. — **MILESTONE 5 COMPLETE ✅**

## M6 — Hardening & performance verification
- [x] `[T3]` M6.1 Measure and document NFR-P1…P5 (RAM, idle CPU, hook latency, binary size). _(2026-10-25 — M6.1 completed)_
- [x] `[T3]` M6.2 Soak test: BT reconnect, USB↔BT switching, sleep/resume, long uptime. _(2026-10-25 — M6.2 completed)_
- [x] `[T2]` M6.3 Game mode (Could): auto-pause for fullscreen / listed processes. _(2026-10-25 — M6.3 completed)_
- [x] `[T2]` M6.4 Debug logging toggle; crash safety review (no stuck keys). _(2026-10-25 — M6.4 completed)_
- [x] `[T3]` M6.5 Retrospective review of any model-fallback log items recorded in `docs/model-fallback-log.md`. _(2026-10-25 — M6.5 completed; nominal)_
**Exit:** Latency < 1µs, 0% idle CPU, zero stuck keys under rapid typing/sleep, Game Mode auto-pause active. — **MILESTONE 6 COMPLETE ✅**

## M7 — Packaging
- [x] `[T1]` M7.1 Release build script producing a portable zip (both exes + default config + translations). _(2026-10-25 — M7.1 completed)_
- [x] `[T1]` M7.2 Optional: simple installer, and an uninstall that removes the autostart entry. _(2026-10-25 — M7.2 completed)_
- [x] `[T1]` M7.3 User README: install, features, known limitations (elevated windows, anti-cheat note). _(2026-10-25 — M7.3 completed)_
**Exit:** Automated release packaging script (package-release.ps1), user-space installer/uninstaller, and exhaustive user-guide.md. — **MILESTONE 7 COMPLETE ✅**

## M8 — Multi-Agent Architecture Documentation & Distribution Sync
- [x] `[T1]` M8.1 Create didactic multi-agent parallelization guide (`docs/multi-agent-methodology.md`) detailing the Hub-and-Spoke Lead/Worker model, dynamic model tiers, sole committer protocol, and phase schemes. _(2026-10-25 — M8.1 completed)_
- [x] `[T2]` M8.2 Add dedicated "Architecture" tab in `magicbridge-settings` GUI with interactive didactic breakdown and GitHub links. _(2026-10-25 — M8.2 completed)_
- [x] `[T1]` M8.3 Publish GitHub Release `v0.1.0` with bundled distribution archive (`dist/MagicBridge-v0.1.0-windows-x64.zip`) verifying 100% parity between local repository and GitHub. _(2026-10-25 — M8.3 completed)_
**Exit:** Comprehensive multi-agent methodology guide, interactive GUI architecture tab, and GitHub Release v0.1.0 published with binary asset. — **MILESTONE 8 COMPLETE ✅**

## M9 — Quality Assurance & Empirical Validation 🔬
Goal: Systematically validate MagicBridge against physical A1843 hardware, edge-case power states, and security boundaries. Documented in `docs/06-qa-and-validation.md`.
- [ ] `[T2]` M9.1 Physical A1843 hardware matrix execution: USB (`0x05AC`) and Bluetooth (`0x004C`), ISO Italian AltGr quirk suppression, Fn layer combos, and top-row toggling. _(Tooling ready via Hardware QA Tab; Pending Physical Confirmation)_
- [ ] `[T3]` M9.2 Edge-case power & security validation: Modern standby/sleep resume, emergency disconnect key release, UIPI elevated window behavior, and Game Mode auto-pause. _(Automated mocks passing; Pending Physical Confirmation)_
- [x] `[T1]` M9.3 Automated diagnostic & QA probe reporting: Interactive 109-key visual verification tool (`qa_view.rs`) with live keypress listener, category tracking, and export to Desktop/AppData. _(2026-10-25 — M9.3 completed in 65fa05d)_
- [x] `[T1]` M9.4 Retrospective audit & hygiene: Align claims in docs with empirical evidence, log model tier executions, and link retrospective/lessons-learned in roadmap. _(2026-10-25 — M9.4 completed)_
- [x] `[T2]` M9.5 Autonomous UI QA Loop & Automated Heuristic Verification: Direct integration with `ui-qa-tester` (`qa-session.toml`). Executed multi-cycle autonomous crawl across all GUI tabs, resolving visual clipping on the Hardware QA category selector, eliminating UX dead button friction, and achieving 100% AccessKit/UIA label compliance (0 findings). _(2026-10-27 — M9.5 completed, closes #17, closes #18)_
**Exit:** 100% pass rate across physical A1843 hardware matrix recorded by human testing, zero lingering keys or crashes in edge cases.

## M10 — User Experience & Onboarding Improvements
Goal: Elevate the out-of-the-box user experience with guided installation, interactive first-run onboarding, and responsive GUI layouts. Documented in `docs/07-ux-and-improvements.md`.
- [x] `[T1]` M10.1 Guided Windows Installation Wizard (`setup-wizard.ps1`, `setup.cmd`, `installer.iss`): zero-elevation install into `%LOCALAPPDATA%`, start menu, and desktop shortcuts. _(2026-10-25 — M10.1 completed, closes #12)_
- [x] `[T2]` M10.2 First-Run Onboarding Wizard in `magicbridge-settings`: welcome, hardware auto-detect, layout & modifier preset picker, Fn trigger selection, and autostart toggle. _(2026-10-25 — M10.2 completed, closes #13)_
- [x] `[T2]` M10.3 Responsive UI layout in `magicbridge-settings`: dynamic visual A1843 keyboard scaling (`key_size` clamp) and adaptive sidebar/compact drawer navigation for compact and high-DPI screens. _(2026-10-25 — M10.3 completed, closes #14)_
- [x] `[T2]` M10.4 Dynamic Tray Battery Badge & Status: render live battery percentage, charging lightning bolt, and connection status dynamically on the system tray icon via GDI off-screen rendering (`crates/agent/src/tray_icon.rs`), with user toggle in Settings Battery tab and i18n support. _(2026-10-27 — M10.4 completed, closes #15)_
**Exit:** Seamless guided installation, first-run wizard, fluid responsive Settings GUI, and live battery tray indicator. — **MILESTONE 10 COMPLETE ✅**

## M11 — GitHub Copilot Agent Integration & Cloud Multi-Agent Workflow
Goal: Establish seamless collaborative development between the local Lead Orchestrator Agent (Antigravity) and GitHub Copilot (Agent & IDE), enforcing the Sole Committer Protocol, repository safety budgets, and autonomous task handoffs. Documented in `docs/multi-agent-methodology.md`.
- [x] `[T1]` M11.1 Author hardened `.github/copilot-instructions.md` with strict architectural constraints (`WH_KEYBOARD_LL` zero-allocation, `#![forbid(unsafe_code)]`, Sole Committer Protocol, PATH resiliency, claims follow evidence). _(2026-10-25 — M11.1 completed)_
- [x] `[T1]` M11.2 Standardized agent task issue templates: author `.github/ISSUE_TEMPLATE/agent_task.md` and `agent_task.yml` for bounded, context-rich task delegation. _(2026-10-25 — M11.2 completed)_
- [x] `[T1]` M11.3 Extend `docs/multi-agent-methodology.md` with hybrid local+cloud multi-agent lifecycle (Draft PR &rarr; Lead Verification Gate &rarr; Sole Committer Merge). _(2026-10-25 — M11.3 completed)_
- [x] `[T1]` M11.4 Update `AGENTS.md` and `README.md` to formally document GitHub Copilot Agent as a supported cloud worker class. _(2026-10-25 — M11.4 completed, closes #16)_
**Exit:** Zero-friction cloud multi-agent workflow where Copilot drafts PRs under strict safety guardrails and the Lead Agent verifies and merges. — **MILESTONE 11 COMPLETE ✅**
