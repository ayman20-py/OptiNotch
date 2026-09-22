mod render;
mod ui;
mod window;

use ui::clock::ClockUI;
use window::NotchWindow;
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, SetTimer, TranslateMessage, MSG, WM_TIMER,
};

const TIMER_CLOCK_ID: usize = 1;

fn main() {
    // 1. Enable Per-Monitor DPI Awareness V2
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // 2. Query system DPI and calculate scaling factor (e.g. 96 DPI = 1.0, 120 DPI = 1.25, 144 DPI = 1.5)
    let dpi = unsafe { GetDpiForSystem() } as f32;
    let scale_factor = dpi / 96.0;

    // 3. Define base logical dimensions (DIPs) and scale to physical pixels
    let base_width = 100.0;
    let base_height = 28.0;
    let base_top_padding = 6.0;

    let compact_width = (base_width * scale_factor).round() as i32;
    let compact_height = (base_height * scale_factor).round() as i32;
    let top_padding = (base_top_padding * scale_factor).round() as i32;

    let mut notch = NotchWindow::new(compact_width, compact_height, top_padding);
    let clock_ui = ClockUI::new(scale_factor);

    // 4. Set up a 1-second interval timer for the clock
    unsafe {
        SetTimer(notch.hwnd, TIMER_CLOCK_ID, 1000, None);
    }

    // 5. Initial Render
    notch.render(|canvas| {
        render::draw_notch(canvas, compact_width as f32, compact_height as f32, &clock_ui);
    });

    println!(
        "OptiNotch running! (DPI: {}, Scale: {}x, Size: {}x{})",
        dpi, scale_factor, compact_width, compact_height
    );

    // 6. Win32 Event Loop
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Re-render clock every second
            if msg.message == WM_TIMER && msg.wParam == TIMER_CLOCK_ID {
                notch.render(|canvas| {
                    render::draw_notch(
                        canvas,
                        compact_width as f32,
                        compact_height as f32,
                        &clock_ui,
                    );
                });
            }
        }
    }
}
