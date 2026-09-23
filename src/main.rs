mod render;
mod ui;
mod window;

use ui::clock::ClockUI;
use window::{NotchConfig, NotchController, NotchWindow, TrayIcon};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PM_REMOVE, PeekMessageW, SetTimer, TranslateMessage,
    WM_QUIT, WM_TIMER,
};

const TIMER_CLOCK_ID: usize = 1;

fn main() {
    // 1. Enable Per-Monitor DPI Awareness V2
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let dpi = unsafe { GetDpiForSystem() } as f32;
    let scale_factor = dpi / 96.0;

    // 2. Setup Config, Controller, Window, and Clock UI
    let config = NotchConfig::new(scale_factor);
    let mut controller = NotchController::new(config);
    let mut notch = NotchWindow::new(&mut controller);
    let clock_ui = ClockUI::new(scale_factor);

    // 3. Register System Tray Icon
    let tray = TrayIcon::new(notch.hwnd, 100, "OptiNotch - Click to toggle");

    // 4. Start 1-second clock timer
    unsafe {
        SetTimer(notch.hwnd, TIMER_CLOCK_ID, 1000, None);
    }

    // 5. Initial Render
    let canvas_w = controller.config.canvas_width;
    let canvas_h = controller.config.canvas_height;
    notch.render(|canvas| {
        render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
    });

    println!("OptiNotch running! Click collapsed notch to expand; click outside to collapse.");

    // 6. Main Event Loop with Hardware VSync
    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        'main_loop: loop {
            // Check for exit command from tray menu
            if notch.check_exit_requested() {
                break 'main_loop;
            }

            // Expand when user clicks the collapsed notch
            if notch.check_expand_requested() {
                controller.expand();
            }

            // Collapse when user clicks outside the expanded card
            if notch.check_collapse_requested() {
                controller.collapse();
            }

            // Check if user right-clicked tray icon -> show context menu
            if notch.check_show_tray_menu() {
                tray.show_context_menu();
            }

            if controller.is_animating {
                // ============================================================
                // HIGH REFRESH RATE ANIMATION LOOP (120Hz VSync)
                // ============================================================
                controller.step_animation();

                notch.render(|canvas| {
                    render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
                });

                while PeekMessageW(&mut msg, 0 as _, 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        break 'main_loop;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                DwmFlush();
            } else {
                // ============================================================
                // IDLE MODE (0.0% CPU)
                // ============================================================
                if GetMessageW(&mut msg, 0 as _, 0, 0) <= 0 {
                    break 'main_loop;
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                if msg.message == WM_TIMER && msg.wParam == TIMER_CLOCK_ID {
                    notch.render(|canvas| {
                        render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
                    });
                }
            }
        }
    }
}
