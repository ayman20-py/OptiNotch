use serde::Deserialize;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "ayman20-py/OptiNotch";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub download_url: String,
    pub is_newer: bool,
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize, Debug)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Query GitHub Releases API for the latest version
pub fn check_for_updates() -> Result<Option<UpdateInfo>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("OptiNotch-Updater")
        .timeout(Duration::from_secs(8))
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let res = client.get(&url).send()?;

    if !res.status().is_success() {
        return Ok(None);
    }

    let release: GitHubRelease = res.json()?;
    let latest_version = release.tag_name.trim_start_matches('v');

    let is_newer = is_version_newer(CURRENT_VERSION, latest_version);

    // Find the standalone executable asset (e.g. OptiNotch.exe)
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".exe") || a.name.to_lowercase().contains("opti-notch"))
        .or_else(|| release.assets.first());

    let download_url = asset
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_default();

    Ok(Some(UpdateInfo {
        tag_name: release.tag_name,
        name: release.name.unwrap_or_else(|| "OptiNotch Update".to_string()),
        body: release.body.unwrap_or_default(),
        download_url,
        is_newer,
    }))
}

/// Download latest executable and atomically swap with currently running binary
pub fn apply_update(download_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if download_url.is_empty() {
        return Err("Download URL is empty".into());
    }

    let current_exe = std::env::current_exe()?;
    let fallback_dir = PathBuf::from(".");
    let current_dir = current_exe.parent().unwrap_or(&fallback_dir);
    let temp_exe = current_dir.join("OptiNotch_new.exe");
    let batch_updater = current_dir.join("update_runner.bat");

    // 1. Stream download the new binary to temporary file
    let client = reqwest::blocking::Client::builder()
        .user_agent("OptiNotch-Updater")
        .timeout(Duration::from_secs(60))
        .build()?;

    let mut response = client.get(download_url).send()?;
    let mut file = fs::File::create(&temp_exe)?;
    std::io::copy(&mut response, &mut file)?;
    drop(file);

    // 2. Generate small self-replacing updater batch script that waits for current process to exit, swaps binary, and relaunches
    let exe_path_str = current_exe.to_string_lossy();
    let temp_path_str = temp_exe.to_string_lossy();
    let batch_path_str = batch_updater.to_string_lossy();

    let batch_script = format!(
        "@echo off\r\n\
        timeout /t 1 /nobreak > NUL\r\n\
        :retry\r\n\
        move /y \"{}\" \"{}\" > NUL\r\n\
        if errorlevel 1 (\r\n\
            timeout /t 1 /nobreak > NUL\r\n\
            goto retry\r\n\
        )\r\n\
        start \"\" \"{}\"\r\n\
        del \"{}\"\r\n",
        temp_path_str, exe_path_str, exe_path_str, batch_path_str
    );

    fs::write(&batch_updater, batch_script)?;

    // 3. Launch the batch updater hidden (CREATE_NO_WINDOW = 0x08000000)
    Command::new("cmd")
        .args(["/C", &batch_path_str])
        .creation_flags(0x08000000)
        .spawn()?;

    // 4. Terminate current process to allow replacement
    std::process::exit(0);
}

/// Simple semantic version comparison
fn is_version_newer(current: &str, latest: &str) -> bool {
    let parse_v = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let curr_parts = parse_v(current);
    let latest_parts = parse_v(latest);

    let max_len = curr_parts.len().max(latest_parts.len());
    for i in 0..max_len {
        let c = *curr_parts.get(i).unwrap_or(&0);
        let l = *latest_parts.get(i).unwrap_or(&0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    false
}
