//! MagicBridge Settings GUI Library.
//!
//! Provides the visual keyboard models, localization, views, and application shell.

pub mod app;
pub mod i18n;
pub mod ipc;
pub mod keyboard_model;
pub mod views;

pub use app::{MagicBridgeApp, SettingsTab};
pub use i18n::I18n;
pub use ipc::SettingsIpc;
pub use keyboard_model::{A1843Model, KeyCluster, PhysicalKey};

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::{Config, CustomAction, KeyAction, KeyboardLayout, NamedProfile};

    #[test]
    fn test_settings_initialization_defaults() {
        let i18n = I18n::default();
        assert_eq!(i18n.current_language(), "en");
        assert_eq!(i18n.t("tab.keyboard"), "Keyboard");

        let model = A1843Model::new();
        assert!(model.keys.len() > 100);
    }

    #[test]
    fn test_profile_operations_roundtrip() {
        let mut config = Config::default();
        assert_eq!(config.active_profile, "Default (IT)");

        // Add new profile
        let new_prof = NamedProfile::new("Gaming", KeyboardLayout::AnsiUs);
        config.add_profile(new_prof);
        assert!(config.profiles.contains_key("Gaming"));

        // Switch profile
        config
            .set_active_profile("Gaming")
            .expect("switch succeeds");
        assert_eq!(config.active_profile, "Gaming");

        // Duplicate profile
        config
            .duplicate_profile("Gaming", "Gaming (Custom)")
            .expect("duplicate succeeds");
        assert!(config.profiles.contains_key("Gaming (Custom)"));

        // Rename profile
        config
            .rename_profile("Gaming (Custom)", "Gaming (V2)")
            .expect("rename succeeds");
        assert!(config.profiles.contains_key("Gaming (V2)"));

        // Delete profile
        config
            .delete_profile("Gaming (V2)")
            .expect("delete succeeds");
        assert!(!config.profiles.contains_key("Gaming (V2)"));
    }

    #[test]
    fn test_key_action_serialization_in_profile() {
        let mut profile = NamedProfile::default_it();

        // Custom launch action
        profile.mappings.insert(
            "F16".to_string(),
            KeyAction::Custom(CustomAction::Launch {
                command: "calc.exe".to_string(),
            }),
        );

        // Custom URL action
        profile.mappings.insert(
            "F17".to_string(),
            KeyAction::Custom(CustomAction::OpenUrl {
                url: "https://github.com".to_string(),
            }),
        );

        // Custom media action
        profile.mappings.insert(
            "F18".to_string(),
            KeyAction::Custom(CustomAction::Media {
                command: "volume_up".to_string(),
            }),
        );

        // Custom shortcut action
        profile.mappings.insert(
            "F19".to_string(),
            KeyAction::Custom(CustomAction::Shortcut {
                keys: vec!["Win".to_string(), "Shift".to_string(), "S".to_string()],
            }),
        );

        let toml_str = toml::to_string_pretty(&profile).expect("toml serialization succeeds");
        let deserialized: NamedProfile =
            toml::from_str(&toml_str).expect("toml deserialization succeeds");

        assert_eq!(profile, deserialized);
    }
}
