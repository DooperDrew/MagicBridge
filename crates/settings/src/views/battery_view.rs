//! Battery reporting and power management configuration view.
//!
//! Complies with F6.1, F6.2, F6.3, and docs/04-ux.md.

use egui::{Color32, ProgressBar, Ui};
use magicbridge_agent::DaemonStatus;
use magicbridge_core::Config;

use crate::i18n::I18n;

/// Renders the Battery status and options view.
pub fn show(
    ui: &mut Ui,
    daemon_status: Option<&DaemonStatus>,
    config: &mut Config,
    i18n: &I18n,
) -> bool {
    let mut modified = false;

    ui.group(|ui| {
        ui.heading(i18n.t("battery.title"));
        ui.add_space(6.0);

        // 1. Live Battery Gauge & Percentage (F6.1)
        let battery_pct = daemon_status.and_then(|s| s.battery_percent);
        ui.horizontal(|ui| {
            ui.label(i18n.t("battery.gauge"));

            if let Some(pct) = battery_pct {
                let fraction = (pct as f32) / 100.0;
                let bar_color = if pct <= config.low_battery_threshold_percent {
                    Color32::from_rgb(220, 50, 40) // Red warning
                } else if pct <= 30 {
                    Color32::from_rgb(230, 160, 20) // Orange caution
                } else {
                    Color32::from_rgb(40, 180, 80) // Green healthy
                };

                ui.add(
                    ProgressBar::new(fraction)
                        .show_percentage()
                        .desired_width(220.0)
                        .fill(bar_color),
                );
                ui.label(format!("{pct}%"));
            } else {
                ui.label(i18n.t("battery.unknown"));
            }
        });

        // 2. Charging Status
        ui.horizontal(|ui| {
            ui.label(i18n.t("battery.charging_status"));
            let status_text = if daemon_status.is_some_and(|s| s.is_connected) {
                if let Some(trans) = daemon_status.and_then(|s| s.transport.as_ref()) {
                    if trans.eq_ignore_ascii_case("USB") {
                        i18n.t("battery.charging")
                    } else {
                        i18n.t("battery.discharging")
                    }
                } else {
                    i18n.t("battery.discharging")
                }
            } else {
                i18n.t("battery.unknown")
            };
            ui.label(status_text);
        });

        ui.separator();

        // 3. Low Battery Threshold Slider (F6.2)
        ui.horizontal(|ui| {
            ui.label(i18n.t("battery.threshold"));
            let slider = egui::Slider::new(&mut config.low_battery_threshold_percent, 5..=50)
                .text(i18n.t("battery.threshold"))
                .suffix("%");
            if ui
                .add(slider)
                .on_hover_text(i18n.t("battery.threshold_desc"))
                .changed()
            {
                modified = true;
            }
        });
        ui.indent("thresh_indent", |ui| {
            ui.label(i18n.t("battery.threshold_desc"));
        });

        ui.add_space(4.0);

        // 4. Poll Interval Slider (F6.3)
        ui.horizontal(|ui| {
            ui.label(i18n.t("battery.poll_interval"));
            let slider = egui::Slider::new(&mut config.battery_poll_minutes, 1..=60)
                .text(i18n.t("battery.poll_interval"))
                .suffix(" min");
            if ui
                .add(slider)
                .on_hover_text(i18n.t("battery.poll_interval"))
                .changed()
            {
                modified = true;
            }
        });

        ui.separator();

        // 5. Hardware Info Note (from M0 spike)
        ui.label(i18n.t("battery.info"));
    });

    modified
}
