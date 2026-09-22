mod render;
mod ui;
mod window;

use ui::clock::ClockUI;
use window::{NotchConfig, NotchController, NotchWindow};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PeekMessageW, SetTimer, TranslateMessage, MSG, PM_REMOVE,
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

    // 2. Setup Config, Spring Controller, and Layered Window
    let config = NotchConfig::new(scale_factor);
    let mut controller = NotchController::new(config);
    let mut notch = NotchWindow::new(&mut controller);
    let clock_ui = ClockUI::new(scale_factor);

    // 3. Start 1-second clock timer
    unsafe {
        SetTimer(notch.hwnd, TIMER_CLOCK_ID, 1000, None);
    }

    // 4. Initial Render
    let canvas_w = controller.config.canvas_width;
    let canvas_h = controller.config.canvas_height;
    notch.render(|canvas| {
        render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
    });

    println!("OptiNotch running! (Synced to display refresh rate via DwmFlush)");

    // 5. Main Event Loop with Hardware VSync (120Hz+)
    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        'main_loop: loop {
            // Check if user clicked the notch
            if notch.check_clicked() {
                controller.toggle_state();
            }

            if controller.is_animating {
                // ============================================================
                // HIGH REFRESH RATE ANIMATION LOOP (120Hz / 144Hz / 240Hz)
                // ============================================================

                // 1. Step spring physics
                controller.step_animation();

                // 2. Render frame with Skia
                notch.render(|canvas| {
                    render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
                });

                // 3. Drain all pending Win32 messages without blocking
                while PeekMessageW(&mut msg, 0 as _, 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        break 'main_loop;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // 4. SYNC WITH HARDWARE VSYNC via DWM Compositor:
                // DwmFlush blocks until the exact monitor refresh interval (8.33ms for 120Hz)
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

                // Handle 1-second clock updates
                if msg.message == WM_TIMER && msg.wParam == TIMER_CLOCK_ID {
                    notch.render(|canvas| {
                        render::draw_notch(canvas, canvas_w, canvas_h, &controller, &clock_ui);
                    });
                }
            }
        }
    }
}
