mod calendar;
mod media;
mod render;
mod ui;
mod window;

use media::MediaManager;
use ui::clock::ClockUI;
use window::{NotchConfig, NotchController, NotchWindow, TrayIcon};
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

    // 2. Setup Config, Controller, Window, Clock UI, and Media Manager
    let config = NotchConfig::new(scale_factor);
    let mut controller = NotchController::new(config);
    let mut notch = NotchWindow::new(&mut controller);
    let mut clock_ui = ClockUI::new(scale_factor);
    let mut media_manager = MediaManager::new();

    // 3. Register System Tray Icon
    let tray = TrayIcon::new(notch.hwnd, 100, "OptiNotch - Click to toggle");

    // 4. Start 1-second clock timer
    unsafe {
        SetTimer(notch.hwnd, TIMER_CLOCK_ID, 1000, None);
    }

    // 5. Initial Render
    let (media_info, album_art) = media_manager.get_state();
    notch.render(|canvas| {
        render::draw_notch(
            canvas,
            controller.config.canvas_width,
            controller.config.canvas_height,
            &controller,
            &clock_ui,
            &media_info,
            album_art,
        );
    });

    println!("OptiNotch running! Multi-monitor scaling & controls active.");

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

            let (media_info, _album_art) = media_manager.get_state();
            controller.sync_play_state(media_info.is_playing);

            // Media Control Button Clicks
            if notch.check_media_toggle() {
                let next_play = !media_info.is_playing;
                controller.trigger_play_press(next_play);
                media_manager.toggle_play_pause();
            }
            if notch.check_media_next() {
                controller.trigger_next_press();
                media_manager.skip_next();
            }
            if notch.check_media_prev() {
                controller.trigger_prev_press();
                media_manager.skip_previous();
            }

            // Monitor Switch Button Click
            if notch.check_switch_monitor() {
                controller.trigger_monitor_press();
                let new_scale = notch.switch_to_next_monitor(&mut controller);
                clock_ui = ClockUI::new(new_scale);

                let (media_info, album_art) = media_manager.get_state();
                notch.render(|canvas| {
                    render::draw_notch(
                        canvas,
                        controller.config.canvas_width,
                        controller.config.canvas_height,
                        &controller,
                        &clock_ui,
                        &media_info,
                        album_art,
                    );
                });
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

                let (media_info, album_art) = media_manager.get_state();
                notch.render(|canvas| {
                    render::draw_notch(
                        canvas,
                        controller.config.canvas_width,
                        controller.config.canvas_height,
                        &controller,
                        &clock_ui,
                        &media_info,
                        album_art,
                    );
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
                    let (media_info, album_art) = media_manager.get_state();
                    notch.render(|canvas| {
                        render::draw_notch(
                            canvas,
                            controller.config.canvas_width,
                            controller.config.canvas_height,
                            &controller,
                            &clock_ui,
                            &media_info,
                            album_art,
                        );
                    });
                }
            }
        }
    }
}
