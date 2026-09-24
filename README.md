<div align="center">

  <img src="assets/tray-icon.svg" alt="OptiNotch Logo" width="100" height="100" />

  # OptiNotch

  **A minimalist, ultra-lightweight Dynamic Island widget for Windows built with Rust & Skia.**

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org)
  [![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-0078D6?logo=windows)](https://microsoft.com/windows)
  [![Memory Footprint](https://img.shields.io/badge/RAM-%3C%2010MB-brightgreen)](#-performance--resource-efficiency)
  [![Release](https://img.shields.io/github/v/release/ayman20-py/OptiNotch?color=blue)](https://github.com/ayman20-py/OptiNotch/releases)

  [Download Standalone Executable](https://github.com/ayman20-py/OptiNotch/releases/latest) • [Key Features](#-key-features) • [Installation](#-installation) • [Google Calendar Setup](#-google-calendar-integration) • [Building from Source](#-building-from-source)

</div>

---

## ✨ Overview

**OptiNotch** brings the elegance and utility of Apple's Dynamic Island to Windows 10 and 11. Engineered in 100% pure Rust with hardware-accelerated Skia 2D rendering, it provides immediate access to live media playback, your daily schedule, system health, and controls—all while maintaining an astonishingly light memory footprint (**~7–10 MB of RAM**).

---

## 🚀 Key Features

### 🎵 Modern Media Player
- **Universal SMTC Integration**: Seamlessly syncs with Spotify, Apple Music, YouTube Music, Tidal, VLC, Chrome, Edge, and more.
- **Interactive Controls**: Play, pause, skip, and seek with micro-spring animations and glowing tactile buttons.
- **Live Album Art**: High-fidelity rounded cover art rendering with smooth metadata scrolling.

### 📅 Calendar & Schedule Agenda
- **7-Day Interactive Agenda**: Fast glanceable view of upcoming meetings and daily tasks.
- **Month Picker Modal**: Expand into a full calendar grid to jump between dates and months with ease.
- **Color-Coded Event Strips**: Matches your original Google Calendar categories and tags.
- **Zero-Friction Sync**: Works via instant **iCal Secret Address** or **Google Cloud OAuth 2.0**.

### ⚡ System Status & Quick Controls
- **Live Clock & Date**: Clean minimalist typography.
- **Battery & Charging Monitor**: Real-time power level and charging indicator.
- **Master Volume**: Live audio level gauge.
- **Multi-Monitor Support**: Switch between monitors on-the-fly with the top-right display toggle.

### 🪶 Performance & Resource Efficiency
- **Sub-10 MB RAM Target**: Uses dynamic working-set compaction and aggressive idle culling to stay between **7 MB and 10 MB** of memory.
- **Zero Idle CPU Usage**: Pauses render loops when idle and leverages 120Hz VSync during spring physics transitions.
- **Pure Win32 GUI**: Native Windows subsystem without background command prompt windows (`cmd.exe`).

---

## ⌨️ Shortcuts & Interactions

| Action | Shortcut / Gesture | Description |
| :--- | :--- | :--- |
| **Expand / Collapse** | `Click Notch` or `Win + \` | Toggles the expanded island dashboard |
| **Peek / Temporary Hide** | `Hold Win + Alt` | Temporarily hides the notch for full-screen games/content |
| **Scroll Calendar** | `Mouse Wheel` on agenda | Smoothly scrolls through overflowing events |
| **Quick Tray Menu** | `Right-Click` Tray Icon | Access autostart, update checks, and app settings |
| **Switch Display** | Click `Monitor Icon` | Moves the notch across connected monitors |

---

## 📥 Installation

### Option 1: Standalone Portable Binary (Recommended)
1. Download **`OptiNotch.exe`** from the [Latest GitHub Release](https://github.com/ayman20-py/OptiNotch/releases/latest).
2. Run `OptiNotch.exe`.
3. When prompted, select **"Yes"** for 1-Click Installation:
   - Installs to `%LOCALAPPDATA%\OptiNotch`.
   - Creates Start Menu and Desktop shortcuts.
   - Registers in Windows Settings (Add or Remove Programs).
   - Configures automatic startup with Windows.
4. *Or select **"No"** to run portably from any USB or folder without modifying system settings.*

---

## 🔄 Auto-Update & Startup

- **Auto-Updates**: OptiNotch periodically checks GitHub Releases for new updates in the background. You can also right-click the tray icon and click **"Check for Updates..."** to update and relaunch in seconds.
- **Start with Windows**: Toggle the **`[✓] Start with Windows`** checkbox directly from the system tray menu.

---

## 📆 Google Calendar Integration

OptiNotch offers two effortless methods to connect your Google Calendar:

### ⚡ Method 1: iCal Secret URL (10-Second Setup — Recommended)
1. Open [Google Calendar Web Settings](https://calendar.google.com/calendar/u/0/r/settings) in your browser.
2. Select your calendar on the left under **"Settings for my calendars"**.
3. Scroll down to **"Secret address in iCal format"** and copy the URL.
4. In OptiNotch, click **"+ Connect Account"** $\rightarrow$ Click **"Yes"** to open `calendar_config.json`.
5. Paste your URL into `"ical_secret_url"`:
   ```json
   {
     "ical_secret_url": "https://calendar.google.com/calendar/ical/your_email/secret-xxxx/basic.ics",
     "client_id": "",
     "client_secret": ""
   }
   ```
6. Save the file. OptiNotch will immediately pull and render your schedule!

### 🔑 Method 2: Google Cloud OAuth 2.0
1. Create Desktop OAuth credentials in the [Google Cloud Console](https://console.cloud.google.com/).
2. Add `"client_id"` and `"client_secret"` into `%APPDATA%\OptiNotch\calendar_config.json`.
3. Click **"+ Connect Account"** in OptiNotch to authorize in your default browser.

---

## 🛠️ Building from Source

### Prerequisites
- [Rust Toolchain](https://rustup.rs/) (edition 2024 / 1.80+)
- Windows 10 or 11 with MSVC Build Tools installed

### Compilation
```powershell
# Clone repository
git clone https://github.com/ayman20-py/OptiNotch.git
cd OptiNotch

# Run in debug mode
cargo run

# Build optimized standalone release executable
powershell -ExecutionPolicy Bypass -File scripts/build_dist.ps1
```
The output executable will be created in `dist/OptiNotch.exe`.

---

## 📁 Architecture Overview

```
OptiNotch/
├── assets/             # Vector SVGs and multi-resolution Windows icons (.ico)
├── scripts/            # Build, icon generation, and packaging scripts
├── src/
│   ├── calendar/       # Calendar state, layout engine, Google & iCal sync, and rendering
│   ├── media/          # Windows SMTC media integration & playback control layout
│   ├── ui/             # Dynamic clock, system volume, and battery monitors
│   ├── updater/        # GitHub Releases version checking and self-updating engine
│   ├── window/         # Win32 layered window, tray icon, input hooks, and spring controller
│   ├── autostart.rs    # Windows Run registry management
│   ├── installer.rs    # 1-Click setup wizard and shortcut registration
│   ├── main.rs         # Event loop, timer dispatch, and memory working-set manager
│   └── render.rs       # Top-level Skia canvas drawing and compositing
└── build.rs            # Windows binary resource and metadata embedding
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE). Contributions, issues, and feature requests are welcome!
