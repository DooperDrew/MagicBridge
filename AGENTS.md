# AGENTS.md — Mandatory Operating Guidelines for AI Agents

> **CRITICAL FOR ALL AI AGENTS & CODING ASSISTANTS:**
> You are working on **MagicBridge** (`https://github.com/DooperDrew/magicbridge`).
> These guidelines are **strictly binding**. Every agent operating in this repository MUST comply with the workflow, architecture, performance budgets, sole committer protocol, and logging rules defined below without exception.

---

## 1. Project Mission & Identity
- **Name:** MagicBridge
- **Purpose:** A native, ultra-lightweight Windows 10/11 x64 utility that makes the Apple Magic Keyboard with Numeric Keypad (**model A1843**) behave as a first-class Windows keyboard.
- **Key Capabilities:** User-mode Fn layer, media keys vs. F-keys switching, modifier presets, missing system keys (`Print Screen`, `Scroll Lock`, `Pause`), direct HID battery reporting, and named profiles with multiple language layouts (`ISO Italian`, `ANSI US`, `ISO UK`, `ISO German`, `ISO French`, custom).

---

## 2. Multi-Agent Organization & Sole Committer Protocol

To enable clean parallelization across multiple tasks and optimize token consumption across different AI models, MagicBridge enforces a **Hub-and-Spoke (Lead & Worker)** hierarchy:

```mermaid
flowchart TD
    LEAD["Lead Orchestrator Agent (This Session)<br/>• Sole Committer & Release Gatekeeper<br/>• Runs final verification gate<br/>• Updates Roadmap, Changelog & Issues"]
    
    subgraph WORKERS["Parallel Worker Subagents (Token-Optimized)"]
        W1["T1-Core-M1 (flash_lite / flash)<br/>Pure logic, schemas, unit tests"]
        W2["T2-UI-M5 (flash / pro)<br/>egui views & i18n"]
        W3["T3-Hook-M2 (pro / inherit)<br/>WH_KEYBOARD_LL hook & Win32 IPC"]
    end
    
    LEAD -->|"Spawns in isolated branch"| W1
    LEAD -->|"Spawns in isolated branch"| W2
    LEAD -->|"Spawns in isolated branch"| W3
    
    W1 -->|"Passes local tests & sends report"| LEAD
    W2 -->|"Passes local tests & sends report"| LEAD
    W3 -->|"Passes local tests & sends report"| LEAD
    
    LEAD -->|"Reviews diff & runs verification gate"| MERGE["Workspace Merge & Quality Gate"]
    MERGE -->|"git commit & git push origin main"| GITHUB[("GitHub main")]
```

### Roles & Responsibilities:
1. **The Lead Orchestrator Agent (Sole Committer):**
   - The **ONLY** entity authorized to run `git commit`, `git push`, edit git remotes, modify `origin main`, or close GitHub Issues.
   - Responsible for launching workers, assigning complexity tiers, merging worker branches, running the final quality gate, and updating project documentation.
2. **Worker Agents (`magicbridge-worker`):**
   - Work strictly in isolated branches (`Workspace: "branch"`).
   - **STRICTLY PROHIBITED** from running `git commit`, `git push`, `git checkout main`, or `gh issue close`.
   - Write code, run local verification commands, and return a structured completion report to the Lead.
3. **GitHub Copilot Agent (Cloud Worker):**
   - Operates in GitHub Cloud or IDE on isolated feature branches (`task/<issue-id>-<description>`).
   - Bound by `.github/copilot-instructions.md` and tasks structured with `.github/ISSUE_TEMPLATE/agent_task.md`.
   - **STRICTLY PROHIBITED** from committing directly to `main` or closing issues; delivers work exclusively via **Draft Pull Requests** for Lead Agent verification and merge.

### Worker Naming Taxonomy
Every subagent role follows the standard convention: `<Tier>-<Domain>-<Milestone>`
- Examples: `T1-Core-M1` (Core config schema), `T3-Hook-M2` (Input hook engine), `T2-UI-M5` (Settings GUI).
- Branch format: `task/<milestone-id>-<short-description>` (e.g. `task/m1.2-config-schema`).

---

## 3. Dynamic Model Selection & Complexity Tiers

To maximize token efficiency without sacrificing code quality or safety, every task is assigned a complexity tier:

| Tier | Scope & Workload | Criteria in MagicBridge | Target Model |
|---|---|---|---|
| **Tier 1 (T1 — Lean)** | Pure logic, schemas, serialization, unit tests, i18n, packaging scripts, docs. | • Pure Rust (`crates/core`).<br>• `#![forbid(unsafe_code)]`.<br>• Zero Win32 dependencies. | **`flash_lite`** or **`flash`** |
| **Tier 2 (T2 — Moderate)** | UI widgets, event handling, named pipes, standard Win32 message pumps. | • `egui` views & layout widgets.<br>• Tray menu callbacks.<br>• Named pipe client/server. | **`flash`** or **`pro`** |
| **Tier 3 (T3 — Complex / Critical)** | Low-level input hooks, atomic memory swaps, raw HID battery reporting, multi-threaded timing synchronization. | • `WH_KEYBOARD_LL` hook callback.<br>• Performance budget compliance (<0.1ms latency, zero alloc/locks).<br>• Hardware timing quirks (`sc=0x21D`). | **`pro`** or **`inherit`** (Opus / Sonnet) |

### Fallback & Retrospective Audit Rule
If a target model is unavailable (due to token limits, quota exhaustion, or rate limits):
1. **Fallback Selection:** Fall back to the next available model in order: `inherit` &rarr; `pro` &rarr; `flash` &rarr; `flash_lite`.
2. **Mandatory Logging:** The Lead Agent MUST immediately log the occurrence in [`docs/model-fallback-log.md`](docs/model-fallback-log.md) recording the task, target model, fallback model used, and reason.
3. **Audit Task Scheduling:** The Lead Agent MUST schedule a retrospective verification task in `docs/05-roadmap.md` under the active milestone or next release hardening gate to review the fallback implementation with a higher-tier model when tokens/quota permit.

---

## 4. Mandatory Reading Order (Before Any Implementation)
Before writing or modifying any code, the agent MUST read:
1. `docs/01-vision.md`: Core problem, guiding principles, non-goals.
2. `docs/02-requirements.md`: Exact requirements (e.g. F1.1, F7.7, NFR-P1…P5).
3. `docs/03-architecture.md`: Two-process model, crates, IPC, event lifecycle.
4. `docs/hardware-findings.md`: **Hardware ground truth** from physical A1843 testing. (Do NOT assume or guess HID/hardware behavior).
5. `docs/04-ux.md`: System tray specifications and `egui` visual keyboard design.
6. `docs/05-roadmap.md`: The single source of truth for current tasks.
7. `docs/06-qa-and-validation.md`: **Hardware QA Test Matrix** and verification procedures.
8. `docs/lessons-learned.md`: **Living Lessons Learned** (known traps, toolchain quirks, hardware findings).
9. `docs/retrospective.md`: Project retrospective and promoted operational insights.
10. `docs/multi-agent-methodology.md`: Parallel worker orchestration guide and reporting templates.
11. `CHANGELOG.md`: Chronological log of decisions, findings, and completed milestones.

---

## 5. Strict Development Workflow & Life of a Task

### A. Pre-Execution Phase
1. **Locate the Task:** Check `docs/05-roadmap.md` for the next unchecked item or work on the specific task assigned by the user.
2. **Restate Intent:** Clearly restate the task, its complexity tier (`T1`/`T2`/`T3`), target crate/files, and acceptance criteria.
3. **Verify Clean Workspace:** Check `git status` and ensure latest changes from `origin main`.
4. **Environment & PATH Resiliency:** Never assume toolchains (`cargo`, `rustc`) exist in subshell `$env:PATH`. Prepend the user cargo bin path before invoking commands:
   `$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin"`

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
  - `crates/hid-probe` (`hid-probe`): Standalone diagnostic hardware probe utility.
- **Safety Documentation:** Every `unsafe` block in Win32 wrappers MUST be accompanied by a clear `// SAFETY:` explanatory comment detailing preconditions and invariants.
- **No Hardcoded UI Strings:** All user-facing text belongs in translation files (`locales/en.toml`, `locales/it.toml`).
- **Windows / PowerShell Hygiene:**
  - When writing files from PowerShell, always force UTF-8 without BOM (`[System.IO.File]::WriteAllText(..., [System.Text.UTF8Encoding]::new($false))` or `Set-Content -Encoding utf8`) and set `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` before capturing external CLI stdout (`gh`, `git`). Prefer dedicated file-writing tools over shell redirection.
  - Use `;` instead of `&&` for command chaining in PowerShell 5.
  - Scope file searches strictly to the project workspace; never run unconstrained recursive searches over `.cargo`, `target`, or dependency caches.

### D. Verification Gate (MUST PASS Locally)
Before finishing any turn or committing, the Lead Agent MUST run and verify clean output:
```powershell
# 0. Ensure resilient environment PATH
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin"

# 1. Format verification
cargo fmt --all -- --check

# 2. Strict linter (warnings are treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Full test suite (use --test-threads=1 if testing concurrent named pipes/hooks)
cargo test --workspace -- --test-threads=1

# 4. Release compilation check
cargo build --workspace --release
```

### E. Post-Execution & Documentation Synchronization (Lead Agent Only)
Once the task is validated:
1. **Facts vs. Guesses (Claims Follow Evidence):** Never mark physical hardware tests as passed without empirical evidence from testing on a physical A1843 keyboard. Automated invariant tests and unit tests are marked PASS; physical matrix rows in `docs/06-qa-and-validation.md` and roadmap remain marked `[Pending Physical Confirmation]`.
2. **Update Roadmap:** Tick the completed box `[x]` in `docs/05-roadmap.md` with commit/date notes (or record `[Pending Physical Confirmation]` for hardware items).
3. **Log to CHANGELOG.md:** Add a comprehensive entry to `CHANGELOG.md` detailing changes, architectural decisions, and hardware discoveries.
4. **Update Docs:** Update `docs/` or ADRs if architectural decisions changed.
5. **Git Cleanliness:** Built binaries or packages (`dist/*.zip`, `.msi`) MUST NEVER be committed to git. Release assets are attached exclusively to GitHub Releases.
6. **Git Commit & Push:** Commit with an informative message (e.g. `M2.3: modifier presets and per-key remap`) and push to `origin main`.
7. **Update GitHub Issue / Milestone:** Update or close the corresponding GitHub Issue using `gh issue close <id>` or `gh issue edit`.
8. **GitHub Governance (Milestones, Issues & Releases):** Every roadmap milestone (`M0`..`Mn`) MUST map 1:1 to a GitHub Milestone and tracked GitHub Issues (`agent_task.yml` for Copilot/worker tasks). Completed tasks close their issue with a commit/release reference; pending hardware/manual QA stays open in its milestone. Every release milestone MUST publish a pre-built downloadable archive (`.zip` / installer) to GitHub Releases (`gh release create`) even while physical hardware QA remains `Pending`.

---

## 6. Hardware Realities & Known Quirks (From M0)
- **Vendor IDs:** The keyboard identifies with `VID = 0x05AC` on USB and **`VID = 0x004C`** over Bluetooth Classic. Always check both via `magicbridge_core::is_apple_vendor`.
- **Physical Fn / Eject:** Stripped by Windows `kbdhid.sys` in kernel mode. MagicBridge relies on user-configurable trigger keys (default: Right Option / `VK_RMENU`).
- **ISO / Italian AltGr Quirk:** Pressing physical Right Option on ISO Italian layouts causes Windows to synthesize an artificial `VK_LCONTROL` with `scanCode == 0x21D` ~0.8 ms earlier. The remapping hook must swallow `0x21D` whenever Right Option is remapped or used as Fn.
- **Battery Report:** Read from Apple Vendor Collection `0xFF00:0x0014` report ID `0x90` (`[0x90, status_flags, pct]`).

---

## 7. Escalation Rule
If physical keyboard behavior or testing contradicts existing documentation:
**STOP immediately. Do not guess or silently hack around it.**
Present the empirical findings to the user, formulate a probe or verification step, and update `docs/hardware-findings.md` upon user alignment.

---

## 8. Continuous Learning Loop
When the user corrects or redirects the agent, the agent MUST proactively propose adding the correction as an explicit rule to:
- `AGENTS.md` (if project-specific, e.g. Win32 hook timings, layout nuances, crate boundaries)
- `docs/lessons-learned.md` (if generic to engineering, tooling, or agentic workflow)
Do not simply apologize or apply it silently in code.
