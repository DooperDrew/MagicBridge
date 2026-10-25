//! Single-instance guard using a Win32 named mutex (F8.3).
//!
//! Ensures only one instance of `magicbridge-agent` runs at any time.

use std::fmt;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::{PCWSTR, w};

/// Default name for the daemon's single-instance mutex.
pub const DEFAULT_MUTEX_NAME: PCWSTR = w!(r"Local\MagicBridge_SingleInstance_Mutex");

/// Error occurred while acquiring the single-instance guard.
#[derive(Debug, PartialEq, Eq)]
pub enum SingleInstanceError {
    /// Another instance is already running.
    AlreadyRunning,
    /// Win32 API returned an unexpected error.
    WindowsError(u32),
}

impl fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => {
                write!(f, "Another instance of MagicBridge is already running")
            }
            Self::WindowsError(code) => {
                write!(
                    f,
                    "Failed to create single-instance mutex (Win32 error: {code})"
                )
            }
        }
    }
}

impl std::error::Error for SingleInstanceError {}

/// RAII guard holding the single-instance Win32 mutex.
///
/// When dropped, the mutex handle is closed, allowing another instance
/// (or a future run) to acquire it.
#[derive(Debug)]
pub struct SingleInstanceGuard {
    handle: HANDLE,
}

// SAFETY: A Win32 mutex HANDLE can be safely transferred across threads and closed.
unsafe impl Send for SingleInstanceGuard {}
// SAFETY: SingleInstanceGuard does not offer internal mutability.
unsafe impl Sync for SingleInstanceGuard {}

impl SingleInstanceGuard {
    /// Attempts to acquire the single-instance mutex with the default name.
    pub fn acquire() -> Result<Self, SingleInstanceError> {
        Self::acquire_named(DEFAULT_MUTEX_NAME)
    }

    /// Attempts to acquire the single-instance mutex with a specific name.
    pub fn acquire_named(name: PCWSTR) -> Result<Self, SingleInstanceError> {
        // SAFETY: `CreateMutexW` is called with None security attributes, initial ownership true,
        // and a valid null-terminated PCWSTR.
        let handle = unsafe { CreateMutexW(None, true, name) }
            .map_err(|e| SingleInstanceError::WindowsError(e.code().0 as u32))?;

        // Check if the mutex already existed prior to this call.
        // SAFETY: `GetLastError` has no preconditions and queries thread-local error state.
        let last_error = unsafe { GetLastError() };
        if last_error == ERROR_ALREADY_EXISTS {
            // Another instance holds the mutex; close our duplicate handle.
            // SAFETY: `handle` is a valid handle returned by `CreateMutexW`.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(SingleInstanceError::AlreadyRunning);
        }

        Ok(Self { handle })
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY: `self.handle` is a valid Win32 mutex handle created by `CreateMutexW`.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HSTRING;

    #[test]
    fn single_instance_acquire_and_conflict() {
        let unique_name = format!(
            r"Local\MagicBridge_Test_Mutex_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let hstring = HSTRING::from(unique_name.as_str());
        let pcwstr = PCWSTR(hstring.as_ptr());

        // First acquisition should succeed.
        let guard1 = SingleInstanceGuard::acquire_named(pcwstr)
            .expect("First acquisition of unique mutex should succeed");

        // Second acquisition with the same name should fail with AlreadyRunning.
        let guard2 = SingleInstanceGuard::acquire_named(pcwstr);
        assert_eq!(guard2.unwrap_err(), SingleInstanceError::AlreadyRunning);

        // After dropping the first guard, acquisition should succeed again.
        drop(guard1);

        let guard3 = SingleInstanceGuard::acquire_named(pcwstr)
            .expect("Acquisition after drop should succeed");
        drop(guard3);
    }
}
