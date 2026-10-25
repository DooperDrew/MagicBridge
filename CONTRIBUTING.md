# Contributing to MagicBridge

Thank you for your interest in contributing to **MagicBridge**! This document explains our principles, architectural constraints, development workflow, and coding guidelines.

---

## 1. Guiding Principles & Non-Negotiable Budgets

MagicBridge runs quietly in the background on Windows. To ensure it never degrades system or gaming responsiveness, all contributions must respect these constraints:

1. **Zero Idle CPU (NFR-P2):** Everything is strictly event-driven (Win32 message loops, device notifications, and named pipes). No polling loops (the only periodic task is battery checks every N minutes).
2. **Hook Safety & Latency (NFR-P3):** The `WH_KEYBOARD_LL` low-level hook callback must remain ultra-lean:
   - **No heap allocations (`Vec`, `String`, `Box`, etc.).**
   - **No file/disk I/O.**
   - **No logging / printing.**
   - **No locks or mutexes** (use precomputed immutable tables or lock-free atomic references).
   - Only call `CallNextHookEx` or `SendInput`.
3. **No Kernel Drivers / Anti-Cheat Safe (NFR-S1):** No `.sys` drivers and no DLL injection. Everything operates in user space.
4. **Resilient Cleanup (NFR-S3):** Synthetic events must never leave keys stuck down, even on unexpected exit or panic. All injected inputs must carry a distinctive `dwExtraInfo` marker to avoid recursive loops.

---

## 2. Repository Architecture

The project is structured as a Cargo workspace under `crates/`:

- [`crates/core`](crates/core) (`magicbridge-core`): Pure business logic (key definitions, profile definitions, compilation to lookup tables, config serialization).
  - **Constraint:** Strictly `#![forbid(unsafe_code)]`. No `windows` crate dependencies. 100% covered by unit tests.
- [`crates/agent`](crates/agent) (`magicbridge-agent`): The background tray executable.
  - Hosts the low-level hook, HID battery reader, single-instance mutex, and named pipe IPC server.
- [`crates/settings`](crates/settings) (`magicbridge-settings`): The user settings GUI.
  - Built with `egui`/`eframe`. Runs **only** while the window is open. Exits completely when closed.
- [`crates/hid-probe`](crates/hid-probe) (`hid-probe`): Hardware diagnostic CLI tool used during the M0 hardware spike.

---

## 3. Development Environment

### Prerequisites
- **OS:** Windows 10 or 11 (64-bit).
- **Toolchain:** Rust stable (`x86_64-pc-windows-msvc`), managed via `rustup` and configured in `rust-toolchain.toml`.
- **C++ Tools:** Visual Studio Build Tools (C++ workload for MSVC linker).

### Essential Commands

Run all commands from the repository root:

```powershell
# 0. Resilient environment PATH preamble
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin"

# 1. Check code formatting (rustfmt)
cargo fmt --all -- --check

# 2. Run Clippy (warnings treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Run unit and integration tests (isolated test threads)
cargo test --workspace -- --test-threads=1

# 4. Build debug binaries
cargo build --workspace

# 5. Build optimized release binaries
cargo build --workspace --release
```

---

## 4. Coding Conventions

- **Unsafe Code:** Unsafe code is strictly forbidden in `magicbridge-core`. Where Win32 calls are required in `magicbridge-agent` or `magicbridge-settings`, isolate them into thin wrapper functions and annotate each with an explicit `// SAFETY:` rationale explaining invariants.
- **Internationalization (i18n):** User-facing strings must never be hardcoded into UI code. Place string keys in the localization files (e.g. `locales/en.toml`, `locales/it.toml`).
- **Formatting:** Keep code compliant with standard `cargo fmt`.
- **Commit Messages:** Use descriptive, imperative commit messages prefixed with the milestone or task ID where applicable (e.g., `M2.3: modifier presets and per-key remap`).

---

## 5. Working on Milestones & Collaborative Workflow

We track our roadmap in [`docs/05-roadmap.md`](docs/05-roadmap.md) and through [GitHub Issues](https://github.com/DooperDrew/magicbridge/issues).

### Branch Strategy
- Main branch (`main`) is protected by the **Sole Committer Protocol**.
- Human developers work on feature branches: `<type>/<milestone>-<short-description>` (e.g., `feat/m10-battery-tray`).
- AI agents (GitHub Copilot Agent or local subagents) work on task branches: `task/<issue-id>-<description>` (e.g., `task/16-copilot-workflow`).

### Cloud Multi-Agent Task Lifecycle
```
[ Lead Agent (Antigravity) ] ──► Creates GitHub Issue (.github/ISSUE_TEMPLATE/agent_task.yml)
                                               │
                                               ▼
                              [ GitHub Copilot Agent (Cloud) ]
                                               │
                                               ▼
                              Opens Draft PR (task/<issue-id>-<desc>)
                                               │
                                               ▼
[ Lead Agent (Antigravity) ] ──► Runs Verification Gate locally & Merges to main
```

Before picking up or drafting a task:
1. Review [`docs/01-vision.md`](docs/01-vision.md), [`docs/02-requirements.md`](docs/02-requirements.md), and [`docs/03-architecture.md`](docs/03-architecture.md).
2. Check [`docs/hardware-findings.md`](docs/hardware-findings.md) for ground truths regarding hardware behaviors (such as Apple vendor IDs and the ISO `AltGr` scancode quirk).
3. Ensure all tests, clippy checks, and verification gates pass cleanly prior to opening a Pull Request.

---

## 6. Guidelines for AI Agents

All AI assistants (Antigravity, GitHub Copilot, Claude Code, Cursor, etc.) must follow [`AGENTS.md`](AGENTS.md) and [`.github/copilot-instructions.md`](.github/copilot-instructions.md) as strictly binding instructions:
- **Lead Agent (Antigravity):** Operates as the **Sole Committer** when interacting with Git remotes. Manages issues, executes verification gates on merged trees, and merges pull requests.
- **Worker Subagents & Copilot Agent:** Strictly forbidden from running `git commit`, `git push`, or merging directly to `main`. Deliver work exclusively via **Draft Pull Requests** on `task/<issue-id>-<description>`.
- **Claims Follow Evidence:** Never mark physical hardware tests as passed without empirical human test evidence; mark physical QA rows as `[Pending Physical Confirmation]`.
- **Windows / PowerShell Hygiene:** Force UTF-8 encoding when writing files on Windows (`Set-Content -Encoding utf8`). Avoid unconstrained recursive searches over dependency caches (`target/`, `.cargo/`). Use `;` instead of `&&` in PowerShell 5.
