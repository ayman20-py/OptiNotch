mod render;
mod window;

use window::NotchWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};
use windows_sys::Win32::UI::HiDpi::{ SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 };

fn main() {
    // To prevent the automatic scalling of the notch due to winodws scalling
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let compact_width = 150;
    let compact_height = 58;
    let top_padding = 6; // 6px from the top edge of screen

    let mut notch = NotchWindow::new(compact_width, compact_height, top_padding);

    // Initial Render using Skia
    notch.render(|canvas| {
        render::draw_notch(canvas, compact_width as f32, compact_height as f32);
    });

    println!("OptiNotch running! Look at the top center of your screen.");

    // Win32 Message Loop
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
