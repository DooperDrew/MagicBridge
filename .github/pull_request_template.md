## Description
<!-- Provide a brief summary of the changes introduced in this PR and their purpose. -->

Closes #<!-- issue number -->

## Type of Change
- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Documentation update
- [ ] Performance improvement / hardening
- [ ] Refactor (no behavioral changes)

## Contributor Checklist
Before submitting this pull request, please verify:
- [ ] Code compiles cleanly: `cargo build --workspace`
- [ ] Formatting passes: `cargo fmt --all -- --check`
- [ ] Linter passes with no warnings: `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Test suite passes: `cargo test --workspace`
- [ ] **Performance & Budgets:** No allocations, file I/O, or locks in the `WH_KEYBOARD_LL` hook callback (NFR-P3).
- [ ] **Safety:** Any `unsafe` block includes an explicit `// SAFETY:` rationale explaining invariants.
- [ ] Injected inputs are tagged with the distinct extra info sentinel to prevent input loops.
- [ ] `docs/05-roadmap.md` and `CHANGELOG.md` updated if completing a milestone step.
