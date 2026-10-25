//! Daemon state management (config, remapping status, battery, active profile).

use magicbridge_core::{Config, TopRowMode};
use std::path::PathBuf;

/// Runtime daemon state.
#[derive(Debug)]
pub struct DaemonState {
    /// Active configuration and profiles.
    pub config: Config,
    /// Path to config file for persistence.
    pub config_path: Option<PathBuf>,
    /// Global remapping enabled flag (F9.1 pause toggle).
    pub enabled: bool,
    /// Latest battery percentage (F6.1), if known.
    pub battery_percent: Option<u8>,
    /// Whether an Apple keyboard is currently connected (NFR-S2).
    pub is_connected: bool,
    /// Transport description ("USB", "Bluetooth", etc.).
    pub transport_name: Option<String>,
}

impl DaemonState {
    /// Creates a new daemon state instance.
    pub fn new(config: Config, config_path: Option<PathBuf>) -> Self {
        Self {
            config,
            config_path,
            enabled: true,
            battery_percent: None,
            is_connected: false,
            transport_name: None,
        }
    }

    /// Toggles the global remapping enabled state.
    pub fn toggle_enabled(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }

    /// Returns the active profile name.
    pub fn active_profile_name(&self) -> &str {
        &self.config.active_profile
    }

    /// Changes the active profile by name and persists config.
    pub fn set_active_profile(&mut self, name: &str) -> bool {
        if self.config.set_active_profile(name).is_ok() {
            self.save_config();
            true
        } else {
            false
        }
    }

    /// Returns the active profile's top-row mode.
    pub fn active_top_row_mode(&self) -> TopRowMode {
        self.config
            .active_profile()
            .map(|p| p.top_row_mode)
            .unwrap_or_default()
    }

    /// Sets the top-row mode for the active profile and persists config.
    pub fn set_top_row_mode(&mut self, mode: TopRowMode) {
        if let Some(profile) = self.config.active_profile_mut() {
            profile.top_row_mode = mode;
            self.save_config();
        }
    }

    /// Toggles autostart configuration and persists config.
    pub fn toggle_autostart(&mut self) -> bool {
        self.config.autostart = !self.config.autostart;
        self.save_config();
        self.config.autostart
    }

    /// Returns the sorted list of profile names.
    pub fn profile_names(&self) -> Vec<String> {
        self.config.profiles.keys().cloned().collect()
    }

    /// Formats the tray tooltip string according to docs/04-ux.md.
    pub fn format_tooltip(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.enabled {
            parts.push("MagicBridge".to_string());
        } else {
            parts.push("MagicBridge (Paused)".to_string());
        }

        if self.is_connected {
            if let Some(ref transport) = self.transport_name {
                parts.push(transport.clone());
            }
            if let Some(pct) = self.battery_percent {
                parts.push(format!("Battery {pct}%"));
            }
        }

        parts.push(format!("Profile: {}", self.active_profile_name()));

        let top_row_str = match self.active_top_row_mode() {
            TopRowMode::MediaKeysFirst => "Media keys first",
            TopRowMode::FKeysFirst => "F-keys first",
        };
        parts.push(top_row_str.to_string());

        let joined = parts.join(" — ");
        // Ensure tooltip fits within Windows 127-char szTip limit.
        if joined.chars().count() > 127 {
            let truncated: String = joined.chars().take(124).collect();
            format!("{truncated}...")
        } else {
            joined
        }
    }

    fn save_config(&self) {
        if let Some(ref path) = self.config_path {
            let _ = self.config.save_to_path(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::NamedProfile;

    #[test]
    fn daemon_state_defaults_and_toggle() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);

        assert!(state.enabled);
        assert_eq!(state.active_profile_name(), "Default (IT)");
        assert_eq!(state.active_top_row_mode(), TopRowMode::MediaKeysFirst);

        // Toggle enabled
        assert!(!state.toggle_enabled());
        assert!(!state.enabled);
        assert!(state.toggle_enabled());
        assert!(state.enabled);
    }

    #[test]
    fn daemon_state_profile_switching() {
        let mut config = Config::default();
        let us_profile = NamedProfile::default_us();
        config.add_profile(us_profile);

        let mut state = DaemonState::new(config, None);
        assert_eq!(state.profile_names().len(), 2);

        assert!(state.set_active_profile("Default (US)"));
        assert_eq!(state.active_profile_name(), "Default (US)");

        assert!(!state.set_active_profile("NonExistentProfile"));
        assert_eq!(state.active_profile_name(), "Default (US)");
    }

    #[test]
    fn daemon_state_top_row_mode_switching() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);

        assert_eq!(state.active_top_row_mode(), TopRowMode::MediaKeysFirst);
        state.set_top_row_mode(TopRowMode::FKeysFirst);
        assert_eq!(state.active_top_row_mode(), TopRowMode::FKeysFirst);
    }

    #[test]
    fn daemon_state_tooltip_formatting() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);

        // Disconnected, enabled
        let tip1 = state.format_tooltip();
        assert_eq!(
            tip1,
            "MagicBridge — Profile: Default (IT) — Media keys first"
        );

        // Disconnected, paused
        state.enabled = false;
        let tip2 = state.format_tooltip();
        assert_eq!(
            tip2,
            "MagicBridge (Paused) — Profile: Default (IT) — Media keys first"
        );

        // Connected Bluetooth with battery 78%
        state.enabled = true;
        state.is_connected = true;
        state.transport_name = Some("Bluetooth".to_string());
        state.battery_percent = Some(78);
        let tip3 = state.format_tooltip();
        assert_eq!(
            tip3,
            "MagicBridge — Bluetooth — Battery 78% — Profile: Default (IT) — Media keys first"
        );
    }
}
