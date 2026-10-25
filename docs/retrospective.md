# MagicBridge — Retrospective (M0 → v0.2.0)

> Date: 2026-10-25 · Scope: 26 commits, milestones M0–M10, releases v0.1.0 / v0.2.0,
> 19 agent conversations (3 interactive lead sessions + 16 worker subagents).
> Method: transcript mining by 3 parallel research subagents + git history + docs review.
> Generic lessons were promoted to the `project-kickstart` plugin (v1.0.0).

## 1. Timeline

| Phase | Commits | Conversations |
|---|---|---|
| Definition (vision, requirements, architecture, roadmap, AGENTS.md) | `8cbeb8b` | [Project definition](conversation://8b5c8372-6864-48d7-b03e-af49fbb1944a) |
| M0 hardware spike: `hid-probe`, USB/BT findings, VID `0x004C` over BT | `4935fbf` → `ac95648` | [M0 lead session](conversation://74a20b38-6821-4721-a963-ab06ab006a26) |
| Process: CI, templates, Sole Committer, tiers, fallback log | `5369267` → `21ca1fb` | [Lead orchestrator](conversation://ef69a3e5-bec9-4b08-9056-e029d4b12cf9) |
| M1–M2 skeleton, engine, hook, IPC (parallel workers) | `b0d6dac` → `fb3e069` | workers `f9c62416`, `239861da`, `c02fe291`, `2d845bf5`, `84015013`, `aac29874`, `c40a9dfd` |
| M3–M5 top row, battery, Settings GUI | `aaddee6`, `01e5e6f` | workers `168ee8e1`, `5cf471b9`, `92a32f53` |
| M6–M7 hardening, game mode, packaging → v0.1.0 | `5e343ef` | workers `01a026f6`, `36fa3616` |
| M8–M10 methodology doc, installer, onboarding, responsive UI, QA tool → v0.2.0 | `f44f470` → `892674e` | workers `017d0f44`, `092b4c29`, `63acd115`, `a4b6ceba` |
| Repo health: IPC test hang fixed | `13e8acb` | lead `ef69a3e5` |

## 2. What worked
- **Spec first, then M0 hardware spike.** The `hid-probe` tool turned three 🔬 unknowns into facts before any daemon code existed:
  Fn is stripped by `kbdhid.sys`, BT uses VID `0x004C`, battery is in report `0x90`, and there's the ISO AltGr `0x21D` quirk.
  Without it, the Fn design would have been wrong.
- **One roadmap task per conversation / worker**, with IDs and acceptance criteria. Workers could start cold from `AGENTS.md`.
- **Isolated branch workspaces + Sole Committer.** Up to 4 parallel workers per phase with zero merge conflicts.
- **Module boundaries designed for parallelism.** The `core` (pure, `forbid(unsafe)`) / `agent` / `settings` split made cross-crate parallel work trivial.
- **Mechanical verification gate** (fmt, clippy `-D warnings`, test, release build) run by workers *and* again by the lead after integration.
  The lead's re-run caught real integration bugs (see §3).
- **Structured worker completion reports** made integration fast.
- **Complexity tiers** kept heavy models for hooks/HID/concurrency and lean models for schemas, scripts and docs.

## 3. What went wrong (evidence)
| # | Issue | Evidence | Impact |
|---|---|---|---|
| 1 | **`cargo` missing from PATH in worker shells.** Happened in essentially every worker. | 13+ workers, e.g. `2d845bf5`, `c40a9dfd`, `a4b6ceba` | 2–3 wasted steps per worker |
| 2 | PATH not refreshed after the user installed Rust mid-session | `74a20b38` (user: *"i've installed rust at system level please try again"*) | User had to intervene |
| 3 | PowerShell: stderr treated as an error, quote/escaping and multiline failures | `74a20b38`, `ef69a3e5`, `092b4c29`, `63acd115` | Repeated retries |
| 4 | **API signature drift across parallel workers** (`IpcServer::start` went from 2 to 4 arguments, caller not updated) | `ef69a3e5` | Caught only by the lead's gate |
| 5 | Undeclared dependencies / missing `windows` crate features (`Win32_Security`, `Win32_System_Pipes`, `serde_json`, `webbrowser`) | `c02fe291`, `c40a9dfd`, `092b4c29`, `ef69a3e5` | Compile failures |
| 6 | **Test races on global OS state** (named-pipe readiness, process-wide hook atomics) and a test-suite **hang** | `c40a9dfd`, `a4b6ceba`, `ef69a3e5` → fix `13e8acb` | User had to ask *"is the repository 100% healthy?"* |
| 7 | Unbounded recursive search over `~/.cargo/registry` stalled a worker | `092b4c29` | Task had to be killed |
| 8 | Requested model tier not always the model that actually ran (T1 workers reported running on `inherit`) | G2 workers (inferred) | Token budget unclear; fallback log stayed empty |
| 9 | **Claims ahead of evidence.** The methodology doc and CHANGELOG claim "zero stuck keys" and production quality, while every hardware row in `06-qa-and-validation.md` is still *Pending Test*. v0.1.0/v0.2.0 shipped before any physical QA. | docs vs. QA matrix | Overstated quality |
| 10 | PowerShell writes corrupted UTF-8 (`�?"` in `model-fallback-log.md`, `?` instead of emoji in CHANGELOG) | files on `main` | Cosmetic, recurring |
| 11 | Release zips committed to git (`dist/*.zip`) | `git ls-files dist` | Repo bloat |
| 12 | Large batched commits (e.g. "M6 & M7", "M3 & M4") | git log | Harder review/rollback |
| 13 | The `magicbridge-worker` subagent was defined only inside a chat, so it couldn't be reused | `ef69a3e5` | Re-defined per session |

## 4. User corrections → rules
| User said (paraphrased) | Rule adopted |
|---|---|
| "Can you be the sole one committing and avoid merge conflicts?" | Sole Committer Protocol (`21ca1fb`) |
| "Dynamic model selection by complexity; fallback + retrospective review if tokens run out" | Tiers T1–T3, fallback order, `model-fallback-log.md`, audit task |
| "No dependent tasks in parallel" | Dependency/file-ownership analysis before spawning |
| "Leave hardware testing for when I'm at the PC; I need a step-by-step QA process" | Separate headless work from hardware-in-the-loop QA; QA matrix + in-app QA tool |
| "Don't these changes deserve a new release? Define a versioning method" | `release-methodology.md`, SemVer, version single source of truth |
| "Manage multiple layouts and custom ones as profiles; commit regularly; make it public" | Keymaps as runtime data (profiles); regular pushes |
| "Why did the command get stuck? Is the repo healthy?" | Never ignore hangs: find the root cause and add timeouts to tests |
| "I've installed Rust at system level, try again" | Refresh PATH from the registry after installs |

## 5. Metrics
- 26 commits · 10 milestones · 2 releases · ~160 automated tests (last reported) · agent binary ≈ 534 KB.
- Hardware QA: **0 / 12** rows passed (all Pending). Budget tests: latency, soak and stuck-key tests are automated and passing.

## 6. Open risks / follow-ups for this repo
- [ ] Run the hardware QA matrix (QA-HW-01…12, QA-SYS-01…05) on a physical A1843 and record the evidence. _(Tooling ready in Settings Hardware QA tab; pending physical confirmation)_
- [x] Tone down unverified claims in `multi-agent-methodology.md` / README until QA passes. _(Completed: claims aligned with automated invariants; hardware QA labeled pending)_
- [x] Remove `dist/*.zip` from git (keep them in GitHub Releases) and add `/dist` to `.gitignore`. _(Completed: dist/ clean and gitignored)_
- [x] Fix the encoding corruption in `docs/model-fallback-log.md` and `CHANGELOG.md`. _(Completed: UTF-8 encoding verified clean)_
- [x] Verify which model actually runs per tier, and log any mismatches in the fallback log. _(Completed: logged in docs/model-fallback-log.md)_

## 7. Generic lessons (promoted to the kit)
1. Put unknowns into an M0 spike with a probe tool. Never design around guesses.
2. Design module boundaries for parallel work. Interface changes run first and alone.
3. One lead commits; workers run in branch workspaces and return structured reports.
4. The lead re-runs the full gate on the **integrated** tree. Worker gates aren't enough.
5. Every worker prompt includes an environment preamble (toolchain PATH) and the exact files it owns.
6. Declare dependencies/features in the same change that uses them, and use `--package` in workspaces.
7. Tests that touch global OS objects need unique names, readiness signals and timeouts. Never rely on sleep.
8. Scope searches to the workspace; never recurse into dependency caches.
9. Claims follow evidence: QA stays Pending until a human confirms, and release notes list what's still Pending.
10. Force UTF-8 when writing from PowerShell; don't commit build artifacts.
11. Persist agent definitions as files (plugin/workspace), not inside a chat.
12. Run a retrospective after every major milestone and promote lessons into the kit.
