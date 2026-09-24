fn main() {
    println!("cargo:rerun-if-changed=.env");

    // Read .env at build-time if present and inject into environment
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');
                if key == "CALENDAR_CLIENT_ID" || key == "CALENDAR_CLIENT_SECRET" {
                    println!("cargo:rustc-env={}={}", key, val);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "OptiNotch");
        res.set("FileDescription", "OptiNotch - Minimalist Dynamic Island Widget for Windows");
        res.set("LegalCopyright", "Copyright (C) 2026");
        res.set("OriginalFilename", "OptiNotch.exe");
        if std::path::Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }
        let _ = res.compile();
    }
}
