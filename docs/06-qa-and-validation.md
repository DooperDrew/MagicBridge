# 06 — Quality Assurance & Empirical Validation Matrix

This document defines the quality assurance framework, empirical hardware verification procedures, automated stress gates, and edge-case validation protocols for **MagicBridge**.

---

## 1. QA Architecture & Verification Levels

```mermaid
flowchart TD
    subgraph L1["Level 1: Automated Unit & Invariant Testing (134 tests)"]
        U1["magicbridge-core: Pure logic, schema roundtrip, lookup tables (32 tests)"]
        U2["magicbridge-agent: IPC, autostart, battery parsing, key injection (72 tests)"]
        U3["magicbridge-settings: i18n, key model, wizard, QA matrix, responsive UI (30 tests)"]
    end

    subgraph L2["Level 2: Automated Hardening & Soak Gates"]
        H1["Hook latency budget gate (<0.1ms NFR-P3)"]
        H2["Stuck-key resilience under rapid toggling & aborts (NFR-S3)"]
        H3["Sleep/resume lifecycle & device arrival/removal loops"]
        H4["Concurrent IPC atomic profile swap stress"]
    end

    subgraph L3["Level 3: Empirical Hardware QA (A1843 Physical)"]
        HW1["USB Connection (VID 0x05AC) vs Bluetooth Classic (VID 0x004C)"]
        HW2["ISO Italian AltGr Quirk (sc=0x21D suppression)"]
        HW3["Fn Layer navigation combos & Top Row media/F-keys toggle"]
        HW4["Missing keys (F13..F15 -> PrtSc, ScrLk, Pause)"]
        HW5["Direct HID Battery reporting (Report ID 0x90)"]
    end

    subgraph L4["Level 4: System Integration & Security Edge Cases"]
        E1["UIPI & Elevated Administrator Windows"]
        E2["Game Mode Fullscreen Auto-Pause"]
        E3["Multi-language Windows Input Layout switching"]
    end

    L1 --> L2 --> L3 --> L4
```

---

## 2. Automated Test Suite (Continuous Quality Gate)

Every code change must pass the automated quality gate locally before being considered for release:
```powershell
# Automated Test Execution
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cargo build --workspace --release
```

### Automated Invariants Tested:
1. **Hook Latency Budget (`test_nfr_p3_hook_latency_budget`):**
   - Invariant: Lookup time per key event in `WH_KEYBOARD_LL` must remain strictly below `100 µs` (`0.1 ms`).
   - Result: Benchmark achieves **`0.0006 ms`** (~600 ns), well within budget.
2. **Stuck-Key Resilience (`test_stuck_key_resilience_rapid_toggling` & `test_emergency_key_release_on_pause`):**
   - Invariant: No synthetic modifier or keydown event may linger if the daemon is paused, switched, or terminated.
   - Result: Key state tracking releases all pressed keys automatically.
3. **Soak & High-Concurrency Table Swapping (`test_soak_concurrent_profile_swaps_and_atomic_table`):**
   - Invariant: Dynamic profile switches via IPC must be atomic (`ArcSwap`) without blocking or corrupting the hook thread.
   - Result: 1,000 rapid concurrent swaps verified with zero race conditions.
4. **Sleep/Resume & Device Notification (`test_soak_device_arrival_and_removal_loops`):**
   - Invariant: `WM_POWERBROADCAST` and `WM_DEVICECHANGE` must cleanly re-initialize HID battery poller and input states without memory leaks or deadlocks.

---

## 3. Empirical Hardware QA Test Matrix (Physical Apple A1843)

The following tests require physical connection of the Apple Magic Keyboard with Numeric Keypad (Model A1843):

| Test ID | Test Category | Target Feature / Hardware Invariant | Test Procedure | Expected Outcome | Pass/Fail |
|---|---|---|---|---|---|
| **QA-HW-01** | Connectivity | USB Lightning Connection | Connect A1843 via Lightning-to-USB cable. Launch `magicbridge-agent.exe`. | Recognized with `VID=0x05AC`. Tray icon shows connected status. | Pending Test |
| **QA-HW-02** | Connectivity | Bluetooth Classic Connection | Pair A1843 via Windows Bluetooth settings. Disconnect USB cable. | Recognized with `VID=0x004C`. Battery poller connects via Bluetooth. | Pending Test |
| **QA-HW-03** | Connectivity | Hot-Switching (USB &harr; BT) | Unplug USB cable while typing; let keyboard reconnect via Bluetooth. | Remapping continues seamlessly with zero crash or re-hook delay. | Pending Test |
| **QA-HW-04** | Modifier Presets | Mac Layout Preset | Select "Mac layout" preset in Settings or Tray. Press physical Command and Option. | Cmd emits `VK_LWIN` / Windows Key; Option emits `VK_LMENU` / Alt. | Pending Test |
| **QA-HW-05** | Modifier Presets | Windows Layout Preset | Select "Windows standard" preset in Settings. Press physical Command and Option. | Cmd emits `VK_LCONTROL` / Ctrl; Option emits `VK_LWIN` / Windows Key. | Pending Test |
| **QA-HW-06** | Hardware Quirk | Italian / ISO AltGr (`0x21D`) | Using ISO Italian layout, press physical Right Option (AltGr) with keys like `@`, `#`, `[`, `]`. | AltGr characters produce output normally; orphan `VK_LCONTROL` is suppressed. | Pending Test |
| **QA-HW-07** | Top Row | Media Keys Mode | With Top Row set to Media-first: press F7 (Prev), F8 (Play/Pause), F9 (Next), F10 (Mute), F11 (Vol-), F12 (Vol+). | Windows media functions trigger immediately. | Pending Test |
| **QA-HW-08** | Top Row | F-Keys Mode | Set Top Row to F-keys first: press F1..F12; press Fn + F10. | F1..F12 behave as standard Windows function keys; Fn + F10 mutes audio. | Pending Test |
| **QA-HW-09** | Fn Layer | Navigation Combos | Hold Fn (default Right Option) + Left/Right/Up/Down Arrow; Fn + Backspace. | Left/Right &rarr; Home/End; Up/Down &rarr; PageUp/PageDown; Backspace &rarr; Delete. | Pending Test |
| **QA-HW-10** | Missing Keys | System Keys (F13..F15) | Press F13, F14, F15. | F13 triggers Print Screen (snip tool); F14 triggers Scroll Lock; F15 triggers Pause. | Pending Test |
| **QA-HW-11** | Numpad | Numeric Keypad & Clear | Press keypad keys `0`..`9`, `+`, `-`, `*`, `/`, `Enter`, and `Clear`. | Numpad functions properly; Clear toggles Num Lock (or custom action). | Pending Test |
| **QA-HW-12** | Battery | Direct HID Battery Reporting | Inspect tray icon tooltip and Settings Battery tab over USB and Bluetooth. | Accurate battery percentage displayed; "Charging" flag shown on USB. | Pending Test |

---

## 4. System & Security Edge Cases

| Test ID | Test Category | Invariant Tested | Test Procedure | Expected Outcome | Pass/Fail |
|---|---|---|---|---|---|
| **QA-SYS-01** | Security / UIPI | Elevated Windows (UAC) | Focus an elevated Task Manager or Administrator CMD prompt. Type remapped keys. | Windows UIPI blocks synthetic keystrokes from non-elevated user-mode hooks; unmapped keys pass through safely without crash. | Pending Test |
| **QA-SYS-02** | Power | Modern Standby & S3 Sleep | Put Windows PC to sleep while agent is running; wake up after 1 minute. | Hook resumes cleanly; battery polling continues; zero high-CPU spin. | Pending Test |
| **QA-SYS-03** | Gaming | Game Mode Auto-Pause | Launch a fullscreen DirectX/Vulkan game or listed process (e.g., `steam.exe`). | MagicBridge automatically pauses remappings; native keys pass directly to game; resumes upon game exit. | Pending Test |
| **QA-SYS-04** | Localization | Windows Layout Dynamic Switch | Switch Windows input language (`Win + Space`) between Italian, US English, and German. | Visual key legends adapt; character output follows active Windows input layout. | Pending Test |
| **QA-SYS-05** | Reliability | Rapid Emergency Disconnect | Unplug keyboard during continuous keydown hold. | Emergency release logic fires; synthetic keys do not remain held down in Windows. | Pending Test |

---

## 5. QA Logging & Incident Tracking Protocol

When running empirical tests:
1. Record results directly into this document with date, firmware/OS details, and pass/fail state.
2. If any discrepancy or hardware defect is observed:
   - Check `docs/hardware-findings.md` to verify if this is a known hardware quirk.
   - If novel, log an empirical probe report and create a GitHub Issue labeled `bug,hardware`.
