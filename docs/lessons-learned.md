# Lessons Learned

Living log of things that went wrong or surprisingly right, and the rule they produced.
**[KIT ✓]** = promoted to the `project-kickstart` plugin. See `docs/retrospective.md` for evidence.

| Date | Context | What happened | Rule / fix adopted | Scope |
|---|---|---|---|---|
| 2026-10-25 | M0 | Hardware probe showed Fn stripped by `kbdhid.sys`, BT VID `0x004C` | Spike unknowns with a probe tool before designing | [KIT ✓] |
| 2026-10-25 | M1–M10 workers | `cargo` not on PATH in nearly every worker shell | Environment preamble in every worker prompt / AGENTS.md | [KIT ✓] |
| 2026-10-25 | M0 | Tool installed mid-session not found | Refresh PATH from the registry after installs | [KIT ✓] |
| 2026-10-25 | M2 integration | `IpcServer::start` signature changed by a worker, caller broken | Interface changes run alone/first; lead re-runs the gate on the merged tree | [KIT ✓] |
| 2026-10-25 | M1–M4 | Missing `windows` crate features / undeclared deps | Declare deps/features in the same change; `cargo add --package` | [KIT ✓] |
| 2026-10-25 | M2, M10, repo health | Named-pipe and global-atomic test races, test hang | Unique OS object names, readiness channels, timeouts | [KIT ✓] |
| 2026-10-25 | M9 | Unbounded search over `~/.cargo/registry` | Scope searches to the workspace | [KIT ✓] |
| 2026-10-25 | v0.1.0/v0.2.0 | Releases and docs claimed quality while hardware QA was Pending | Claims follow evidence; release notes list Pending QA | [KIT ✓] |
| 2026-10-25 | docs | PowerShell corrupted UTF-8 characters | Force `-Encoding utf8`; prefer file tools | [KIT ✓] |
| 2026-10-25 | packaging | `dist/*.zip` committed | Artifacts go to GitHub Releases only | [KIT ✓] |
| 2026-10-25 | orchestration | Worker agent defined only in a chat | Persist agent definitions as files | [KIT ✓] |
| 2026-10-25 | Italian ISO | Right Option synthesizes `LCONTROL` `sc=0x21D` | Swallow `0x21D` when RAlt is remapped | project |
| 2026-10-25 | M11 | GitHub Copilot without permanent instructions could violate hook budgets and Sole Committer rule | Author `.github/copilot-instructions.md` and `agent_task.yml` | [KIT ✓] |
| 2026-10-25 | distribution | Download & install options were built but absent from homepage/README | Document all distribution channels (WinGet, Scoop, Wizard, Portable) prominently in README | [KIT ✓] |
| 2026-10-25 | PowerShell | External CLI stdout capture (`gh`, `git`) on Windows corrupted non-ASCII characters | Set `[Console]::OutputEncoding = UTF8` and force UTF-8 without BOM | [KIT ✓] |
