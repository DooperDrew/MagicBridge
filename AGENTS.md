# AGENTS.md — Mandatory Operating Guidelines for AI Agents

> **CRITICAL FOR ALL AI AGENTS & CODING ASSISTANTS:**
> You are working on **MagicBridge** (`https://github.com/DooperDrew/test-repo-antigravity`).
> These guidelines are **strictly binding**. Every agent operating in this repository MUST comply with the workflow, architecture, performance budgets, and logging rules defined below without exception.

---

## 1. Project Mission & Identity
- **Name:** MagicBridge
- **Purpose:** A native, ultra-lightweight Windows 10/11 x64 utility that makes the Apple Magic Keyboard with Numeric Keypad (**model A1843**) behave as a first-class Windows keyboard.
- **Key Capabilities:** User-mode Fn layer, media keys vs. F-keys switching, modifier presets, missing system keys (`Print Screen`, `Scroll Lock`, `Pause`), direct HID battery reporting, and named profiles with multiple language layouts (`ISO Italian`, `ANSI US`, `ISO UK`, `ISO German`, `ISO French`, custom).

---

## 2. Mandatory Reading Order (Before Any Implementation)
Before writing or modifying any code, the agent MUST read:
1. `docs/01-vision.md`: Core problem, guiding principles, non-goals.
2. `docs/02-requirements.md`: Exact requirements (e.g. F1.1, F7.7, NFR-P1…P5).
3. `docs/03-architecture.md`: Two-process model, crates, IPC, event lifecycle.
4. `docs/hardware-findings.md`: **Hardware ground truth** from physical A1843 testing. (Do NOT assume or guess HID/hardware behavior).
5. `docs/04-ux.md`: System tray specifications and `egui` visual keyboard design.
6. `docs/05-roadmap.md`: The single source of truth for current tasks.
7. `CHANGELOG.md`: Chronological log of decisions, findings, and completed milestones.

---

## 3. Strict Development Workflow & Life of a Task

Whenever an agent takes on a task or milestone step:

### A. Pre-Execution Phase
1. **Locate the Task:** Check `docs/05-roadmap.md` for the next unchecked item or work on the specific task assigned by the user.
2. **Restate Intent:** Clearly restate the task, target crate/files, and its acceptance criteria before making edits.
3. **Verify Clean Workspace:** Check `git status` and pull latest changes from `origin main`.

### B. Implementation & Performance Guards
- **Hook Thread Rule (NFR-P3):** The `WH_KEYBOARD_LL` callback in `magicbridge-agent` MUST NEVER allocate memory (`Vec`, `String`, `Box`), perform disk/file I/O, output log prints, acquire locks, or invoke blocking syscalls. Precomputed lookup tables only.
- **Event-Driven Only (NFR-P2):** Zero busy loops or polling timers. All reactions must stem from Win32 message queues, device arrival notifications, or named pipes (except low-frequency battery polling).
- **Anti-Cheat & Driver Rule (NFR-S1):** Strictly **no kernel drivers**, no `.sys` binaries, and **no DLL injection**.
- **Loop Prevention:** Every synthetic input sent via `SendInput` MUST include the distinct `dwExtraInfo` marker (`MAGICBRIDGE_EXTRA_INFO`) and the hook must ignore it.
- **Stuck-Key Resilience (NFR-S3):** Synthetic keys must never remain stuck down if the daemon is closed, suspended, or encounters an error.

### C. Code Conventions & Quality Checks
- **Crate Separation:**
  - `crates/core` (`magicbridge-core`): Pure business logic. `#![forbid(unsafe_code)]`. Zero Win32 dependencies. 100% unit tested.
  - `crates/agent` (`magicbridge-agent`): Daemon, hooks, tray, HID, IPC server.
  - `crates/settings` (`magicbridge-settings`): `egui` GUI, only resident while open.
- **Safety Documentation:** Every `unsafe` block in Win32 wrappers MUST be accompanied by a clear `// SAFETY:` explanatory comment detailing preconditions and invariants.
- **No Hardcoded UI Strings:** All user-facing text belongs in translation files (`locales/en.toml`, `locales/it.toml`).

### D. Verification Gate (MUST PASS Locally)
Before finishing any turn or committing, the agent MUST run and verify clean output:
```powershell
# 1. Format verification
cargo fmt --all -- --check

# 2. Strict linter (warnings are treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Full test suite
cargo test --workspace

# 4. Release compilation check
cargo build --workspace --release
```

### E. Post-Execution & Documentation Synchronization
Once the task is validated:
1. **Update Roadmap:** Tick the completed box `[x]` in `docs/05-roadmap.md` with commit/date notes.
2. **Log to CHANGELOG.md:** Add a comprehensive entry to `CHANGELOG.md` detailing changes, architectural decisions, and hardware discoveries.
3. **Update Docs:** Update `docs/` or ADRs if architectural decisions changed.
4. **Git Commit & Push:** Commit with an informative message (e.g. `M2.3: modifier presets and per-key remap`) and push to `origin main`.
5. **Update GitHub Issue / Milestone:** Update or close the corresponding GitHub Issue using `gh issue close <id>` or `gh issue edit`.

---

## 4. Hardware Realities & Known Quirks (From M0)
- **Vendor IDs:** The keyboard identifies with `VID = 0x05AC` on USB and **`VID = 0x004C`** over Bluetooth Classic. Always check both via `magicbridge_core::is_apple_vendor`.
- **Physical Fn / Eject:** Stripped by Windows `kbdhid.sys` in kernel mode. MagicBridge relies on user-configurable trigger keys (default: Right Option / `VK_RMENU`).
- **ISO / Italian AltGr Quirk:** Pressing physical Right Option on ISO Italian layouts causes Windows to synthesize an artificial `VK_LCONTROL` with `scanCode == 0x21D` ~0.8 ms earlier. The remapping hook must swallow `0x21D` whenever Right Option is remapped or used as Fn.
- **Battery Report:** Read from Apple Vendor Collection `0xFF00:0x0014` report ID `0x90` (`[0x90, status_flags, pct]`).

---

## 5. Escalation Rule
If physical keyboard behavior or testing contradicts existing documentation:
**STOP immediately. Do not guess or silently hack around it.**
Present the empirical findings to the user, formulate a probe or verification step, and update `docs/hardware-findings.md` upon user alignment.
