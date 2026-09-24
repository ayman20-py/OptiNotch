use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;

const SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_NAME: &str = "OptiNotch";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Checks whether OptiNotch is registered in the Windows Run key
pub fn is_autostart_enabled() -> bool {
    unsafe {
        let subkey_wide = to_wide(SUBKEY);
        let mut hkey: HKEY = 0 as _;

        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey_wide.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );

        if status != ERROR_SUCCESS {
            return false;
        }

        let name_wide = to_wide(APP_NAME);
        let mut data_type: u32 = 0;
        let mut data_len: u32 = 0;

        let query_res = RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            null_mut(),
            &mut data_type,
            null_mut(),
            &mut data_len,
        );

        RegCloseKey(hkey);

        query_res == ERROR_SUCCESS && data_len > 0
    }
}

/// Sets or unsets OptiNotch to start automatically on Windows logon using the current executable
pub fn set_autostart(enable: bool) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    set_autostart_for_path(&current_exe, enable)
}

/// Sets or unsets OptiNotch to start automatically on Windows logon for a specific executable path
pub fn set_autostart_for_path(exe_path: &std::path::Path, enable: bool) -> Result<(), String> {
    let exe_str = exe_path.to_string_lossy();
    let formatted_path = format!("\"{}\"", exe_str);

    unsafe {
        let subkey_wide = to_wide(SUBKEY);
        let mut hkey: HKEY = 0 as _;

        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey_wide.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_WRITE | KEY_READ,
            null_mut(),
            &mut hkey,
            null_mut(),
        );

        if status != ERROR_SUCCESS {
            return Err(format!("Failed to open/create registry key (error code {})", status));
        }

        let name_wide = to_wide(APP_NAME);

        let result = if enable {
            let path_wide = to_wide(&formatted_path);
            let byte_len = (path_wide.len() * std::mem::size_of::<u16>()) as u32;

            let set_res = RegSetValueExW(
                hkey,
                name_wide.as_ptr(),
                0,
                REG_SZ,
                path_wide.as_ptr() as *const u8,
                byte_len,
            );

            if set_res == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(format!("Failed to set registry value (error code {})", set_res))
            }
        } else {
            let del_res = RegDeleteValueW(hkey, name_wide.as_ptr());
            if del_res == ERROR_SUCCESS || del_res == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                Err(format!("Failed to delete registry value (error code {})", del_res))
            }
        };

        RegCloseKey(hkey);
        result
    }
}
