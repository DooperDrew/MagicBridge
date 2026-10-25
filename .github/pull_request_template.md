## Description
<!-- Provide a brief summary of the changes introduced in this PR and their purpose. -->
<!-- Link the roadmap task (e.g. M11.2) and/or issue number (#N). -->

Closes #<!-- issue number -->

## Type of Change
- [ ] 🤖 Agent implementation task (Draft PR)
- [ ] 🐛 Bug fix (non-breaking change fixing an issue)
- [ ] ✨ New feature (non-breaking change adding functionality)
- [ ] 📝 Documentation update
- [ ] ⚡ Performance improvement / hardening
- [ ] ♻️ Refactor (no behavioral changes)

## Requirements Touched
<!-- Specify requirement IDs from docs/02-requirements.md e.g., F1.2, NFR-P3 -->

## Contributor & Agent Checklist
Before submitting or merging this pull request, verify:
- [ ] **Verification Gate:**
  - Formatting: `cargo fmt --all -- --check`
  - Strict Linter: `cargo clippy --workspace --all-targets -- -D warnings`
  - Automated Tests: `cargo test --workspace -- --test-threads=1`
  - Build: `cargo build --workspace --release`
- [ ] **Performance Budgets (NFR-P3):** No heap allocations (`Vec`, `String`, `Box`, `format!`), file/disk I/O, or locks in the `WH_KEYBOARD_LL` hook callback (<0.1ms latency).
- [ ] **Safety & Invariants (NFR-S1):** Pure user-mode Win32. Every `unsafe` block includes an explicit `// SAFETY:` rationale.
- [ ] **Loop Prevention:** Synthetic inputs sent via `SendInput` tagged with `MAGICBRIDGE_EXTRA_INFO`.
- [ ] **No Hardcoded Strings:** User-facing strings defined in localized translation dictionaries (`locales/en.toml`, `locales/it.toml`).
- [ ] **Claims Follow Evidence:** Automated invariant tests pass; physical hardware QA items remain marked `[Pending Physical Confirmation]` until empirical confirmation is logged.
- [ ] **Sole Committer Protocol:** AI agents deliver changes as a **Draft PR** on a `task/<issue-id>-<description>` branch. Only the Lead Agent (Antigravity) merges to `main`.

## Documentation Updates
- [ ] Roadmap task ticked in `docs/05-roadmap.md` (upon lead merge)
- [ ] CHANGELOG entry added in `CHANGELOG.md`
- [ ] Architecture / ADR updated (if design changed)
- [ ] Lessons learned recorded in `docs/lessons-learned.md` (if any friction occurred)
