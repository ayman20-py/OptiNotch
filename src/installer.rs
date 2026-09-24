use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const UNINSTALL_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\OptiNotch";
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn get_install_dir() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string());
    PathBuf::from(local_app_data).join("OptiNotch")
}

pub fn get_installed_exe() -> PathBuf {
    get_install_dir().join("OptiNotch.exe")
}

pub fn is_installed_location() -> bool {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let install_dir = get_install_dir();
    if let (Ok(cur), Ok(inst)) = (current_exe.canonicalize(), install_dir.canonicalize()) {
        cur.starts_with(inst)
    } else {
        current_exe.to_string_lossy().to_lowercase().starts_with(&install_dir.to_string_lossy().to_lowercase())
    }
}

/// Creates a Windows .lnk shortcut using Windows Script Host
fn create_shortcut(target_exe: &Path, shortcut_path: &Path, name: &str) -> Result<(), String> {
    let parent = shortcut_path.parent().unwrap_or(shortcut_path);
    let _ = std::fs::create_dir_all(parent);

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{0}'); $s.TargetPath = '{1}'; $s.WorkingDirectory = '{2}'; $s.Description = '{3}'; $s.Save()",
        shortcut_path.to_string_lossy().replace('\'', "''"),
        target_exe.to_string_lossy().replace('\'', "''"),
        target_exe.parent().unwrap_or(target_exe).to_string_lossy().replace('\'', "''"),
        name
    );

    let status = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-NoProfile")
        .arg("-Command")
        .arg(&script)
        .status()
        .map_err(|e| format!("Failed to create shortcut: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to execute shortcut creation script".to_string())
    }
}

/// Registers the application in Windows Settings (Add/Remove Programs)
fn register_uninstaller(installed_exe: &Path, install_dir: &Path) -> Result<(), String> {
    unsafe {
        let subkey_wide = to_wide(UNINSTALL_SUBKEY);
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
            return Err("Failed to create uninstall registry key".into());
        }

        let set_string_val = |hkey: HKEY, name: &str, val: &str| {
            let n_wide = to_wide(name);
            let v_wide = to_wide(val);
            let byte_len = (v_wide.len() * std::mem::size_of::<u16>()) as u32;
            RegSetValueExW(
                hkey,
                n_wide.as_ptr(),
                0,
                REG_SZ,
                v_wide.as_ptr() as *const u8,
                byte_len,
            );
        };

        set_string_val(hkey, "DisplayName", "OptiNotch");
        set_string_val(hkey, "DisplayVersion", crate::updater::CURRENT_VERSION);
        set_string_val(hkey, "Publisher", "ayman20-py");
        set_string_val(hkey, "DisplayIcon", &installed_exe.to_string_lossy());
        set_string_val(hkey, "InstallLocation", &install_dir.to_string_lossy());
        
        let uninstall_cmd = format!("\"{}\" --uninstall", installed_exe.to_string_lossy());
        set_string_val(hkey, "UninstallString", &uninstall_cmd);

        RegCloseKey(hkey);
        Ok(())
    }
}

/// Unregisters the uninstaller entry from Windows Settings
fn unregister_uninstaller() {
    unsafe {
        let subkey_wide = to_wide(UNINSTALL_SUBKEY);
        RegDeleteKeyW(HKEY_CURRENT_USER, subkey_wide.as_ptr());
    }
}

/// Performs the installation of OptiNotch
pub fn install() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Could not get current executable path: {}", e))?;

    let install_dir = get_install_dir();
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Could not create installation folder: {}", e))?;

    let target_exe = get_installed_exe();

    // Copy executable to LocalAppData folder
    if current_exe != target_exe {
        std::fs::copy(&current_exe, &target_exe)
            .map_err(|e| format!("Failed to copy binary to install location: {}", e))?;
    }

    // 1. Create Start Menu Shortcut
    if let Ok(app_data) = std::env::var("APPDATA") {
        let start_menu_path = PathBuf::from(app_data)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\OptiNotch.lnk");
        let _ = create_shortcut(&target_exe, &start_menu_path, "OptiNotch");
    }

    // 2. Create Desktop Shortcut
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let desktop_path = PathBuf::from(user_profile)
            .join("Desktop\\OptiNotch.lnk");
        let _ = create_shortcut(&target_exe, &desktop_path, "OptiNotch");
    }

    // 3. Register Uninstaller in Windows Settings
    let _ = register_uninstaller(&target_exe, &install_dir);

    // 4. Enable Startup on Boot
    let _ = crate::autostart::set_autostart_for_path(&target_exe, true);

    // 5. Spawn installed instance cleanly
    let _ = Command::new(&target_exe)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    unsafe {
        let msg = to_wide("OptiNotch has been successfully installed!\n\n• Location: %LOCALAPPDATA%\\OptiNotch\n• Shortcuts added to Start Menu and Desktop\n• Configured to start with Windows\n\nOptiNotch is now running in your system tray.");
        let title = to_wide("OptiNotch Setup");
        MessageBoxW(
            0 as _,
            msg.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }

    Ok(())
}

/// Uninstalls OptiNotch from the machine
pub fn uninstall() -> Result<(), String> {
    // 1. Remove autostart
    let _ = crate::autostart::set_autostart(false);

    // 2. Remove Shortcuts
    if let Ok(app_data) = std::env::var("APPDATA") {
        let start_menu_path = PathBuf::from(app_data)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\OptiNotch.lnk");
        let _ = std::fs::remove_file(start_menu_path);
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let desktop_path = PathBuf::from(user_profile)
            .join("Desktop\\OptiNotch.lnk");
        let _ = std::fs::remove_file(desktop_path);
    }

    // 3. Remove Registry Uninstaller entry
    unregister_uninstaller();

    // 4. Schedule cleanup script to delete install directory after process exits
    let install_dir = get_install_dir();
    let runner_path = std::env::temp_dir().join("optinotch_uninstall_runner.bat");
    let batch_script = format!(
        "@echo off\r\ntimeout /t 1 /nobreak >nul\r\nrd /s /q \"{0}\"\r\ndel \"%~f0\"\r\n",
        install_dir.to_string_lossy()
    );

    let _ = std::fs::write(&runner_path, batch_script);
    let _ = Command::new("cmd.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/C", runner_path.to_str().unwrap_or("")])
        .spawn();

    unsafe {
        let msg = to_wide("OptiNotch has been uninstalled successfully from your system.");
        let title = to_wide("OptiNotch Uninstaller");
        MessageBoxW(
            0 as _,
            msg.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }

    Ok(())
}

/// Handles CLI arguments and setup prompts on initial launch.
/// Returns `true` if OptiNotch should proceed to run, or `false` if it should exit (e.g. after setup).
pub fn handle_startup_and_installer() -> bool {
    let args: Vec<String> = std::env::args().collect();

    // Check for explicit flags
    if args.iter().any(|a| a == "--uninstall") {
        let _ = uninstall();
        return false;
    }

    if args.iter().any(|a| a == "--portable") {
        return true;
    }

    if args.iter().any(|a| a == "--install") {
        let _ = install();
        return false;
    }

    // If already running from the installed directory, simply run normally
    if is_installed_location() {
        return true;
    }

    // Otherwise, launched from Downloads/Desktop/external location:
    // Prompt the user for 1-Click Install or Portable execution
    unsafe {
        let prompt_text = to_wide("Welcome to OptiNotch!\n\nWould you like to install OptiNotch to your system?\n\n• Click 'Yes' to install to %LOCALAPPDATA%\\OptiNotch with Start Menu / Desktop shortcuts & Auto-Start.\n• Click 'No' to run portably without installing.\n• Click 'Cancel' to exit.");
        let title = to_wide("OptiNotch Setup");

        let response = MessageBoxW(
            0 as _,
            prompt_text.as_ptr(),
            title.as_ptr(),
            MB_YESNOCANCEL | MB_ICONQUESTION,
        );

        match response {
            IDYES => {
                let _ = install();
                false // Installer launched installed instance, exit this wrapper
            }
            IDNO => true, // Run portably
            _ => false,   // Cancel / Exit
        }
    }
}
