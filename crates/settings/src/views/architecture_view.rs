//! Multi-Agent Architecture and Methodology Documentation View.
//!
//! Provides an interactive educational tab explaining how MagicBridge was developed
//! using a Hub-and-Spoke multi-agent system with parallel subagents, token optimization,
//! and the Sole Committer protocol.

use crate::i18n::I18n;
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};

/// State for the Architecture & Methodology view.
#[derive(Debug, Clone)]
pub struct ArchitectureViewState {
    /// Active documentation section.
    pub selected_section: ArchitectureSection,
}

impl Default for ArchitectureViewState {
    fn default() -> Self {
        Self {
            selected_section: ArchitectureSection::Overview,
        }
    }
}

/// Sections available in the multi-agent documentation view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureSection {
    /// High-level methodology overview.
    Overview,
    /// Hub-and-Spoke Lead & Worker organization.
    HubAndSpoke,
    /// Dynamic token and complexity tiers (T1, T2, T3).
    ComplexityTiers,
    /// Phase-by-phase parallelization workflows.
    Phases,
    /// Extensibility & contributing guide.
    Extending,
}

impl ArchitectureViewState {
    /// Renders the multi-agent methodology view.
    pub fn ui(&mut self, ui: &mut Ui, _i18n: &I18n) {
        ui.horizontal(|ui| {
            ui.heading("🤖 Multi-Agent Architecture & Methodology");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to(
                    "📖 Full Guide",
                    "https://github.com/DooperDrew/MagicBridge/blob/main/docs/multi-agent-methodology.md",
                );
                ui.hyperlink_to(
                    "🧪 QA Matrix",
                    "https://github.com/DooperDrew/MagicBridge/blob/main/docs/06-qa-and-validation.md",
                );
                ui.hyperlink_to(
                    "🚀 UX Roadmap",
                    "https://github.com/DooperDrew/MagicBridge/blob/main/docs/07-ux-and-improvements.md",
                );
            });
        });

        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "A didactic reference explaining how MagicBridge was developed in parallel \
                 using autonomous AI subagents, dynamic token tiers, and the Sole Committer protocol.",
            )
            .color(Color32::from_rgb(170, 180, 200)),
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Section selector buttons
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut self.selected_section,
                ArchitectureSection::Overview,
                "Overview",
            );
            ui.selectable_value(
                &mut self.selected_section,
                ArchitectureSection::HubAndSpoke,
                "Hub-and-Spoke Model",
            );
            ui.selectable_value(
                &mut self.selected_section,
                ArchitectureSection::ComplexityTiers,
                "Token & Complexity Tiers",
            );
            ui.selectable_value(
                &mut self.selected_section,
                ArchitectureSection::Phases,
                "Parallel Phases",
            );
            ui.selectable_value(
                &mut self.selected_section,
                ArchitectureSection::Extending,
                "Extending Over Time",
            );
        });

        ui.add_space(10.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.selected_section {
                ArchitectureSection::Overview => self.render_overview(ui),
                ArchitectureSection::HubAndSpoke => self.render_hub_and_spoke(ui),
                ArchitectureSection::ComplexityTiers => self.render_complexity_tiers(ui),
                ArchitectureSection::Phases => self.render_phases(ui),
                ArchitectureSection::Extending => self.render_extending(ui),
            });
    }

    fn render_overview(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.strong("Core Problem & Innovation");
            ui.add_space(4.0);
            ui.label(
                "When building native systems utilities with low-level Win32 hooks and strict performance budgets \
                 (0% idle CPU, <0.1ms hook latency), single-agent sessions quickly suffer from context saturation and \
                 token exhaustion. MagicBridge solves this using an autonomous Hub-and-Spoke multi-agent architecture.",
            );
        });

        ui.add_space(8.0);

        ui.columns(3, |cols| {
            cols[0].group(|ui| {
                ui.strong("1. Zero Overlap");
                ui.add_space(4.0);
                ui.label(
                    "Tasks are decomposed into strictly orthogonal packages. Workers operate in isolated git worktrees.",
                );
            });
            cols[1].group(|ui| {
                ui.strong("2. Dynamic Model Tiers");
                ui.add_space(4.0);
                ui.label(
                    "Token costs are reduced by ~80-90% by routing boilerplate to Flash and critical hooks to Pro.",
                );
            });
            cols[2].group(|ui| {
                ui.strong("3. Sole Committer");
                ui.add_space(4.0);
                ui.label(
                    "Only the Lead Orchestrator agent runs git commits, pushes to origin main, or closes issues.",
                );
            });
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.strong("Key Project Metrics Achieved");
            ui.add_space(4.0);
            ui.label("• Test Suite: 128 tests passing across workspace (0 failed)");
            ui.label("• Keystroke Latency: < 1 µs (< 1,000 ns), outperforming budget by >100x");
            ui.label("• Idle Footprint: 0.0% idle CPU and ~680 KB daemon binary size");
            ui.label("• Git Hygiene: 0 merge conflicts, 100% clean verified commits on main");
        });
    }

    fn render_hub_and_spoke(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.strong("The Hub-and-Spoke Pattern");
            ui.add_space(4.0);
            ui.label(
                "In traditional multi-agent systems, agents frequently overstep each other's changes or lock git repositories. \
                 MagicBridge implements a strict hierarchy:",
            );
            ui.add_space(6.0);
            ui.label(
                "• Lead Orchestrator Agent: Analyzes dependencies, spawns worker subagents in isolated worktrees, runs the \
                 quality gate (cargo fmt, clippy, test, release build), commits to main, and publishes GitHub releases.",
            );
            ui.label(
                "• Parallel Worker Subagents (magicbridge-worker): Execute specific work packages, run local validation, \
                 and return structured completion reports. Prohibited from running git commit or modifying remotes.",
            );
        });

        ui.add_space(8.0);
        ui.group(|ui| {
            ui.strong("The Quality Gate Handshake");
            ui.label("Every pull or merge into main must pass the 4-step local gate:");
            ui.monospace("1. cargo fmt --all -- --check");
            ui.monospace("2. cargo clippy --workspace --all-targets -- -D warnings");
            ui.monospace("3. cargo test --workspace");
            ui.monospace("4. cargo build --workspace --release");
        });
    }

    fn render_complexity_tiers(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.strong("Dynamic Token Optimization Tiers");
            ui.add_space(4.0);
            ui.label(
                "Every milestone task is assigned a complexity tier based on risk, required reasoning depth, and safety constraints:",
            );
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.heading(RichText::new("Tier 1 (T1 — Lean)").color(Color32::from_rgb(120, 220, 140)));
            ui.label("• Scope: Pure business logic, serialization schemas, i18n TOML files, packaging scripts, docs.");
            ui.label("• Target Model: flash or flash_lite (~80-90% token savings).");
            ui.label("• Examples: M1.2 (Config schema), M2.1 (Remapping table), M7.1 (Packaging script).");
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.heading(RichText::new("Tier 2 (T2 — Moderate)").color(Color32::from_rgb(100, 180, 240)));
            ui.label("• Scope: UI widgets, standard Win32 message pumps, system tray menus, Named Pipe IPC.");
            ui.label("• Target Model: flash or pro.");
            ui.label("• Examples: M1.1 (Tray & message window), M2.5 (Named Pipe IPC), M5 (Settings GUI).");
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.heading(RichText::new("Tier 3 (T3 — Complex / Critical)").color(Color32::from_rgb(240, 140, 100)));
            ui.label("• Scope: Low-level WH_KEYBOARD_LL hook, zero-allocation timing, raw HID battery reporting, concurrency.");
            ui.label("• Target Model: pro or inherit (Sonnet / Opus).");
            ui.label("• Examples: M2.2 (Input hook & AltGr 0x21D suppression), M4.2 (HID Report 0x90), M6 (Hardening).");
        });
    }

    fn render_phases(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.strong("Empirical Parallel Execution Phases");
            ui.add_space(4.0);
            ui.label("Review how tasks were parallelized across branches throughout development:");
        });

        ui.add_space(8.0);

        ui.collapsing("Phase 1: Milestone 1.1 (Tray) & Milestone 1.3 (Autostart)", |ui| {
            ui.label("• Worker 1 (T2-Agent-M1.1): Developed tray icon, message window, and single-instance mutex.");
            ui.label("• Worker 2 (T1-Autostart-M1.3): Developed HKCU Run key registry autostart manager.");
            ui.label("• Outcome: Zero conflict merge into commit fb16c60; 38 tests passing.");
        });

        ui.collapsing("Phase 2: Milestone 1.4 (PnP Lifecycle) & Milestone 2.1 (Engine)", |ui| {
            ui.label("• Worker 1 (T2-Device-M1.4): Registered RegisterDeviceNotificationW and sleep/resume handling in agent.");
            ui.label("• Worker 2 (T1-Engine-M2.1): Built pure O(1) flat remapping table in core.");
            ui.label("• Outcome: Cross-crate separation allowed 100% parallel development; commit 473221b.");
        });

        ui.collapsing("Phase 3: Milestone 2.2 (Hook) & Milestone 2.5 (Named Pipe IPC)", |ui| {
            ui.label("• Worker 1 (T3-Hook-M2.2): Built WH_KEYBOARD_LL hook, AltGr suppression, and extra info tagging.");
            ui.label("• Worker 2 (T2-IPC-M2.5): Built Named Pipe IPC server and arc-swap atomic table swapping.");
            ui.label("• Outcome: Commit fb3e069; closed Milestone 2.");
        });

        ui.collapsing("Phase 4: Milestone 3 (Top Row) & Milestone 4 (Battery & Actions)", |ui| {
            ui.label("• Worker 1 (T2-TopRow-M3): Implemented F3 Task View, F4 Start, and F7..F12 media toggling.");
            ui.label("• Worker 2 (T3-Battery-M4): Implemented off-thread action worker and Apple HID report 0x90 reader.");
            ui.label("• Outcome: Commit aaddee6; closed Milestones 3 & 4.");
        });

        ui.collapsing("Phase 5: Milestone 6 (Hardening) & Milestone 7 (Packaging)", |ui| {
            ui.label("• Worker 1 (T3-Hardening-M6): Implemented Game Mode auto-pause and 500-cycle soak tests.");
            ui.label("• Worker 2 (T1-Package-M7): Built package-release.ps1 script and user-guide.md.");
            ui.label("• Outcome: Commit 5e343ef; closed all roadmap milestones.");
        });
    }

    fn render_extending(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.strong("Extending This Pattern for Future Tasks");
            ui.add_space(4.0);
            ui.label(
                "When adding new features or QA improvements to MagicBridge, follow this 3-step lifecycle:",
            );
            ui.add_space(6.0);
            ui.label("1. Orthogonality Analysis: Ensure proposed tasks touch distinct files or crates.");
            ui.label("2. Worker Delegation: Assign roles (<Tier>-<Domain>-<Milestone>) using invoke_subagent.");
            ui.label("3. Quality Gate & Commit: Run cargo fmt, clippy, test, and release builds before merging.");
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("For full specifications, consult:");
            ui.hyperlink_to(
                "AGENTS.md",
                "https://github.com/DooperDrew/MagicBridge/blob/main/AGENTS.md",
            );
            ui.label("and");
            ui.hyperlink_to(
                "docs/multi-agent-methodology.md",
                "https://github.com/DooperDrew/MagicBridge/blob/main/docs/multi-agent-methodology.md",
            );
        });
    }
}
