//! Named pipe IPC client wrapper and background agent process lifecycle.
//!
//! Provides communication with `magicbridge-agent` via `\\.\pipe\MagicBridge_Agent_IPC` (F7.5)
//! and agent launch detection ("Agent Not Running" banner with "Start Agent" button).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use magicbridge_agent::{DaemonStatus, IpcClient, IpcRequest};

/// Client controller for communication with the background agent.
#[derive(Debug, Clone)]
pub struct SettingsIpc {
    client: IpcClient,
}

impl Default for SettingsIpc {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsIpc {
    /// Creates a new IPC controller targeting the default agent named pipe.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: IpcClient::default_client(),
        }
    }

    /// Queries the current daemon status. Returns `None` if the agent is not running.
    pub fn query_status(&self) -> Option<DaemonStatus> {
        self.client.get_status().ok()
    }

    /// Sends a request to switch the active profile immediately (F7.5).
    pub fn switch_profile(&self, profile_name: &str) -> Result<(), String> {
        match self.client.send(&IpcRequest::SwitchProfile {
            name: profile_name.to_string(),
        }) {
            Ok(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err(resp
                        .error
                        .unwrap_or_else(|| "Failed to switch profile".to_string()))
                }
            }
            Err(e) => Err(format!("IPC connection failed: {e}")),
        }
    }

    /// Sends a reload command instructing the agent to reload `%APPDATA%\MagicBridge\config.toml`.
    pub fn reload_config(&self) -> Result<(), String> {
        match self.client.send(&IpcRequest::Reload) {
            Ok(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err(resp
                        .error
                        .unwrap_or_else(|| "Failed to reload agent config".to_string()))
                }
            }
            Err(e) => Err(format!("IPC connection failed: {e}")),
        }
    }

    /// Toggles the remapping engine pause state.
    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        match self.client.send(&IpcRequest::Pause { paused }) {
            Ok(resp) => {
                if resp.success {
                    Ok(())
                } else {
                    Err(resp
                        .error
                        .unwrap_or_else(|| "Failed to set pause state".to_string()))
                }
            }
            Err(e) => Err(format!("IPC connection failed: {e}")),
        }
    }

    /// Attempts to locate the `magicbridge-agent.exe` executable on the host system.
    #[must_use]
    pub fn find_agent_exe() -> Option<PathBuf> {
        // 1. Check beside the running settings executable
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(dir) = current_exe.parent()
        {
            let candidate = dir.join("magicbridge-agent.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // 2. Check cargo target build dirs for development
        let dev_candidates = [
            Path::new("target/release/magicbridge-agent.exe"),
            Path::new("target/debug/magicbridge-agent.exe"),
            Path::new("../target/release/magicbridge-agent.exe"),
            Path::new("../target/debug/magicbridge-agent.exe"),
            Path::new("magicbridge-agent.exe"),
        ];

        for c in dev_candidates {
            if c.is_file() {
                if let Ok(canonical) = c.canonicalize() {
                    return Some(canonical);
                }
                return Some(c.to_path_buf());
            }
        }

        None
    }

    /// Launches `magicbridge-agent.exe` in the background.
    pub fn launch_agent() -> Result<(), String> {
        let exe = Self::find_agent_exe().ok_or_else(|| {
            "magicbridge-agent.exe could not be found beside settings executable or in target/"
                .to_string()
        })?;

        Command::new(&exe)
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {e}", exe.display()))?;

        // Give the agent process a moment to initialize its IPC listener
        std::thread::sleep(Duration::from_millis(200));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_agent_exe_does_not_panic() {
        let _ = SettingsIpc::find_agent_exe();
    }
}
