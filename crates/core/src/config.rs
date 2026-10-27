//! MagicBridge configuration schema and persistence.
//!
//! Defines the configuration data models, named profiles, key remappings,
//! TOML serialization/deserialization, and atomic file persistence.
//!
//! This module is 100% safe Rust (`#![forbid(unsafe_code)]`) with zero Win32
//! dependencies (NFR-Q1).

use core::fmt;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Physical and language keyboard layout (F7.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardLayout {
    /// Italian ISO layout (standard Apple A1843 IT).
    #[default]
    #[serde(alias = "iso_it", alias = "italian")]
    IsoItalian,
    /// US ANSI layout (standard Apple A1843 US).
    #[serde(alias = "ansi", alias = "us", alias = "english_us")]
    AnsiUs,
    /// UK ISO layout.
    #[serde(alias = "iso_en", alias = "uk", alias = "british")]
    IsoUk,
    /// German ISO layout.
    #[serde(alias = "iso_de", alias = "german")]
    IsoGerman,
    /// French ISO layout (AZERTY).
    #[serde(alias = "iso_fr", alias = "french")]
    IsoFrench,
    /// Extensible custom or alternative layout label.
    Custom(String),
}

impl fmt::Display for KeyboardLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsoItalian => f.write_str("ISO Italian"),
            Self::AnsiUs => f.write_str("ANSI US"),
            Self::IsoUk => f.write_str("ISO UK"),
            Self::IsoGerman => f.write_str("ISO German"),
            Self::IsoFrench => f.write_str("ISO French"),
            Self::Custom(name) => write!(f, "Custom ({name})"),
        }
    }
}

/// Modifier layout preset (F3.1, F3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierPreset {
    /// Windows layout: Cmd -> Alt, Option -> Win (left and right independently).
    #[default]
    #[serde(alias = "windows", alias = "WindowsLayout")]
    WindowsLayout,
    /// Mac layout: no swap (Cmd stays Win/Meta, Option stays Alt).
    #[serde(alias = "mac", alias = "MacLayout")]
    MacLayout,
    /// Custom individual modifier remappings.
    #[serde(alias = "custom", alias = "Custom")]
    Custom,
}

impl fmt::Display for ModifierPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowsLayout => f.write_str("Windows layout"),
            Self::MacLayout => f.write_str("Mac layout"),
            Self::Custom => f.write_str("Custom"),
        }
    }
}

/// Top row behaviour mode: media keys vs. F-keys (F2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopRowMode {
    /// Media keys first: press = media, Fn+press = F-key.
    #[default]
    #[serde(alias = "media_first", alias = "media", alias = "MediaKeysFirst")]
    MediaKeysFirst,
    /// F-keys first: press = F-key, Fn+press = media.
    #[serde(
        alias = "fkeys_first",
        alias = "f_keys_first",
        alias = "fkeys",
        alias = "FKeysFirst"
    )]
    FKeysFirst,
}

impl fmt::Display for TopRowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediaKeysFirst => f.write_str("Media keys first"),
            Self::FKeysFirst => f.write_str("F-keys first"),
        }
    }
}

/// User-configurable Fn trigger key (F1.1, F1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FnKeyChoice {
    /// Right Option (VK_RMENU) — default for A1843.
    #[default]
    #[serde(alias = "right_alt", alias = "rmenu", alias = "RightOption")]
    RightOption,
    /// Right Control (VK_RCONTROL).
    #[serde(alias = "right_ctrl", alias = "rcontrol", alias = "RightControl")]
    RightControl,
    /// F19 key (VK_F19).
    #[serde(alias = "F19")]
    F19,
    /// Fn layer disabled.
    #[serde(alias = "none", alias = "Disabled")]
    Disabled,
    /// Arbitrary virtual key code.
    Other(u16),
}

impl fmt::Display for FnKeyChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RightOption => f.write_str("Right Option"),
            Self::RightControl => f.write_str("Right Control"),
            Self::F19 => f.write_str("F19"),
            Self::Disabled => f.write_str("Disabled"),
            Self::Other(vk) => write!(f, "Other (0x{vk:04X})"),
        }
    }
}

/// Destination action for a remapped key (F5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyAction {
    /// Destination key name or VK identifier (e.g. "PrintScreen", "ScrollLock", "Pause", "Insert").
    Key(String),
    /// Custom action (executable/URL, media action, or key combination).
    Custom(CustomAction),
}

impl KeyAction {
    /// Creates a key remap action targeting the given key name.
    #[must_use]
    pub fn key(target: impl Into<String>) -> Self {
        Self::Key(target.into())
    }

    /// Returns the target key name if this action is a standard key remap.
    #[must_use]
    pub fn as_key(&self) -> Option<&str> {
        match self {
            Self::Key(k) => Some(k.as_str()),
            Self::Custom(_) => None,
        }
    }
}

impl From<&str> for KeyAction {
    fn from(s: &str) -> Self {
        Self::Key(s.to_string())
    }
}

impl From<String> for KeyAction {
    fn from(s: String) -> Self {
        Self::Key(s)
    }
}

impl fmt::Display for KeyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(k) => f.write_str(k),
            Self::Custom(CustomAction::Launch { command }) => write!(f, "Launch: {command}"),
            Self::Custom(CustomAction::OpenUrl { url }) => write!(f, "OpenUrl: {url}"),
            Self::Custom(CustomAction::Media { command }) => write!(f, "Media: {command}"),
            Self::Custom(CustomAction::Shortcut { keys }) => {
                write!(f, "Shortcut: {}", keys.join("+"))
            }
        }
    }
}

/// Custom actions for keys such as F13–F19 or Clear (F5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CustomAction {
    /// Launch an external program, file or URL.
    Launch { command: String },
    /// Open a URL or document via the default shell handler (ShellExecuteW).
    #[serde(alias = "open_url", alias = "url")]
    OpenUrl { url: String },
    /// Execute a media action.
    Media { command: String },
    /// Send a key combination.
    Shortcut { keys: Vec<String> },
}

/// A named configuration profile (F7.7).
///
/// Holds the keyboard physical/language layout, modifier preset, top-row mode,
/// Fn key choice, and per-key normal and Fn-layer mappings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedProfile {
    /// Profile display name.
    pub name: String,
    /// Active keyboard layout for visual display and key identification.
    pub layout: KeyboardLayout,
    /// Modifier preset (Windows, Mac, Custom).
    #[serde(default)]
    pub modifier_preset: ModifierPreset,
    /// Top-row behaviour mode (Media keys first vs. F-keys first).
    #[serde(default)]
    pub top_row_mode: TopRowMode,
    /// Configured Fn trigger key.
    #[serde(default)]
    pub fn_key: FnKeyChoice,
    /// Normal layer key remappings (e.g. "F13" -> "PrintScreen").
    #[serde(default)]
    pub mappings: BTreeMap<String, KeyAction>,
    /// Fn layer key remappings (e.g. "Left" -> "Home", "Backspace" -> "Delete").
    #[serde(default)]
    pub fn_mappings: BTreeMap<String, KeyAction>,
}

impl NamedProfile {
    /// Creates a new profile with the given name, layout, and default options.
    #[must_use]
    pub fn new(name: impl Into<String>, layout: KeyboardLayout) -> Self {
        Self {
            name: name.into(),
            layout,
            modifier_preset: ModifierPreset::default(),
            top_row_mode: TopRowMode::default(),
            fn_key: FnKeyChoice::default(),
            mappings: BTreeMap::new(),
            fn_mappings: BTreeMap::new(),
        }
    }

    /// Creates the standard default Italian ISO profile with Windows modifier layout,
    /// Right Option as Fn, Media keys first, missing key remaps (F13->PrtSc, F14->ScrLk, F15->Pause),
    /// and standard Fn combos (F1.2, F4.1).
    #[must_use]
    pub fn default_it() -> Self {
        let mut mappings = BTreeMap::new();
        // F4.1 missing Windows keys defaults
        mappings.insert("F13".to_string(), KeyAction::key("PrintScreen"));
        mappings.insert("F14".to_string(), KeyAction::key("ScrollLock"));
        mappings.insert("F15".to_string(), KeyAction::key("Pause"));

        let mut fn_mappings = BTreeMap::new();
        // F1.2 default Fn combos
        fn_mappings.insert("Left".to_string(), KeyAction::key("Home"));
        fn_mappings.insert("Right".to_string(), KeyAction::key("End"));
        fn_mappings.insert("Up".to_string(), KeyAction::key("PageUp"));
        fn_mappings.insert("Down".to_string(), KeyAction::key("PageDown"));
        fn_mappings.insert("Backspace".to_string(), KeyAction::key("Delete"));
        fn_mappings.insert("Enter".to_string(), KeyAction::key("Insert"));

        Self {
            name: "Default (IT)".to_string(),
            layout: KeyboardLayout::IsoItalian,
            modifier_preset: ModifierPreset::WindowsLayout,
            top_row_mode: TopRowMode::MediaKeysFirst,
            fn_key: FnKeyChoice::RightOption,
            mappings,
            fn_mappings,
        }
    }

    /// Creates the standard default US ANSI profile.
    #[must_use]
    pub fn default_us() -> Self {
        let mut profile = Self::default_it();
        profile.name = "Default (US)".to_string();
        profile.layout = KeyboardLayout::AnsiUs;
        profile
    }
}

/// Global MagicBridge configuration root (F7.4, F7.6, F7.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Name of the currently active profile.
    #[serde(default = "default_active_profile")]
    pub active_profile: String,
    /// Whether the agent automatically launches at Windows user log-in (F8.1).
    #[serde(default)]
    pub autostart: bool,
    /// Interval in minutes between battery level queries (F6.3).
    #[serde(default = "default_battery_poll_minutes")]
    pub battery_poll_minutes: u32,
    /// Percentage threshold for low-battery notification (F6.2).
    #[serde(default = "default_battery_threshold")]
    pub low_battery_threshold_percent: u8,
    /// Whether dynamic battery percentage badge on tray icon is enabled (IMP-04).
    #[serde(default = "default_battery_tray_badge")]
    pub battery_tray_badge: bool,
    /// UI language code, e.g. "en" or "it" (F7.3).
    #[serde(default = "default_ui_language")]
    pub ui_language: String,
    /// Global pause hotkey shortcut, e.g. "F19" or "Ctrl+Alt+F12" (F9.1).
    #[serde(default)]
    pub pause_hotkey: Option<String>,
    /// Whether debug logging to disk is enabled (NFR-Q2).
    #[serde(default)]
    pub debug_logging: bool,
    /// Whether game mode auto-pause is enabled (F9.2).
    #[serde(default)]
    pub game_mode_enabled: bool,
    /// List of executable names (e.g. ["game.exe", "steam.exe"]) that trigger game mode auto-pause (F9.2).
    #[serde(default)]
    pub game_mode_processes: Vec<String>,
    /// Named profiles map indexed by profile name (F7.7).
    #[serde(default)]
    pub profiles: BTreeMap<String, NamedProfile>,
}

fn default_active_profile() -> String {
    "Default (IT)".to_string()
}

const fn default_battery_poll_minutes() -> u32 {
    10
}

const fn default_battery_threshold() -> u8 {
    15
}

const fn default_battery_tray_badge() -> bool {
    true
}

fn default_ui_language() -> String {
    "en".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let default_profile = NamedProfile::default_it();
        let active_name = default_profile.name.clone();
        let mut profiles = BTreeMap::new();
        profiles.insert(active_name.clone(), default_profile);

        Self {
            active_profile: active_name,
            autostart: false,
            battery_poll_minutes: default_battery_poll_minutes(),
            low_battery_threshold_percent: default_battery_threshold(),
            battery_tray_badge: default_battery_tray_badge(),
            ui_language: default_ui_language(),
            pause_hotkey: None,
            debug_logging: false,
            game_mode_enabled: false,
            game_mode_processes: Vec::new(),
            profiles,
        }
    }
}

impl Config {
    /// Returns `true` if `exe_name` matches any configured game mode process (case-insensitive).
    #[must_use]
    pub fn is_game_process(&self, exe_name: &str) -> bool {
        let trimmed_exe = exe_name.trim();
        self.game_mode_processes.iter().any(|p| {
            let p_trimmed = p.trim();
            p_trimmed.eq_ignore_ascii_case(trimmed_exe)
                || (p_trimmed.ends_with(".exe")
                    && p_trimmed[..p_trimmed.len() - 4].eq_ignore_ascii_case(trimmed_exe))
                || (trimmed_exe.ends_with(".exe")
                    && trimmed_exe[..trimmed_exe.len() - 4].eq_ignore_ascii_case(p_trimmed))
        })
    }
    /// Returns a reference to the active profile, if it exists in `profiles`.
    #[must_use]
    pub fn active_profile(&self) -> Option<&NamedProfile> {
        self.profiles.get(&self.active_profile)
    }

    /// Returns a mutable reference to the active profile, if it exists in `profiles`.
    #[must_use]
    pub fn active_profile_mut(&mut self) -> Option<&mut NamedProfile> {
        self.profiles.get_mut(&self.active_profile)
    }

    /// Sets the active profile by name.
    ///
    /// Returns `Err(ConfigError::ProfileNotFound)` if no profile with that name exists.
    pub fn set_active_profile(&mut self, name: &str) -> Result<(), ConfigError> {
        if self.profiles.contains_key(name) {
            self.active_profile = name.to_string();
            Ok(())
        } else {
            Err(ConfigError::ProfileNotFound(name.to_string()))
        }
    }

    /// Adds or updates a profile in the configuration.
    ///
    /// Returns the previous profile with that name if one existed.
    pub fn add_profile(&mut self, profile: NamedProfile) -> Option<NamedProfile> {
        self.profiles.insert(profile.name.clone(), profile)
    }

    /// Duplicates an existing profile under a new name.
    ///
    /// Returns a mutable reference to the newly created profile.
    pub fn duplicate_profile(
        &mut self,
        source_name: &str,
        new_name: impl Into<String>,
    ) -> Result<&mut NamedProfile, ConfigError> {
        let new_name = new_name.into();
        if self.profiles.contains_key(&new_name) {
            return Err(ConfigError::ProfileAlreadyExists(new_name));
        }

        let source = self
            .profiles
            .get(source_name)
            .ok_or_else(|| ConfigError::ProfileNotFound(source_name.to_string()))?;

        let mut cloned = source.clone();
        cloned.name = new_name.clone();
        self.profiles.insert(new_name.clone(), cloned);
        Ok(self
            .profiles
            .get_mut(&new_name)
            .expect("profile was just inserted"))
    }

    /// Renames an existing profile.
    ///
    /// If the profile being renamed is currently active, `active_profile` is updated accordingly.
    pub fn rename_profile(
        &mut self,
        old_name: &str,
        new_name: impl Into<String>,
    ) -> Result<(), ConfigError> {
        let new_name = new_name.into();
        if old_name == new_name {
            return Ok(());
        }

        if self.profiles.contains_key(&new_name) {
            return Err(ConfigError::ProfileAlreadyExists(new_name));
        }

        let mut profile = self
            .profiles
            .remove(old_name)
            .ok_or_else(|| ConfigError::ProfileNotFound(old_name.to_string()))?;

        profile.name = new_name.clone();
        self.profiles.insert(new_name.clone(), profile);

        if self.active_profile == old_name {
            self.active_profile = new_name;
        }

        Ok(())
    }

    /// Deletes a profile by name.
    ///
    /// Safety rules:
    /// - The active profile cannot be deleted (switch active profile first).
    /// - The last remaining profile cannot be deleted.
    pub fn delete_profile(&mut self, name: &str) -> Result<NamedProfile, ConfigError> {
        if self.profiles.len() <= 1 {
            return Err(ConfigError::CannotDeleteLastProfile);
        }

        if self.active_profile == name {
            return Err(ConfigError::CannotDeleteActiveProfile(name.to_string()));
        }

        self.profiles
            .remove(name)
            .ok_or_else(|| ConfigError::ProfileNotFound(name.to_string()))
    }

    /// Serializes this configuration to a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::TomlSerialize)
    }

    /// Deserializes a configuration from a TOML string.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ConfigError> {
        toml::from_str(toml_str).map_err(ConfigError::TomlDeserialize)
    }

    /// Loads the configuration from the specified file path.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// Saves the configuration to the specified file path atomically.
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let toml_str = self.to_toml_string()?;
        atomic_write(path, toml_str.as_bytes())
    }

    /// Loads the configuration from the specified path, or returns `Self::default()`
    /// if the file does not exist.
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if path.exists() {
            Self::load_from_path(path)
        } else {
            Ok(Self::default())
        }
    }
}

/// Writes `contents` to `path` atomically using a temporary file in the same directory,
/// followed by an atomic rename replacing the destination.
pub fn atomic_write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<(), ConfigError> {
    let path = path.as_ref();
    let parent = path.parent().ok_or_else(|| {
        ConfigError::InvalidPath(format!("Path '{}' has no parent directory", path.display()))
    })?;

    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)?;
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config");
    let temp_name = format!("{file_stem}.tmp.{pid}.{count}");
    let temp_path = parent.join(temp_name);

    {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(contents.as_ref())?;
        file.sync_all()?;
    }

    if let Err(err) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(ConfigError::Io(err));
    }

    Ok(())
}

/// Returns the default MagicBridge configuration directory:
/// `%APPDATA%\MagicBridge` on Windows.
pub fn default_config_dir() -> Result<PathBuf, ConfigError> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        Ok(PathBuf::from(appdata).join("MagicBridge"))
    } else if let Some(userprofile) = std::env::var_os("USERPROFILE") {
        Ok(PathBuf::from(userprofile)
            .join("AppData")
            .join("Roaming")
            .join("MagicBridge"))
    } else if let Some(home) = std::env::var_os("HOME") {
        Ok(PathBuf::from(home).join(".config").join("magicbridge"))
    } else {
        Err(ConfigError::AppDataNotFound)
    }
}

/// Returns the default MagicBridge configuration file path:
/// `%APPDATA%\MagicBridge\config.toml` on Windows.
pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    default_config_dir().map(|dir| dir.join("config.toml"))
}

/// Errors occurring during configuration loading, saving, or profile manipulation.
#[derive(Debug)]
pub enum ConfigError {
    /// File system or I/O error.
    Io(std::io::Error),
    /// Failed to serialize configuration to TOML.
    TomlSerialize(toml::ser::Error),
    /// Failed to deserialize configuration from TOML.
    TomlDeserialize(toml::de::Error),
    /// The specified profile was not found.
    ProfileNotFound(String),
    /// A profile with this name already exists.
    ProfileAlreadyExists(String),
    /// Cannot delete the last remaining profile.
    CannotDeleteLastProfile,
    /// Cannot delete the profile that is currently active.
    CannotDeleteActiveProfile(String),
    /// `%APPDATA%` environment variable not found.
    AppDataNotFound,
    /// Specified path is invalid.
    InvalidPath(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::TomlSerialize(err) => write!(f, "TOML serialization error: {err}"),
            Self::TomlDeserialize(err) => write!(f, "TOML deserialization error: {err}"),
            Self::ProfileNotFound(name) => write!(f, "Profile '{name}' was not found"),
            Self::ProfileAlreadyExists(name) => write!(f, "Profile '{name}' already exists"),
            Self::CannotDeleteLastProfile => write!(f, "Cannot delete the last remaining profile"),
            Self::CannotDeleteActiveProfile(name) => {
                write!(
                    f,
                    "Cannot delete active profile '{name}'; switch profile first"
                )
            }
            Self::AppDataNotFound => {
                write!(
                    f,
                    "AppData environment variable is not defined on this system"
                )
            }
            Self::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::TomlSerialize(err) => Some(err),
            Self::TomlDeserialize(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        Self::TomlSerialize(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::TomlDeserialize(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_valid_it_profile() {
        let config = Config::default();
        assert_eq!(config.active_profile, "Default (IT)");
        assert!(!config.autostart);
        assert_eq!(config.battery_poll_minutes, 10);
        assert_eq!(config.low_battery_threshold_percent, 15);
        assert_eq!(config.ui_language, "en");
        assert_eq!(config.profiles.len(), 1);

        let profile = config
            .active_profile()
            .expect("active profile should exist");
        assert_eq!(profile.name, "Default (IT)");
        assert_eq!(profile.layout, KeyboardLayout::IsoItalian);
        assert_eq!(profile.modifier_preset, ModifierPreset::WindowsLayout);
        assert_eq!(profile.top_row_mode, TopRowMode::MediaKeysFirst);
        assert_eq!(profile.fn_key, FnKeyChoice::RightOption);

        // F4.1 missing Windows keys
        assert_eq!(
            profile.mappings.get("F13"),
            Some(&KeyAction::key("PrintScreen"))
        );
        assert_eq!(
            profile.mappings.get("F14"),
            Some(&KeyAction::key("ScrollLock"))
        );
        assert_eq!(profile.mappings.get("F15"), Some(&KeyAction::key("Pause")));

        // F1.2 Fn combos
        assert_eq!(
            profile.fn_mappings.get("Left"),
            Some(&KeyAction::key("Home"))
        );
        assert_eq!(
            profile.fn_mappings.get("Right"),
            Some(&KeyAction::key("End"))
        );
        assert_eq!(
            profile.fn_mappings.get("Up"),
            Some(&KeyAction::key("PageUp"))
        );
        assert_eq!(
            profile.fn_mappings.get("Down"),
            Some(&KeyAction::key("PageDown"))
        );
        assert_eq!(
            profile.fn_mappings.get("Backspace"),
            Some(&KeyAction::key("Delete"))
        );
        assert_eq!(
            profile.fn_mappings.get("Enter"),
            Some(&KeyAction::key("Insert"))
        );
    }

    #[test]
    fn round_trip_toml_serialization() {
        let original = Config::default();
        let toml_str = original
            .to_toml_string()
            .expect("serialization should succeed");

        assert!(toml_str.contains("active_profile = \"Default (IT)\""));
        assert!(toml_str.contains("layout = \"iso_italian\""));
        assert!(toml_str.contains("modifier_preset = \"windows_layout\""));
        assert!(toml_str.contains("top_row_mode = \"media_keys_first\""));
        assert!(toml_str.contains("fn_key = \"right_option\""));
        assert!(toml_str.contains("F13 = \"PrintScreen\""));
        assert!(toml_str.contains("Backspace = \"Delete\""));

        let deserialized =
            Config::from_toml_str(&toml_str).expect("deserialization should succeed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn profile_creation_and_selection() {
        let mut config = Config::default();
        let us_profile = NamedProfile::default_us();
        config.add_profile(us_profile);

        assert_eq!(config.profiles.len(), 2);
        assert!(config.set_active_profile("Default (US)").is_ok());
        assert_eq!(config.active_profile, "Default (US)");

        let active = config.active_profile().unwrap();
        assert_eq!(active.name, "Default (US)");
        assert_eq!(active.layout, KeyboardLayout::AnsiUs);

        // Unknown profile selection fails
        assert!(config.set_active_profile("NonExistent").is_err());
    }

    #[test]
    fn profile_duplication_and_rename() {
        let mut config = Config::default();

        // Duplicate
        let dup = config
            .duplicate_profile("Default (IT)", "Work (IT)")
            .expect("duplication should succeed");
        dup.mappings
            .insert("F19".to_string(), KeyAction::key("Calc"));

        assert_eq!(config.profiles.len(), 2);
        assert_eq!(
            config
                .profiles
                .get("Work (IT)")
                .unwrap()
                .mappings
                .get("F19"),
            Some(&KeyAction::key("Calc"))
        );
        // Original remains untouched
        assert_eq!(
            config
                .profiles
                .get("Default (IT)")
                .unwrap()
                .mappings
                .get("F19"),
            None
        );

        // Duplicate with existing name fails
        assert!(matches!(
            config.duplicate_profile("Default (IT)", "Work (IT)"),
            Err(ConfigError::ProfileAlreadyExists(_))
        ));

        // Rename
        config
            .set_active_profile("Work (IT)")
            .expect("set active profile");
        config
            .rename_profile("Work (IT)", "Office (IT)")
            .expect("rename should succeed");

        assert_eq!(config.active_profile, "Office (IT)");
        assert!(config.profiles.contains_key("Office (IT)"));
        assert!(!config.profiles.contains_key("Work (IT)"));
    }

    #[test]
    fn profile_deletion_rules() {
        let mut config = Config::default();
        let us = NamedProfile::default_us();
        config.add_profile(us);

        // Cannot delete active profile
        assert!(matches!(
            config.delete_profile("Default (IT)"),
            Err(ConfigError::CannotDeleteActiveProfile(_))
        ));

        // Switch active, then delete old
        config.set_active_profile("Default (US)").unwrap();
        assert!(config.delete_profile("Default (IT)").is_ok());
        assert_eq!(config.profiles.len(), 1);

        // Cannot delete last remaining profile
        assert!(matches!(
            config.delete_profile("Default (US)"),
            Err(ConfigError::CannotDeleteLastProfile)
        ));
    }

    #[test]
    fn round_trip_with_custom_values() {
        let mut config = Config::default();
        let mut custom_profile = NamedProfile::new(
            "Custom Profile",
            KeyboardLayout::Custom("Colemak-DH".to_string()),
        );
        custom_profile.modifier_preset = ModifierPreset::Custom;
        custom_profile.top_row_mode = TopRowMode::FKeysFirst;
        custom_profile.fn_key = FnKeyChoice::Other(0x79); // VK_F10
        custom_profile.mappings.insert(
            "F13".to_string(),
            KeyAction::Custom(CustomAction::Launch {
                command: "calc.exe".to_string(),
            }),
        );
        custom_profile.mappings.insert(
            "F14".to_string(),
            KeyAction::Custom(CustomAction::Media {
                command: "mute".to_string(),
            }),
        );
        custom_profile.mappings.insert(
            "F15".to_string(),
            KeyAction::Custom(CustomAction::Shortcut {
                keys: vec!["Ctrl".to_string(), "Shift".to_string(), "Esc".to_string()],
            }),
        );

        config.add_profile(custom_profile);
        config.set_active_profile("Custom Profile").unwrap();
        config.pause_hotkey = Some("Ctrl+Alt+P".to_string());
        config.debug_logging = true;

        let toml_str = config.to_toml_string().unwrap();
        let loaded = Config::from_toml_str(&toml_str).unwrap();

        assert_eq!(config, loaded);
        let active = loaded.active_profile().unwrap();
        assert_eq!(
            active.layout,
            KeyboardLayout::Custom("Colemak-DH".to_string())
        );
        assert_eq!(active.fn_key, FnKeyChoice::Other(0x79));
        assert_eq!(active.top_row_mode, TopRowMode::FKeysFirst);
    }

    #[test]
    fn file_atomic_save_and_load_round_trip() {
        let temp_dir =
            std::env::temp_dir().join(format!("magicbridge_test_{}", std::process::id()));
        let config_path = temp_dir.join("test_config.toml");

        let config = Config::default();
        config
            .save_to_path(&config_path)
            .expect("atomic save should succeed");
        assert!(config_path.exists());

        let loaded =
            Config::load_from_path(&config_path).expect("loading from path should succeed");
        assert_eq!(config, loaded);

        // Test overwrite with updated config
        let mut modified = loaded;
        modified.autostart = true;
        modified
            .save_to_path(&config_path)
            .expect("overwrite should succeed");

        let reloaded = Config::load_from_path(&config_path).expect("reload should succeed");
        assert_eq!(modified, reloaded);

        // Test load_or_default on nonexistent path
        let nonexistent = temp_dir.join("does_not_exist.toml");
        let def = Config::load_or_default(&nonexistent).expect("load_or_default on nonexistent");
        assert_eq!(def, Config::default());

        // Cleanup
        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn default_config_path_format() {
        let path = default_config_path().expect("default path should resolve");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(r"MagicBridge\config.toml")
                || path_str.ends_with("magicbridge/config.toml")
        );
    }

    #[test]
    fn display_implementations() {
        assert_eq!(KeyboardLayout::IsoItalian.to_string(), "ISO Italian");
        assert_eq!(KeyboardLayout::AnsiUs.to_string(), "ANSI US");
        assert_eq!(
            KeyboardLayout::Custom("Dvorak".into()).to_string(),
            "Custom (Dvorak)"
        );

        assert_eq!(ModifierPreset::WindowsLayout.to_string(), "Windows layout");
        assert_eq!(ModifierPreset::MacLayout.to_string(), "Mac layout");
        assert_eq!(ModifierPreset::Custom.to_string(), "Custom");

        assert_eq!(TopRowMode::MediaKeysFirst.to_string(), "Media keys first");
        assert_eq!(TopRowMode::FKeysFirst.to_string(), "F-keys first");

        assert_eq!(FnKeyChoice::RightOption.to_string(), "Right Option");
        assert_eq!(FnKeyChoice::RightControl.to_string(), "Right Control");
        assert_eq!(FnKeyChoice::F19.to_string(), "F19");
        assert_eq!(FnKeyChoice::Disabled.to_string(), "Disabled");
        assert_eq!(FnKeyChoice::Other(0x70).to_string(), "Other (0x0070)");

        assert_eq!(KeyAction::key("PrtSc").to_string(), "PrtSc");
        assert_eq!(
            KeyAction::Custom(CustomAction::Launch {
                command: "calc".into()
            })
            .to_string(),
            "Launch: calc"
        );
    }

    #[test]
    fn aliases_and_defaults_in_deserialization() {
        let partial_toml = r#"
            active_profile = "Test"

            [profiles.Test]
            name = "Test"
            layout = "iso_it"
            modifier_preset = "windows"
            top_row_mode = "media"
            fn_key = "right_alt"
        "#;

        let config: Config = toml::from_str(partial_toml).expect("should parse aliases");
        assert_eq!(config.active_profile, "Test");
        assert!(!config.autostart);
        assert_eq!(config.battery_poll_minutes, 10);
        assert_eq!(config.low_battery_threshold_percent, 15);
        assert_eq!(config.ui_language, "en");

        let profile = config.profiles.get("Test").unwrap();
        assert_eq!(profile.layout, KeyboardLayout::IsoItalian);
        assert_eq!(profile.modifier_preset, ModifierPreset::WindowsLayout);
        assert_eq!(profile.top_row_mode, TopRowMode::MediaKeysFirst);
        assert_eq!(profile.fn_key, FnKeyChoice::RightOption);
    }

    #[test]
    fn game_mode_config_and_process_matching() {
        let mut config = Config::default();
        assert!(!config.game_mode_enabled);
        assert!(config.game_mode_processes.is_empty());
        assert!(!config.is_game_process("Cyberpunk2077.exe"));

        config.game_mode_enabled = true;
        config.game_mode_processes = vec![
            "Cyberpunk2077.exe".to_string(),
            "cs2".to_string(),
            "eldenring.exe ".to_string(),
        ];

        // Exact match
        assert!(config.is_game_process("Cyberpunk2077.exe"));
        // Case-insensitive match
        assert!(config.is_game_process("cyberpunk2077.exe"));
        // Match with or without .exe extension
        assert!(config.is_game_process("Cyberpunk2077"));
        assert!(config.is_game_process("cs2.exe"));
        assert!(config.is_game_process("CS2"));
        assert!(config.is_game_process("EldenRing.exe"));
        assert!(config.is_game_process("eldenring"));

        // Non-matches
        assert!(!config.is_game_process("notepad.exe"));
        assert!(!config.is_game_process("chrome.exe"));
        assert!(!config.is_game_process("explorer.exe"));

        // Serialization round trip
        let toml_str = config.to_toml_string().expect("should serialize config");
        assert!(toml_str.contains("game_mode_enabled = true"));
        assert!(toml_str.contains("Cyberpunk2077.exe"));

        let loaded = Config::from_toml_str(&toml_str).expect("should deserialize");
        assert!(loaded.game_mode_enabled);
        assert_eq!(loaded.game_mode_processes.len(), 3);
        assert!(loaded.is_game_process("cs2"));
    }
}
