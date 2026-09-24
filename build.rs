fn main() {
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
