# Operating Rules & Instructions for GitHub Copilot (MagicBridge)

> **Repository:** `https://github.com/DooperDrew/magicbridge`  
> **Role of GitHub Copilot:** Worker Subagent / Fast Draft Implementer  
> **Lead Orchestrator & Sole Committer:** Antigravity (Local Lead Agent)

Whenever working on this repository — whether via Copilot Agent, Copilot in VS Code / IDE, or GitHub Pull Request reviewer — you MUST strictly comply with the guidelines, architecture, performance budgets, and Sole Committer protocol defined below and in [`AGENTS.md`](../AGENTS.md).

---

## 1. Division of Labor & Sole Committer Protocol

MagicBridge enforces a strict **Hub-and-Spoke (Lead Orchestrator & Worker)** multi-agent model:

```
[ User ]  ──► [ Lead Agent (Antigravity) ]
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
 Project Docs & Issue Prep   Verification & Quality Gate
(Tasks & Acceptance Criteria) (Runs tests, checks budgets)
        │                         ▲
        ▼                         │
 [ GitHub Issue #N ]              │
        │                         │
        ▼                         │
[ Copilot Agent ] ──( Draft PR )──┘
```

### Copilot Responsibilities & Boundaries:
- **Fast Draft Implementer:** Consume self-contained GitHub Issues, generate boilerplate, implement pure logic / UI views, and write unit tests.
- **Draft Pull Requests Only:** Work on dedicated feature branches (`task/<issue-id>-<description>`) and submit **Draft Pull Requests**.
- **PROHIBITED:** Never execute `git push origin main`, never merge PRs directly into `main`, and never close GitHub Issues. Only the Lead Agent (Antigravity) is authorized to run the final release gate, merge to `main`, and close issues.

---

## 2. Technology Stack & Workspace Structure

- **Language:** Rust (2024 edition, stable toolchain).
- **Architecture:** Two-process user-mode architecture (`magicbridge-agent` background daemon + `magicbridge-settings` on-demand GUI).

### Crate Separation Boundaries:
1. **`crates/core` (`magicbridge-core`):**
   - Pure business logic, data models, key mappings, config serialization.
   - **STRICT RULE:** `#![forbid(unsafe_code)]`. Zero Win32 dependencies. Zero OS I/O.
   - 100% unit-tested.
2. **`crates/agent` (`magicbridge-agent`):**
   - Native Windows background daemon.
   - Low-level keyboard hook (`WH_KEYBOARD_LL`), Win32 message pump, named pipe IPC server (`\\.\pipe\MagicBridge_Agent_IPC`), system tray icon, and raw HID battery telemetry.
3. **`crates/settings` (`magicbridge-settings`):**
   - Standalone configuration GUI built with `eframe` / `egui`.
   - Only resident in memory while open. Connects to `magicbridge-agent` via IPC.
   - Responsive layouts, onboarding wizard, hardware QA diagnostic matrix.
4. **`crates/hid-probe` (`hid-probe`):**
   - Standalone hardware diagnostic probe CLI utility for Apple HID device discovery.

---

## 3. Non-Negotiable Performance & Safety Budgets

Every contribution by Copilot MUST respect these non-functional requirements:

### A. The Hook Thread Rule (NFR-P3 — CRITICAL):
Inside the `WH_KEYBOARD_LL` low-level input hook callback (`crates/agent/src/hook.rs`):
- **NEVER** allocate heap memory (`Vec`, `String`, `Box`, `format!`, `to_string()`).
- **NEVER** perform disk, file, or network I/O.
- **NEVER** write log prints or console output.
- **NEVER** acquire blocking synchronization locks or mutexes.
- **NEVER** make blocking Win32 syscalls.
- **Precomputed flat arrays and O(1) table lookups ONLY.** Target hook execution latency is `< 0.1 ms` (< 100 microseconds).

### B. Anti-Cheat & Driver Safety (NFR-S1):
- **NO kernel drivers (`.sys`), NO driver signatures, and NO DLL injection.**
- 100% user-mode Win32 API.

### C. Loop Prevention:
- Every synthetic key sent via `SendInput` (`crates/agent/src/injector.rs`) MUST set `dwExtraInfo = MAGICBRIDGE_EXTRA_INFO`.
- The hook callback MUST check `dwExtraInfo` and pass tagged inputs through immediately without processing to avoid infinite synthetic injection loops.

### D. Stuck-Key Resilience (NFR-S3):
- Synthetic keys must never remain stuck in the down state if the daemon is terminated, paused, or suspended.
- Implement explicit key-up cleanup on profile switches, game mode activations, and shutdown.

### E. Safety Comments:
- Every `unsafe` block in Win32 wrappers MUST be preceded by a clear, explanatory `// SAFETY:` comment documenting preconditions and invariants.

### F. No Hardcoded Strings:
- All user-facing UI text in `crates/settings` belongs in localized translation tables (`locales/en.toml`, `locales/it.toml`).

---

## 4. Hardware Ground Truth & Known Quirks

MagicBridge is specifically calibrated for the **Apple Magic Keyboard with Numeric Keypad (Model A1843)**. Consult `docs/hardware-findings.md` before making any hardware assumptions:

1. **Vendor IDs:**
   - Lightning-to-USB Cable: `VID = 0x05AC`
   - Bluetooth Classic: **`VID = 0x004C`** (Apple broadcom controller).
   - Always check both vendor IDs via `magicbridge_core::is_apple_vendor`.
2. **Fn Key / Eject Kernel Stripping:**
   - Windows kernel driver `kbdhid.sys` completely strips physical Fn and Eject events before they reach user mode.
   - Do NOT attempt to read physical Fn from raw input. MagicBridge uses user-configurable trigger keys (default: Right Option / `VK_RMENU`).
3. **Italian / ISO AltGr Quirk (`sc=0x21D`):**
   - Pressing physical Right Option on ISO Italian keyboard layouts causes Windows to automatically synthesize an artificial `VK_LCONTROL` keydown with `scanCode == 0x21D` ~0.8 ms before `VK_RMENU`.
   - The hook engine must swallow `0x21D` whenever Right Option is remapped or used as the Fn trigger.
4. **Battery Reporting:**
   - Battery telemetry is read via Apple Vendor Collection `0xFF00:0x0014`, report ID `0x90` (`[0x90, status_flags, pct]`).

---

## 5. Verification Commands & Environment Hygiene

### Toolchain PATH Resiliency & PowerShell Hygiene (Windows):
Never assume `cargo` is present in subshell `$env:PATH`. In PowerShell, always execute:
```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin"
```
When writing files or executing CLI commands via PowerShell:
- Force UTF-8 without BOM: `[System.IO.File]::WriteAllText(..., [System.Text.UTF8Encoding]::new($false))` or `Set-Content -Encoding utf8`.
- Set console output encoding before capturing CLI stdout (`gh`, `git`): `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8`.
- Scope file searches strictly to the workspace; never recurse into dependency caches (`target/`, `.cargo/`).
- Use `;` instead of `&&` for command chaining in PowerShell 5.

### Verification Gate (Must Pass 100% Locally):
Before submitting a Draft PR or marking a task complete, run:
```powershell
# 1. Format verification
cargo fmt --all -- --check

# 2. Strict linter (warnings are treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Full test suite (isolated thread runner for named pipes)
cargo test --workspace -- --test-threads=1

# 4. Release compilation check
cargo build --workspace --release
```

### Claims Follow Evidence Rule:
- Automated invariants, unit tests, and mock tests are marked **PASS**.
- Physical hardware validation items on an Apple Magic Keyboard A1843 must remain marked **`[Pending Physical Confirmation]`** until human testing logs are captured. Never claim physical hardware behavior is verified without empirical test data.
