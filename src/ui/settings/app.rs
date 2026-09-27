use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use std::num::NonZeroIsize;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::ProcessStatus::EmptyWorkingSet;
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use wry::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use wry::{Rect as WryRect, WebView, WebViewBuilder};

struct HwndWrapper(HWND);

impl HasWindowHandle for HwndWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(NonZeroIsize::new(self.0 as isize).unwrap());
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

impl HasDisplayHandle for HwndWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Windows(
                WindowsDisplayHandle::new(),
            )))
        }
    }
}

static SETTINGS_HWND: AtomicUsize = AtomicUsize::new(0);
static IS_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

const SETTINGS_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; user-select: none; }
  body {
    background: #121216;
    color: #e4e4e7;
    font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, sans-serif;
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  /* Sidebar */
  #sidebar {
    width: 200px;
    background: #0e0e12;
    border-right: 1px solid #1e1e24;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }
  .sidebar-header {
    font-size: 16px;
    font-weight: 700;
    color: #ffffff;
    padding: 6px 10px 14px 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .nav-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: #a1a1aa;
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    width: 100%;
  }
  .nav-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #ffffff;
  }
  .nav-btn.active {
    background: rgba(37, 99, 235, 0.18);
    border-color: rgba(59, 130, 246, 0.5);
    color: #60a5fa;
    font-weight: 600;
  }
  /* Content Area */
  #content {
    flex: 1;
    background: #16161c;
    padding: 28px 32px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .tab-title {
    font-size: 20px;
    font-weight: 700;
    color: #ffffff;
  }
  .divider {
    height: 1px;
    background: #272730;
    width: 100%;
  }
  .tab-panel {
    display: none;
    flex-direction: column;
    gap: 16px;
  }
  .tab-panel.active {
    display: flex;
  }
</style>
</head>
<body>
  <div id="sidebar">
    <div class="sidebar-header">Settings</div>
    <button class="nav-btn active" onclick="switchTab('general')">General</button>
    <button class="nav-btn" onclick="switchTab('appearance')">Appearance</button>
    <button class="nav-btn" onclick="switchTab('calendar')">Calendar</button>
    <button class="nav-btn" onclick="switchTab('media')">Media</button>
    <button class="nav-btn" onclick="switchTab('hotkeys')">Hotkeys</button>
    <button class="nav-btn" onclick="switchTab('about')">About</button>
  </div>
  <div id="content">
    <div id="tab-title" class="tab-title">General Settings</div>
    <div class="divider"></div>

    <!-- Tab 0: General -->
    <div id="panel-general" class="tab-panel active">
      <!-- General UI components go here -->
    </div>

    <!-- Tab 1: Appearance -->
    <div id="panel-appearance" class="tab-panel">
      <!-- Appearance UI components go here -->
    </div>

    <!-- Tab 2: Calendar -->
    <div id="panel-calendar" class="tab-panel">
      <!-- Calendar UI components go here -->
    </div>

    <!-- Tab 3: Media -->
    <div id="panel-media" class="tab-panel">
      <!-- Media UI components go here -->
    </div>

    <!-- Tab 4: Hotkeys -->
    <div id="panel-hotkeys" class="tab-panel">
      <!-- Hotkeys UI components go here -->
    </div>

    <!-- Tab 5: About -->
    <div id="panel-about" class="tab-panel">
      <!-- About UI components go here -->
    </div>
  </div>

  <script>
    const titles = {
      'general': 'General Settings',
      'appearance': 'Appearance',
      'calendar': 'Calendar',
      'media': 'Media',
      'hotkeys': 'Hotkeys & Shortcuts',
      'about': 'About OptiNotch'
    };
    function switchTab(name) {
      document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.tab-panel').forEach(p => p.classList.remove('active'));
      const buttons = Array.from(document.querySelectorAll('.nav-btn'));
      const activeBtn = buttons.find(b => b.getAttribute('onclick').includes("'" + name + "'"));
      if (activeBtn) activeBtn.classList.add('active');
      const panel = document.getElementById('panel-' + name);
      if (panel) panel.classList.add('active');
      document.getElementById('tab-title').textContent = titles[name] || 'Settings';
    }
  </script>
</body>
</html>"#;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static mut WEBVIEW_PTR: usize = 0;

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            unsafe {
                if WEBVIEW_PTR != 0 {
                    let webview = &mut *(WEBVIEW_PTR as *mut WebView);
                    let mut rect: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rect);
                    let w = (rect.right - rect.left) as u32;
                    let h = (rect.bottom - rect.top) as u32;
                    let _ = webview.set_bounds(WryRect {
                        position: Position::Physical(PhysicalPosition::new(0, 0)),
                        size: Size::Physical(PhysicalSize::new(w, h)),
                    });
                }
            }
            0
        }
        WM_CLOSE => {
            // Hide window instead of destroying so it can be re-shown instantly
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
                EmptyWorkingSet(GetCurrentProcess());
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn run_settings_window_thread() {
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("OptiNotchSettingsClass");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0 as _,
            hCursor: LoadCursorW(0 as _, IDC_ARROW),
            hbrBackground: 0 as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0 as _,
        };
        RegisterClassExW(&wc);

        let window_title = to_wide("OptiNotch Settings");
        let width = 740;
        let height = 540;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - width) / 2;
        let y = (screen_h - height) / 2;

        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_THICKFRAME,
            x,
            y,
            width,
            height,
            0 as _,
            0 as _,
            hinstance,
            null_mut(),
        );

        if hwnd == 0 as _ {
            eprintln!("[!] Failed to create Settings HWND");
            IS_THREAD_RUNNING.store(false, Ordering::SeqCst);
            return;
        }

        // Enable Windows Immersive Dark Mode title bar
        let dark_mode: i32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );

        // Build WebView2 attached directly to the Win32 HWND
        let mut client_rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client_rect);
        let cw = (client_rect.right - client_rect.left) as u32;
        let ch = (client_rect.bottom - client_rect.top) as u32;

        let handle_wrapper = HwndWrapper(hwnd);
        let webview = match WebViewBuilder::new()
            .with_bounds(WryRect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(PhysicalSize::new(cw, ch)),
            })
            .with_html(SETTINGS_HTML)
            .build_as_child(&handle_wrapper)
        {
            Ok(wv) => wv,
            Err(e) => {
                eprintln!("[!] Failed to build WebView2: {:?}", e);
                DestroyWindow(hwnd);
                IS_THREAD_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        let mut boxed_webview = Box::new(webview);
        WEBVIEW_PTR = boxed_webview.as_mut() as *mut WebView as usize;

        SETTINGS_HWND.store(hwnd as usize, Ordering::SeqCst);

        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        WEBVIEW_PTR = 0;
        SETTINGS_HWND.store(0, Ordering::SeqCst);
        IS_THREAD_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Opens or brings the Settings window into focus.
pub fn open_settings_window() {
    let hwnd_val = SETTINGS_HWND.load(Ordering::SeqCst);
    if hwnd_val != 0 {
        let hwnd = hwnd_val as HWND;
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
        return;
    }

    if !IS_THREAD_RUNNING.swap(true, Ordering::SeqCst) {
        thread::spawn(|| {
            run_settings_window_thread();
        });
    }
}
