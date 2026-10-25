# Operating Rules for AI Coding Assistants (MagicBridge)

Whenever working on this repository, you MUST strictly adhere to the operational principles, architecture, performance budgets, and synchronization workflows defined in [AGENTS.md](../AGENTS.md).

## Non-Negotiable Essentials:
1. **Never allocate, perform I/O, lock, or log** inside the `WH_KEYBOARD_LL` input hook callback (`magicbridge-agent`).
2. **Never add kernel drivers (`.sys`) or DLL injection.**
3. **Pure business logic belongs in `crates/core`** with `#![forbid(unsafe_code)]` and zero Win32 dependencies.
4. **Annotate every `unsafe` block** with an explanatory `// SAFETY:` comment.
5. **Hardware ground truth:** Consult `docs/hardware-findings.md` for verified Apple A1843 USB (`0x05AC`) and Bluetooth (`0x004C`) behaviors and the Italian `AltGr` `0x21D` scancode quirk.
6. **Task lifecycle:** Before ending any milestone task:
   - Run `cargo fmt --all -- --check`
   - Run `cargo clippy --workspace --all-targets -- -D warnings`
   - Run `cargo test --workspace`
   - Tick the box in `docs/05-roadmap.md`
   - Log decisions and steps in `CHANGELOG.md`
   - Push to `origin main` and update/close the GitHub Issue via `gh issue`.
