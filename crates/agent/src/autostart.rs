//! Autostart configuration for MagicBridge background agent.
//!
//! Registers/unregisters the agent executable in the Windows registry:
//! `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
//! under the value name `"MagicBridge"`.
//!
//! Per NFR-S1 and F8.1:
//! - Operates per-user in `HKCU`, requiring zero administrative / elevation privileges.
//! - Safe against missing keys and values (idempotent operations).

use std::path::Path;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

#[cfg(test)]
use windows::Win32::System::Registry::RegDeleteKeyW;

/// Default registry subkey for per-user autostart entries.
pub const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Default registry value name for MagicBridge.
pub const APP_VALUE_NAME: &str = "MagicBridge";

/// Errors that can occur during autostart configuration operations.
#[derive(Debug)]
pub enum AutostartError {
    /// Failed to retrieve the path of the currently executing binary.
    CurrentExe(std::io::Error),
    /// The specified executable path is invalid (empty or non-UTF-8).
    InvalidPath(String),
    /// Registry value contained invalid UTF-16 data.
    InvalidEncoding(std::string::FromUtf16Error),
    /// A Win32 registry call failed with an unexpected error code.
    Registry {
        /// Win32 API function that produced the error.
        action: &'static str,
        /// Raw Win32 error code.
        code: u32,
        /// Localized Windows error message.
        message: String,
    },
}

impl AutostartError {
    fn from_win32(action: &'static str, err: WIN32_ERROR) -> Self {
        Self::Registry {
            action,
            code: err.0,
            message: err.to_hresult().message().to_string(),
        }
    }
}

impl std::fmt::Display for AutostartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentExe(err) => {
                write!(f, "failed to determine current executable path: {err}")
            }
            Self::InvalidPath(msg) => {
                write!(f, "invalid executable path: {msg}")
            }
            Self::InvalidEncoding(err) => {
                write!(f, "invalid UTF-16 encoding in registry value: {err}")
            }
            Self::Registry {
                action,
                code,
                message,
            } => {
                write!(
                    f,
                    "registry operation '{action}' failed with code {code} (0x{code:08X}): {message}"
                )
            }
        }
    }
}

impl std::error::Error for AutostartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentExe(err) => Some(err),
            Self::InvalidEncoding(err) => Some(err),
            _ => None,
        }
    }
}

/// RAII wrapper for an open Windows registry key (`HKEY`).
///
/// Ensures `RegCloseKey` is invoked exactly once when the handle drops,
/// preventing registry handle leaks.
struct OwnedKey(HKEY);

impl OwnedKey {
    #[inline]
    fn raw(&self) -> HKEY {
        self.0
    }
}

impl Drop for OwnedKey {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: self.0 is a valid open registry handle owned by this instance.
            // RegCloseKey releases the OS resource and is safe to call.
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
}

/// Encodes a UTF-8 string as a null-terminated UTF-16 vector.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Formats an executable path into a properly quoted Windows command line string.
///
/// If the path is not already wrapped in double quotes, quotes are added to ensure
/// paths containing whitespace are executed correctly by the Windows shell upon logon.
pub fn format_autostart_command(path: &Path) -> Result<String, AutostartError> {
    let path_str = path.to_str().ok_or_else(|| {
        AutostartError::InvalidPath(format!(
            "path contains invalid UTF-8: {}",
            path.to_string_lossy()
        ))
    })?;

    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(AutostartError::InvalidPath(
            "path cannot be empty".to_string(),
        ));
    }

    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("\"{trimmed}\""))
    }
}

/// Opens an existing registry key with the requested permissions.
/// Returns `Ok(None)` if the key does not exist (`ERROR_FILE_NOT_FOUND`).
fn open_existing_key(
    root: HKEY,
    subkey: &str,
    desired_access: REG_SAM_FLAGS,
) -> Result<Option<OwnedKey>, AutostartError> {
    let wide_subkey = to_wide(subkey);
    let mut hkey = HKEY::default();

    // SAFETY: root is a valid root handle (e.g. HKEY_CURRENT_USER).
    // wide_subkey is a null-terminated UTF-16 buffer.
    // hkey is a valid out-pointer for the opened handle.
    let err = unsafe {
        RegOpenKeyExW(
            root,
            PCWSTR::from_raw(wide_subkey.as_ptr()),
            None,
            desired_access,
            &mut hkey,
        )
    };

    if err == ERROR_SUCCESS {
        Ok(Some(OwnedKey(hkey)))
    } else if err == ERROR_FILE_NOT_FOUND {
        Ok(None)
    } else {
        Err(AutostartError::from_win32("RegOpenKeyExW", err))
    }
}

/// Opens an existing registry key or creates it if absent.
fn open_or_create_key(
    root: HKEY,
    subkey: &str,
    desired_access: REG_SAM_FLAGS,
) -> Result<OwnedKey, AutostartError> {
    if let Some(key) = open_existing_key(root, subkey, desired_access)? {
        return Ok(key);
    }

    let wide_subkey = to_wide(subkey);
    let mut hkey = HKEY::default();

    // SAFETY: RegCreateKeyW creates or opens the subkey. root is valid,
    // wide_subkey is null-terminated, and hkey receives the handle.
    let err = unsafe { RegCreateKeyW(root, PCWSTR::from_raw(wide_subkey.as_ptr()), &mut hkey) };

    if err == ERROR_SUCCESS {
        Ok(OwnedKey(hkey))
    } else {
        Err(AutostartError::from_win32("RegCreateKeyW", err))
    }
}

/// Reads the raw string command registered for `value_name` at `subkey` under `root`.
/// Returns `Ok(None)` if the subkey or value does not exist.
pub fn get_autostart_command_at(
    root: HKEY,
    subkey: &str,
    value_name: &str,
) -> Result<Option<String>, AutostartError> {
    let key = match open_existing_key(root, subkey, KEY_QUERY_VALUE)? {
        Some(k) => k,
        None => return Ok(None),
    };

    let wide_name = to_wide(value_name);
    let mut data_type = REG_VALUE_TYPE::default();
    let mut byte_len: u32 = 0;

    // First query the required buffer size.
    // SAFETY: key.raw() is a valid open handle. wide_name is null-terminated.
    // byte_len will receive the required buffer length in bytes.
    let err = unsafe {
        RegQueryValueExW(
            key.raw(),
            PCWSTR::from_raw(wide_name.as_ptr()),
            None,
            Some(&mut data_type),
            None,
            Some(&mut byte_len),
        )
    };

    if err == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if err != ERROR_SUCCESS {
        return Err(AutostartError::from_win32("RegQueryValueExW", err));
    }

    if byte_len == 0 {
        return Ok(Some(String::new()));
    }

    // Allocate buffer for UTF-16 units.
    let u16_len = (byte_len as usize).div_ceil(2);
    let mut buffer: Vec<u16> = vec![0u16; u16_len];

    // SAFETY: buffer is allocated to at least byte_len bytes and will be filled by the API.
    let read_err = unsafe {
        RegQueryValueExW(
            key.raw(),
            PCWSTR::from_raw(wide_name.as_ptr()),
            None,
            Some(&mut data_type),
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut byte_len),
        )
    };

    if read_err != ERROR_SUCCESS {
        return Err(AutostartError::from_win32("RegQueryValueExW", read_err));
    }

    let actual_u16_len = (byte_len as usize) / 2;
    let slice = &buffer[..actual_u16_len];

    // Strip trailing null characters if present.
    let null_trimmed = match slice.iter().position(|&c| c == 0) {
        Some(pos) => &slice[..pos],
        None => slice,
    };

    let string = String::from_utf16(null_trimmed).map_err(AutostartError::InvalidEncoding)?;
    Ok(Some(string))
}

/// Checks whether an autostart entry exists at `subkey` under `root`.
pub fn is_autostart_enabled_at(
    root: HKEY,
    subkey: &str,
    value_name: &str,
) -> Result<bool, AutostartError> {
    Ok(get_autostart_command_at(root, subkey, value_name)?.is_some())
}

/// Enables autostart by writing `"<exe_path>"` into `subkey` under `root`.
pub fn enable_autostart_at(
    root: HKEY,
    subkey: &str,
    value_name: &str,
    exe_path: &Path,
) -> Result<(), AutostartError> {
    let command = format_autostart_command(exe_path)?;
    let key = open_or_create_key(root, subkey, KEY_SET_VALUE)?;
    let wide_name = to_wide(value_name);
    let wide_command = to_wide(&command);

    // SAFETY: wide_command contains valid u16 values including the null terminator.
    // Casting &[u16] as a byte slice with size len * 2 is memory-safe.
    let byte_slice: &[u8] = unsafe {
        std::slice::from_raw_parts(
            wide_command.as_ptr() as *const u8,
            wide_command.len() * std::mem::size_of::<u16>(),
        )
    };

    // SAFETY: key.raw() is a valid open handle with KEY_SET_VALUE.
    // wide_name is a null-terminated UTF-16 string.
    // byte_slice is valid and points to the null-terminated UTF-16 command string.
    let err = unsafe {
        RegSetValueExW(
            key.raw(),
            PCWSTR::from_raw(wide_name.as_ptr()),
            None,
            REG_SZ,
            Some(byte_slice),
        )
    };

    if err == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(AutostartError::from_win32("RegSetValueExW", err))
    }
}

/// Disables autostart by deleting `value_name` from `subkey` under `root`.
/// Idempotent: returns `Ok(())` if the key or value is already absent.
pub fn disable_autostart_at(
    root: HKEY,
    subkey: &str,
    value_name: &str,
) -> Result<(), AutostartError> {
    let key = match open_existing_key(root, subkey, KEY_SET_VALUE)? {
        Some(k) => k,
        None => return Ok(()),
    };

    let wide_name = to_wide(value_name);

    // SAFETY: key.raw() is an open handle with KEY_SET_VALUE.
    // wide_name is a null-terminated UTF-16 string.
    let err = unsafe { RegDeleteValueW(key.raw(), PCWSTR::from_raw(wide_name.as_ptr())) };

    if err == ERROR_SUCCESS || err == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(AutostartError::from_win32("RegDeleteValueW", err))
    }
}

// ---------------------------------------------------------------------------
// Public API operating on HKCU\Software\Microsoft\Windows\CurrentVersion\Run
// ---------------------------------------------------------------------------

/// Checks if autostart is currently enabled for MagicBridge in `HKEY_CURRENT_USER\...\Run`.
pub fn is_autostart_enabled() -> Result<bool, AutostartError> {
    is_autostart_enabled_at(HKEY_CURRENT_USER, RUN_KEY_PATH, APP_VALUE_NAME)
}

/// Retrieves the command string currently stored for MagicBridge in `HKEY_CURRENT_USER\...\Run`,
/// or `Ok(None)` if not registered.
pub fn get_autostart_command() -> Result<Option<String>, AutostartError> {
    get_autostart_command_at(HKEY_CURRENT_USER, RUN_KEY_PATH, APP_VALUE_NAME)
}

/// Registers the given executable path in `HKEY_CURRENT_USER\...\Run` under `"MagicBridge"`.
///
/// The path is quoted automatically if not already quoted.
pub fn enable_autostart(exe_path: &Path) -> Result<(), AutostartError> {
    enable_autostart_at(HKEY_CURRENT_USER, RUN_KEY_PATH, APP_VALUE_NAME, exe_path)
}

/// Resolves `std::env::current_exe()` and registers it for autostart.
pub fn enable_current_exe_autostart() -> Result<(), AutostartError> {
    let current_exe = std::env::current_exe().map_err(AutostartError::CurrentExe)?;
    enable_autostart(&current_exe)
}

/// Unregisters MagicBridge from `HKEY_CURRENT_USER\...\Run`.
///
/// If the value is not present or the key does not exist, this is a no-op and returns `Ok(())`.
pub fn disable_autostart() -> Result<(), AutostartError> {
    disable_autostart_at(HKEY_CURRENT_USER, RUN_KEY_PATH, APP_VALUE_NAME)
}

/// Helper function to delete a subkey under `HKEY_CURRENT_USER` (used for test teardown).
#[cfg(test)]
fn delete_test_key(root: HKEY, subkey: &str) {
    let wide = to_wide(subkey);
    // SAFETY: Deleting isolated test subkey during test cleanup.
    unsafe {
        let _ = RegDeleteKeyW(root, PCWSTR::from_raw(wide.as_ptr()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_autostart_command_unquoted() {
        let p = Path::new(r"C:\Program Files\MagicBridge\magicbridge-agent.exe");
        let formatted = format_autostart_command(p).expect("formatting should succeed");
        assert_eq!(
            formatted,
            r#""C:\Program Files\MagicBridge\magicbridge-agent.exe""#
        );
    }

    #[test]
    fn test_format_autostart_command_already_quoted() {
        let p = Path::new(r#""C:\Program Files\MagicBridge\magicbridge-agent.exe""#);
        let formatted = format_autostart_command(p).expect("formatting should succeed");
        assert_eq!(
            formatted,
            r#""C:\Program Files\MagicBridge\magicbridge-agent.exe""#
        );
    }

    #[test]
    fn test_format_autostart_command_no_spaces() {
        let p = Path::new(r"C:\MagicBridge\agent.exe");
        let formatted = format_autostart_command(p).expect("formatting should succeed");
        assert_eq!(formatted, r#""C:\MagicBridge\agent.exe""#);
    }

    #[test]
    fn test_format_autostart_command_empty() {
        let p = Path::new("   ");
        let err = format_autostart_command(p).unwrap_err();
        match err {
            AutostartError::InvalidPath(msg) => assert!(msg.contains("empty")),
            other => panic!("expected InvalidPath, got {other:?}"),
        }
    }

    #[test]
    fn test_autostart_error_display() {
        let err1 = AutostartError::InvalidPath("test path".to_string());
        assert!(err1.to_string().contains("invalid executable path"));

        let err2 = AutostartError::Registry {
            action: "RegOpenKeyExW",
            code: 2,
            message: "The system cannot find the file specified.".to_string(),
        };
        assert!(err2.to_string().contains("RegOpenKeyExW"));
        assert!(err2.to_string().contains("code 2"));
    }

    #[test]
    fn test_current_exe_formatting() {
        let current = std::env::current_exe().expect("should resolve current exe");
        let formatted = format_autostart_command(&current).expect("should format current exe");
        assert!(formatted.starts_with('"'));
        assert!(formatted.ends_with('"'));
    }

    #[test]
    fn test_registry_crud_on_isolated_test_key() {
        let test_subkey = r"Software\MagicBridge\UnitTest_Autostart_Crud";
        let test_val_name = "TestApp";
        let test_exe = PathBuf::from(r"C:\TestPath\magicbridge.exe");

        // Clean up any leftovers from prior runs before starting.
        delete_test_key(HKEY_CURRENT_USER, test_subkey);

        // 1. Initially, key does not exist, so is_enabled should be false.
        let enabled = is_autostart_enabled_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("query on absent key should succeed");
        assert!(!enabled);

        let cmd = get_autostart_command_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("get on absent key should succeed");
        assert_eq!(cmd, None);

        // 2. Enable autostart on this test key.
        enable_autostart_at(HKEY_CURRENT_USER, test_subkey, test_val_name, &test_exe)
            .expect("enabling autostart should succeed");

        // 3. Verify enabled and command matches formatted path.
        let enabled = is_autostart_enabled_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("query after enable should succeed");
        assert!(enabled);

        let cmd = get_autostart_command_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("get after enable should succeed")
            .expect("command should be present");
        assert_eq!(cmd, r#""C:\TestPath\magicbridge.exe""#);

        // 4. Update with a new path.
        let new_exe = PathBuf::from(r"C:\NewPath with spaces\agent.exe");
        enable_autostart_at(HKEY_CURRENT_USER, test_subkey, test_val_name, &new_exe)
            .expect("updating autostart should succeed");

        let cmd2 = get_autostart_command_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("get after update should succeed")
            .expect("command should be present");
        assert_eq!(cmd2, r#""C:\NewPath with spaces\agent.exe""#);

        // 5. Disable autostart.
        disable_autostart_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("disabling autostart should succeed");

        let enabled_after_disable =
            is_autostart_enabled_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
                .expect("query after disable should succeed");
        assert!(!enabled_after_disable);

        // 6. Disable again (idempotency check).
        disable_autostart_at(HKEY_CURRENT_USER, test_subkey, test_val_name)
            .expect("second disable should succeed as a no-op");

        // Cleanup test subkey.
        delete_test_key(HKEY_CURRENT_USER, test_subkey);
        delete_test_key(HKEY_CURRENT_USER, r"Software\MagicBridge");
    }

    #[test]
    fn test_is_autostart_enabled_does_not_panic() {
        // Querying HKCU Run should execute cleanly without panicking.
        let res = is_autostart_enabled();
        assert!(res.is_ok(), "is_autostart_enabled returned error: {res:?}");
    }
}
