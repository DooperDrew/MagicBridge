//! Daemon state management (config, remapping status, battery, active profile).

use crate::ipc::DaemonStatus;
use magicbridge_core::{CompiledMappingTable, Config, TopRowMode};
use std::collections::HashSet;
use std::path::PathBuf;

/// Runtime daemon state.
#[derive(Debug, Clone)]
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
    /// Tracked paths of currently connected Apple keyboard HID collections.
    pub connected_devices: HashSet<String>,
    /// Whether game mode auto-pause is currently active (F9.2).
    pub game_mode_active: bool,
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
            connected_devices: HashSet::new(),
            game_mode_active: false,
        }
    }

    /// Records the arrival of an Apple keyboard interface.
    ///
    /// Returns `true` if the overall connection status transitioned from disconnected to connected.
    pub fn on_device_arrival(&mut self, path: &str, transport: &str) -> bool {
        let was_connected = self.is_connected;
        self.connected_devices.insert(path.to_ascii_lowercase());
        self.is_connected = true;
        self.transport_name = Some(transport.to_string());
        !was_connected
    }

    /// Records the removal of an Apple keyboard interface.
    ///
    /// Returns `true` if this device was tracked and all Apple keyboards are now disconnected.
    pub fn on_device_removal(&mut self, path: &str) -> bool {
        let was_connected = self.is_connected;
        self.connected_devices.remove(&path.to_ascii_lowercase());
        self.is_connected = !self.connected_devices.is_empty();
        if !self.is_connected {
            self.transport_name = None;
            self.battery_percent = None;
        }
        was_connected && !self.is_connected
    }

    /// Synchronizes the connected state from a list of probed Apple keyboard devices.
    pub fn sync_connected_devices(&mut self, devices: &[(String, String)]) {
        self.connected_devices.clear();
        let mut last_transport = None;
        for (path, transport) in devices {
            self.connected_devices.insert(path.to_ascii_lowercase());
            last_transport = Some(transport.clone());
        }
        self.is_connected = !self.connected_devices.is_empty();
        self.transport_name = last_transport;
        if !self.is_connected {
            self.battery_percent = None;
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
        if !self.enabled {
            parts.push("MagicBridge (Paused)".to_string());
        } else if self.game_mode_active {
            parts.push("MagicBridge (Game Mode Paused)".to_string());
        } else {
            parts.push("MagicBridge".to_string());
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

    /// Compiles the currently active profile into an immutable [`CompiledMappingTable`].
    #[must_use]
    pub fn compile_active_profile(&self) -> CompiledMappingTable {
        if let Some(profile) = self.config.active_profile() {
            magicbridge_core::compile_profile(profile)
        } else {
            CompiledMappingTable::default()
        }
    }

    /// Returns a snapshot of daemon status for IPC or UI queries.
    #[must_use]
    pub fn status(&self) -> DaemonStatus {
        DaemonStatus {
            agent_running: true,
            connected_keyboards: self.connected_devices.len(),
            is_connected: self.is_connected,
            transport: self.transport_name.clone(),
            battery_percent: self.battery_percent,
            active_profile: self.active_profile_name().to_string(),
            top_row_mode: self.active_top_row_mode(),
            enabled: self.enabled,
            paused: !self.enabled || self.game_mode_active,
            game_mode_active: self.game_mode_active,
        }
    }

    /// Persists configuration to disk if a config path is configured.
    pub fn save_config(&self) {
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

    #[test]
    fn daemon_state_device_arrival_and_removal() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);

        let dev1 = r"\\?\hid#vid_05ac&pid_026c&mi_01&col01#8&2fa5df45&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        let dev2 = r"\\?\hid#vid_05ac&pid_026c&mi_01&col02#8&2fa5df45&0&0001#{4d1e55b2-f16f-11cf-88cb-001111000030}";

        // Initial state
        assert!(!state.is_connected);
        assert_eq!(state.transport_name, None);
        assert_eq!(state.connected_devices.len(), 0);

        // Arrival 1: transitions to connected
        let transitioned = state.on_device_arrival(dev1, "USB");
        assert!(transitioned);
        assert!(state.is_connected);
        assert_eq!(state.transport_name.as_deref(), Some("USB"));
        assert_eq!(state.connected_devices.len(), 1);

        // Arrival 2: same device collection, does not transition disconnected -> connected again
        let transitioned2 = state.on_device_arrival(dev2, "USB");
        assert!(!transitioned2);
        assert!(state.is_connected);
        assert_eq!(state.connected_devices.len(), 2);

        // Removal 1: one remains, still connected
        let disconnected = state.on_device_removal(dev1);
        assert!(!disconnected);
        assert!(state.is_connected);
        assert_eq!(state.connected_devices.len(), 1);

        // Removal 2: all removed, transitions to disconnected
        let disconnected2 = state.on_device_removal(dev2);
        assert!(disconnected2);
        assert!(!state.is_connected);
        assert_eq!(state.transport_name, None);
        assert_eq!(state.battery_percent, None);
        assert_eq!(state.connected_devices.len(), 0);
    }

    #[test]
    fn daemon_state_sync_devices() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);

        let devices = vec![
            (
                r"\\?\hid#vid_05ac&pid_026c&mi_01&col01#123#{4d1e55b2-f16f-11cf-88cb-001111000030}"
                    .to_string(),
                "USB".to_string(),
            ),
            (
                r"\\?\hid#vid_05ac&pid_026c&mi_01&col02#123#{4d1e55b2-f16f-11cf-88cb-001111000030}"
                    .to_string(),
                "USB".to_string(),
            ),
        ];

        state.sync_connected_devices(&devices);
        assert!(state.is_connected);
        assert_eq!(state.transport_name.as_deref(), Some("USB"));
        assert_eq!(state.connected_devices.len(), 2);

        // Sync with empty
        state.sync_connected_devices(&[]);
        assert!(!state.is_connected);
        assert_eq!(state.transport_name, None);
        assert_eq!(state.connected_devices.len(), 0);
    }

    #[test]
    fn daemon_state_compile_profile_and_status() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);
        state.is_connected = true;
        state.transport_name = Some("Bluetooth".to_string());
        state.battery_percent = Some(88);

        // Compile table
        let table = state.compile_active_profile();
        assert_eq!(table.fn_vk(), Some(magicbridge_core::VK_RMENU));

        // Status snapshot
        let status = state.status();
        assert!(status.agent_running);
        assert!(status.is_connected);
        assert_eq!(status.transport.as_deref(), Some("Bluetooth"));
        assert_eq!(status.battery_percent, Some(88));
        assert_eq!(status.active_profile, "Default (IT)");
        assert_eq!(status.top_row_mode, TopRowMode::MediaKeysFirst);
        assert!(status.enabled);
        assert!(!status.paused);
        assert!(!status.game_mode_active);
    }

    #[test]
    fn daemon_state_game_mode_tooltip_and_status() {
        let config = Config::default();
        let mut state = DaemonState::new(config, None);
        state.is_connected = true;

        assert!(!state.game_mode_active);
        assert!(state.format_tooltip().starts_with("MagicBridge —"));

        state.game_mode_active = true;
        assert!(
            state
                .format_tooltip()
                .starts_with("MagicBridge (Game Mode Paused) —")
        );

        let status = state.status();
        assert!(status.game_mode_active);
        assert!(status.paused);
    }
}
