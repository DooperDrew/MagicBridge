# AGENTS.md — Rules for AI agents working on this repo

## Project
**MagicBridge** (working name): a tiny native Windows 10 x64 utility that makes the Apple Magic Keyboard
A1843 work fully on Windows (Fn layer, media keys, modifier layout, missing keys, battery), with a small
settings GUI.

## Read before coding
1. `docs/01-vision.md`: goals, principles, non-goals
2. `docs/02-requirements.md`: requirements (IDs like F1.2, NFR-P3)
3. `docs/03-architecture.md`: stack, process model, technical design
4. `docs/04-ux.md`: tray + settings GUI
5. `docs/05-roadmap.md`: **the task list. Work on the task the user names, or the next unchecked one.**
6. `docs/hardware-findings.md` (after M0): the ground truth about the keyboard's HID behaviour
7. `docs/decisions/`: ADRs. Don't contradict them without proposing a new ADR.

## Workflow
- One roadmap task per conversation unless told otherwise. Before writing code, restate the task and its acceptance criteria.
- When done: tick the box in `docs/05-roadmap.md`, update docs if the design changed, and commit with a message like `M2.3: modifier presets`.
- If reality contradicts the docs (especially 🔬 items), **stop and tell the user**, then update the docs once they agree.
- Hardware behaviour can't be guessed. When something depends on the physical keyboard, write a probe or test and ask the user to run it.

## Hard rules (performance & safety)
- **Hook callback:** no heap allocation, no I/O, no logging, no locks, no syscalls besides `CallNextHookEx`/`SendInput`. Precomputed table lookup only.
- **No polling loops.** Everything is event-driven, except battery polling at the configured interval.
- **No DLL injection, no kernel drivers, no admin requirement, no network access.**
- Always mark injected events (`dwExtraInfo`) and ignore them in the hook.
- Never leave keys stuck: track pressed state and release synthetic keys on disable/exit/crash paths.
- Budgets in NFR-P1…P5 are requirements. Flag any change that could affect them.

## Code conventions
- Rust stable, Cargo workspace under `crates/`. `cargo fmt` + `cargo clippy -- -D warnings` must pass.
- Business logic goes in `crates/core` (pure, unit-tested, no `windows` crate dependency).
- `unsafe` only in thin Win32 wrapper modules, each with a `// SAFETY:` comment.
- User-facing strings go through the i18n files (`en.toml`, `it.toml`). No hardcoded UI text.

## Build & run
Prerequisites: Rust stable `x86_64-pc-windows-msvc` (pinned via `rust-toolchain.toml`, with `clippy` + `rustfmt`)
and Visual Studio (Build Tools) with the C++ workload. Run everything from the repo root.

| Purpose | Command |
|---|---|
| Build (debug) | `cargo build --workspace` |
| Build (release) | `cargo build --workspace --release` → `target\release\*.exe` |
| Test | `cargo test --workspace` |
| Format check | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Run agent | `cargo run -p magicbridge-agent` |
| Run settings | `cargo run -p magicbridge-settings` |
| Run hid-probe | `cargo run -p hid-probe -- <args>` |

Package names: the `crates/core` package is `magicbridge-core` (Rust name `magicbridge_core`), because
a crate named `core` would clash with Rust's built-in `core` library. Release builds of the agent and settings use the
Windows GUI subsystem (no console); debug builds keep the console for diagnostics.
