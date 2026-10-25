---
name: "🤖 Agent Task (Copilot / Subagent)"
about: Structured implementation task designed for AI coding agents and autonomous workers
title: "[Task]: "
labels: ["agent-task", "enhancement"]
assignees: []
---

## 🎯 Task Objective & Scope
<!-- A clear 1-2 sentence description of what needs to be implemented and why -->

## 📊 Complexity Tier & Target
- **Complexity Tier:** Tier 1 (Lean) | Tier 2 (Moderate) | Tier 3 (Complex/Critical)
- **Target Crate:** `crates/core` | `crates/agent` | `crates/settings` | `crates/hid-probe`
- **Target Files (File Ownership):**
  - `crates/...`
  - `tests/...`

## 🛡️ Architectural & Performance Invariants
<!-- Specify any critical constraints (check all that apply) -->
- [ ] **Pure Logic:** Pure Rust in `crates/core` with `#![forbid(unsafe_code)]` and zero Win32 deps.
- [ ] **Hook Safety:** `WH_KEYBOARD_LL` path: zero heap allocations, zero locks, zero file I/O, zero log prints.
- [ ] **ExtraInfo Tagging:** All synthetic inputs set `dwExtraInfo = MAGICBRIDGE_EXTRA_INFO`.
- [ ] **Localization:** User-facing strings must reside in `locales/en.toml` / `locales/it.toml`.
- [ ] **Unsafe Comments:** Every `unsafe` block must carry a `// SAFETY:` rationale.

## 📝 Implementation Instructions
<!-- Step-by-step guidance for the coding agent -->
1. 
2. 
3. 

## ✅ Verifiable Acceptance Criteria
<!-- Checkboxes that can be verified mechanically -->
- [ ] Unit tests added covering all new logic/branches.
- [ ] `cargo fmt --all -- --check` passes with zero formatting diffs.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` produces zero warnings.
- [ ] `cargo test --workspace -- --test-threads=1` passes 100%.
- [ ] `cargo build --workspace --release` compiles without errors.

## 🔄 Sole Committer Protocol Reminder
> **Note for AI Agents:** Deliver your changes via a **Draft Pull Request** on branch `task/<issue-id>-<description>`. Do not commit directly to `main` or attempt to close this issue. The Lead Agent will review, run the integration gate, and merge.
