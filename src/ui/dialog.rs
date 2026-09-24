use skia_safe::{
    AlphaType, Canvas, Color, ColorType, Contains, Font, ImageInfo, Paint, PaintStyle, Point,
    RRect, Rect, surfaces,
};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, VK_ESCAPE, VK_RETURN};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::font_cache::FontCache;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResponse {
    Primary,
    Secondary,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogIcon {
    AppLogo,
    SuccessCheck,
    UpdateRocket,
    InfoNotice,
    ErrorAlert,
}

#[derive(Debug, Clone)]
pub struct DialogCardItem {
    pub badge: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DialogConfig {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: DialogIcon,
    pub cards: Vec<DialogCardItem>,
    pub primary_button: Option<String>,
    pub secondary_button: Option<String>,
    pub show_close_button: bool,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static DIALOG_CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

struct DialogLayout {
    win_w: i32,
    win_h: i32,
    scale: f32,
    card_rect: Rect,
    close_btn: Rect,
    primary_btn: Rect,
    secondary_btn: Rect,
    option_cards: Vec<Rect>,
}

struct DialogState {
    config: DialogConfig,
    layout: DialogLayout,
    hovered_primary: bool,
    hovered_secondary: bool,
    hovered_close: bool,
    hovered_card: Option<usize>,
    result: Option<DialogResponse>,
    hdc_mem: HDC,
    _hbitmap: HBITMAP,
    pixel_ptr: *mut c_void,
}

fn compute_layout(config: &DialogConfig, scale: f32) -> DialogLayout {
    let pad = 24.0 * scale;
    let card_w = 460.0 * scale;

    let header_h = 76.0 * scale;
    let subtitle_h = if config.subtitle.is_some() {
        24.0 * scale
    } else {
        0.0
    };

    let mut cards_total_h = 0.0;
    let card_item_h = 56.0 * scale;
    let card_spacing = 10.0 * scale;
    if !config.cards.is_empty() {
        cards_total_h = (config.cards.len() as f32 * card_item_h)
            + ((config.cards.len().saturating_sub(1)) as f32 * card_spacing)
            + (14.0 * scale);
    }

    let buttons_h = if config.primary_button.is_some() || config.secondary_button.is_some() {
        44.0 * scale
    } else {
        0.0
    };

    let shadow_pad = 18.0 * scale;
    let total_card_h = pad + header_h + subtitle_h + cards_total_h + buttons_h + pad;
    let total_win_w = (card_w + (shadow_pad * 2.0)) as i32;
    let total_win_h = (total_card_h + (shadow_pad * 2.0)) as i32;

    let card_rect = Rect::from_xywh(shadow_pad, shadow_pad, card_w, total_card_h);

    let close_size = 24.0 * scale;
    let close_btn = Rect::from_xywh(
        card_rect.right - (18.0 * scale) - close_size,
        card_rect.top + (18.0 * scale),
        close_size,
        close_size,
    );

    // Cards layout
    let mut option_cards = Vec::new();
    let mut current_y = card_rect.top + pad + header_h + subtitle_h;
    for _ in &config.cards {
        let r = Rect::from_xywh(
            card_rect.left + pad,
            current_y,
            card_w - (pad * 2.0),
            card_item_h,
        );
        option_cards.push(r);
        current_y += card_item_h + card_spacing;
    }

    // Button layout
    let btn_y = card_rect.bottom - pad - (38.0 * scale);
    let btn_h = 38.0 * scale;

    let (primary_btn, secondary_btn) = match (
        config.primary_button.is_some(),
        config.secondary_button.is_some(),
    ) {
        (true, true) => {
            let available_w = card_w - (pad * 2.0) - (12.0 * scale);
            let btn_w = available_w / 2.0;
            let sec = Rect::from_xywh(card_rect.left + pad, btn_y, btn_w, btn_h);
            let pri = Rect::from_xywh(
                card_rect.left + pad + btn_w + (12.0 * scale),
                btn_y,
                btn_w,
                btn_h,
            );
            (pri, sec)
        }
        (true, false) => {
            let btn_w = card_w - (pad * 2.0);
            let pri = Rect::from_xywh(card_rect.left + pad, btn_y, btn_w, btn_h);
            (pri, Rect::default())
        }
        (false, true) => {
            let btn_w = card_w - (pad * 2.0);
            let sec = Rect::from_xywh(card_rect.left + pad, btn_y, btn_w, btn_h);
            (Rect::default(), sec)
        }
        _ => (Rect::default(), Rect::default()),
    };

    DialogLayout {
        win_w: total_win_w,
        win_h: total_win_h,
        scale,
        card_rect,
        close_btn,
        primary_btn,
        secondary_btn,
        option_cards,
    }
}

fn render_dialog(canvas: &Canvas, state: &DialogState) {
    let s = state.layout.scale;
    let fonts = FontCache::get();

    canvas.clear(Color::TRANSPARENT);

    // 1. Soft Window Drop Shadow
    let shadow_rrect = RRect::new_rect_xy(state.layout.card_rect, 20.0 * s, 20.0 * s);
    let mut shadow_paint = Paint::default();
    shadow_paint.set_anti_alias(true);
    shadow_paint.set_color(Color::from_argb(90, 0, 0, 0));
    shadow_paint.set_mask_filter(skia_safe::MaskFilter::blur(
        skia_safe::BlurStyle::Normal,
        14.0 * s,
        None,
    ));
    canvas.draw_rrect(shadow_rrect, &shadow_paint);

    // 2. Card Background (Deep Dark Glass)
    let card_rrect = RRect::new_rect_xy(state.layout.card_rect, 20.0 * s, 20.0 * s);
    let mut bg_paint = Paint::default();
    bg_paint.set_anti_alias(true);
    bg_paint.set_color(Color::from_argb(250, 18, 18, 22)); // #121216
    canvas.draw_rrect(card_rrect, &bg_paint);

    // 3. Card Border Stroke
    let mut border_paint = Paint::default();
    border_paint.set_anti_alias(true);
    border_paint.set_style(PaintStyle::Stroke);
    border_paint.set_stroke_width(1.2 * s);
    border_paint.set_color(Color::from_argb(90, 113, 113, 122)); // #71717A subtle border
    canvas.draw_rrect(card_rrect, &border_paint);

    // 4. Icon & Header
    let icon_cx = state.layout.card_rect.left + (24.0 * s) + (18.0 * s);
    let icon_cy = state.layout.card_rect.top + (24.0 * s) + (18.0 * s);

    match state.config.icon {
        DialogIcon::AppLogo => {
            // Mini Vector Notch Logo
            let s_icon = s * 1.1;
            let pill_rect = Rect::from_xywh(
                icon_cx - (16.0 * s_icon),
                icon_cy - (8.0 * s_icon),
                32.0 * s_icon,
                16.0 * s_icon,
            );
            let pill_rrect = RRect::new_rect_xy(pill_rect, 8.0 * s_icon, 8.0 * s_icon);

            let mut p_fill = Paint::default();
            p_fill.set_anti_alias(true);
            p_fill.set_color(Color::from_argb(255, 228, 228, 231));
            canvas.draw_rrect(pill_rrect, &p_fill);

            let mut dot_paint = Paint::default();
            dot_paint.set_anti_alias(true);
            dot_paint.set_color(Color::from_argb(255, 245, 158, 11)); // Amber Dot
            canvas.draw_circle((icon_cx, icon_cy), 3.2 * s_icon, &dot_paint);
        }
        DialogIcon::SuccessCheck => {
            let mut circle_paint = Paint::default();
            circle_paint.set_anti_alias(true);
            circle_paint.set_color(Color::from_argb(255, 16, 185, 129)); // Emerald #10B981
            canvas.draw_circle((icon_cx, icon_cy), 18.0 * s, &circle_paint);

            let mut check_paint = Paint::default();
            check_paint.set_anti_alias(true);
            check_paint.set_style(PaintStyle::Stroke);
            check_paint.set_stroke_width(2.8 * s);
            check_paint.set_stroke_cap(skia_safe::PaintCap::Round);
            check_paint.set_stroke_join(skia_safe::PaintJoin::Round);
            check_paint.set_color(Color::WHITE);

            canvas.draw_line(
                (icon_cx - (6.0 * s), icon_cy),
                (icon_cx - (1.5 * s), icon_cy + (4.5 * s)),
                &check_paint,
            );
            canvas.draw_line(
                (icon_cx - (1.5 * s), icon_cy + (4.5 * s)),
                (icon_cx + (7.0 * s), icon_cy - (5.0 * s)),
                &check_paint,
            );
        }
        DialogIcon::UpdateRocket => {
            let mut circle_paint = Paint::default();
            circle_paint.set_anti_alias(true);
            circle_paint.set_color(Color::from_argb(255, 56, 189, 248)); // Sky Blue #38BDF8
            canvas.draw_circle((icon_cx, icon_cy), 18.0 * s, &circle_paint);

            let font_icon = Font::new(fonts.bold.clone(), 16.0 * s);
            let mut p = Paint::default();
            p.set_anti_alias(true);
            p.set_color(Color::from_argb(255, 15, 23, 42));
            canvas.draw_str(
                "↑",
                (icon_cx - (4.5 * s), icon_cy + (5.5 * s)),
                &font_icon,
                &p,
            );
        }
        _ => {
            let mut circle_paint = Paint::default();
            circle_paint.set_anti_alias(true);
            circle_paint.set_color(Color::from_argb(255, 99, 102, 241)); // Indigo #6366F1
            canvas.draw_circle((icon_cx, icon_cy), 18.0 * s, &circle_paint);

            let font_icon = Font::new(fonts.bold.clone(), 15.0 * s);
            let mut p = Paint::default();
            p.set_anti_alias(true);
            p.set_color(Color::WHITE);
            canvas.draw_str(
                "i",
                (icon_cx - (3.0 * s), icon_cy + (5.0 * s)),
                &font_icon,
                &p,
            );
        }
    }

    // 5. Title & Subtitle
    let text_x = icon_cx + (28.0 * s);
    let title_y = state.layout.card_rect.top + (24.0 * s) + (16.0 * s);

    let mut title_font = Font::new(fonts.bold.clone(), 17.0 * s);
    title_font.set_subpixel(true);
    let mut title_paint = Paint::default();
    title_paint.set_anti_alias(true);
    title_paint.set_color(Color::WHITE);
    canvas.draw_str(
        &state.config.title,
        (text_x, title_y),
        &title_font,
        &title_paint,
    );

    if let Some(ref sub) = state.config.subtitle {
        let mut sub_font = Font::new(fonts.regular.clone(), 12.5 * s);
        sub_font.set_subpixel(true);
        let mut sub_paint = Paint::default();
        sub_paint.set_anti_alias(true);
        sub_paint.set_color(Color::from_argb(200, 161, 161, 170)); // #A1A1AA
        canvas.draw_str(sub, (text_x, title_y + (18.0 * s)), &sub_font, &sub_paint);
    }

    // 6. Close '✕' Button
    if state.config.show_close_button {
        let close_rect = state.layout.close_btn;
        let close_rrect = RRect::new_rect_xy(close_rect, 6.0 * s, 6.0 * s);
        if state.hovered_close {
            let mut close_bg = Paint::default();
            close_bg.set_anti_alias(true);
            close_bg.set_color(Color::from_argb(60, 255, 255, 255));
            canvas.draw_rrect(close_rrect, &close_bg);
        }

        let mut close_font = Font::new(fonts.regular.clone(), 14.0 * s);
        close_font.set_subpixel(true);
        let mut close_paint = Paint::default();
        close_paint.set_anti_alias(true);
        close_paint.set_color(if state.hovered_close {
            Color::WHITE
        } else {
            Color::from_argb(150, 161, 161, 170)
        });
        canvas.draw_str(
            "✕",
            (close_rect.left + (7.0 * s), close_rect.top + (17.0 * s)),
            &close_font,
            &close_paint,
        );
    }

    // 7. Render Option / Feature Cards
    for (idx, (card_r, item)) in state
        .layout
        .option_cards
        .iter()
        .zip(&state.config.cards)
        .enumerate()
    {
        let is_hovered = state.hovered_card == Some(idx);
        let cr = RRect::new_rect_xy(*card_r, 12.0 * s, 12.0 * s);

        let mut card_bg = Paint::default();
        card_bg.set_anti_alias(true);
        card_bg.set_color(if is_hovered {
            Color::from_argb(240, 39, 39, 46) // Hovered #27272E
        } else {
            Color::from_argb(200, 26, 26, 30) // Default #1A1A1E
        });
        canvas.draw_rrect(cr, &card_bg);

        let mut card_border = Paint::default();
        card_border.set_anti_alias(true);
        card_border.set_style(PaintStyle::Stroke);
        card_border.set_stroke_width(1.0 * s);
        card_border.set_color(if is_hovered {
            Color::from_argb(120, 56, 189, 248) // Sky blue highlight on hover
        } else {
            Color::from_argb(50, 255, 255, 255)
        });
        canvas.draw_rrect(cr, &card_border);

        // Badge Icon
        let mut badge_font = Font::new(fonts.regular.clone(), 16.0 * s);
        badge_font.set_subpixel(true);
        let mut badge_paint = Paint::default();
        badge_paint.set_anti_alias(true);
        badge_paint.set_color(Color::WHITE);
        canvas.draw_str(
            &item.badge,
            (card_r.left + (14.0 * s), card_r.top + (33.0 * s)),
            &badge_font,
            &badge_paint,
        );

        // Title
        let text_lx = card_r.left + (42.0 * s);
        let mut item_title_font = Font::new(fonts.bold.clone(), 12.5 * s);
        item_title_font.set_subpixel(true);
        let mut item_title_paint = Paint::default();
        item_title_paint.set_anti_alias(true);
        item_title_paint.set_color(Color::WHITE);
        canvas.draw_str(
            &item.title,
            (text_lx, card_r.top + (22.0 * s)),
            &item_title_font,
            &item_title_paint,
        );

        // Description
        let mut item_desc_font = Font::new(fonts.regular.clone(), 11.0 * s);
        item_desc_font.set_subpixel(true);
        let mut item_desc_paint = Paint::default();
        item_desc_paint.set_anti_alias(true);
        item_desc_paint.set_color(Color::from_argb(180, 161, 161, 170));
        canvas.draw_str(
            &item.description,
            (text_lx, card_r.top + (40.0 * s)),
            &item_desc_font,
            &item_desc_paint,
        );
    }

    // 8. Secondary Button
    if let Some(ref sec_label) = state.config.secondary_button {
        let r = state.layout.secondary_btn;
        let rrect = RRect::new_rect_xy(r, 10.0 * s, 10.0 * s);

        let mut sec_bg = Paint::default();
        sec_bg.set_anti_alias(true);
        sec_bg.set_color(if state.hovered_secondary {
            Color::from_argb(255, 45, 45, 52)
        } else {
            Color::from_argb(220, 32, 32, 38)
        });
        canvas.draw_rrect(rrect, &sec_bg);

        let mut sec_stroke = Paint::default();
        sec_stroke.set_anti_alias(true);
        sec_stroke.set_style(PaintStyle::Stroke);
        sec_stroke.set_stroke_width(1.0 * s);
        sec_stroke.set_color(Color::from_argb(60, 255, 255, 255));
        canvas.draw_rrect(rrect, &sec_stroke);

        let mut sec_font = Font::new(fonts.bold.clone(), 12.0 * s);
        sec_font.set_subpixel(true);
        let mut sec_text_paint = Paint::default();
        sec_text_paint.set_anti_alias(true);
        sec_text_paint.set_color(Color::from_argb(240, 244, 244, 245));

        let (tw, _) = sec_font.measure_str(sec_label, Some(&sec_text_paint));
        let tx = r.left + (r.width() - tw) / 2.0;
        let ty = r.top + (r.height() / 2.0) + (4.0 * s);
        canvas.draw_str(sec_label, (tx, ty), &sec_font, &sec_text_paint);
    }

    // 9. Primary Button (Vibrant Accent Glow)
    if let Some(ref pri_label) = state.config.primary_button {
        let r = state.layout.primary_btn;
        let rrect = RRect::new_rect_xy(r, 10.0 * s, 10.0 * s);

        let mut pri_bg = Paint::default();
        pri_bg.set_anti_alias(true);
        pri_bg.set_color(if state.hovered_primary {
            Color::from_argb(255, 59, 130, 246) // Hover #3B82F6
        } else {
            Color::from_argb(255, 37, 99, 235) // #2563EB
        });
        canvas.draw_rrect(rrect, &pri_bg);

        let mut pri_stroke = Paint::default();
        pri_stroke.set_anti_alias(true);
        pri_stroke.set_style(PaintStyle::Stroke);
        pri_stroke.set_stroke_width(1.0 * s);
        pri_stroke.set_color(Color::from_argb(100, 255, 255, 255));
        canvas.draw_rrect(rrect, &pri_stroke);

        let mut pri_font = Font::new(fonts.bold.clone(), 12.5 * s);
        pri_font.set_subpixel(true);
        let mut pri_text_paint = Paint::default();
        pri_text_paint.set_anti_alias(true);
        pri_text_paint.set_color(Color::WHITE);

        let (tw, _) = pri_font.measure_str(pri_label, Some(&pri_text_paint));
        let tx = r.left + (r.width() - tw) / 2.0;
        let ty = r.top + (r.height() / 2.0) + (4.0 * s);
        canvas.draw_str(pri_label, (tx, ty), &pri_font, &pri_text_paint);
    }
}

unsafe extern "system" fn dialog_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DialogState };

    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let state = unsafe { &mut *state_ptr };

    match msg {
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as f32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as f32;
            let pt = Point::new(x, y);

            let old_p = state.hovered_primary;
            let old_s = state.hovered_secondary;
            let old_c = state.hovered_close;
            let old_card = state.hovered_card;

            state.hovered_primary =
                !state.layout.primary_btn.is_empty() && state.layout.primary_btn.contains(pt);
            state.hovered_secondary =
                !state.layout.secondary_btn.is_empty() && state.layout.secondary_btn.contains(pt);
            state.hovered_close =
                state.config.show_close_button && state.layout.close_btn.contains(pt);

            state.hovered_card = None;
            for (idx, r) in state.layout.option_cards.iter().enumerate() {
                if r.contains(pt) {
                    state.hovered_card = Some(idx);
                    break;
                }
            }

            if old_p != state.hovered_primary
                || old_s != state.hovered_secondary
                || old_c != state.hovered_close
                || old_card != state.hovered_card
            {
                unsafe { update_dialog_surface(hwnd, state) };
            }
            0
        }

        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as f32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as f32;
            let pt = Point::new(x, y);

            // If clicked on empty card background, allow dragging window
            let on_button = state.layout.primary_btn.contains(pt)
                || state.layout.secondary_btn.contains(pt)
                || state.layout.close_btn.contains(pt);

            if !on_button && state.layout.card_rect.contains(pt) {
                unsafe {
                    ReleaseCapture();
                    SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as _, 0);
                }
            }
            0
        }

        WM_LBUTTONUP => {
            let x = (lparam & 0xFFFF) as i16 as f32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as f32;
            let pt = Point::new(x, y);

            if !state.layout.primary_btn.is_empty() && state.layout.primary_btn.contains(pt) {
                state.result = Some(DialogResponse::Primary);
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            } else if !state.layout.secondary_btn.is_empty()
                && state.layout.secondary_btn.contains(pt)
            {
                state.result = Some(DialogResponse::Secondary);
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            } else if state.config.show_close_button && state.layout.close_btn.contains(pt) {
                state.result = Some(DialogResponse::Cancel);
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            } else if let Some(idx) = state.hovered_card {
                if idx == 0 && state.config.primary_button.is_some() {
                    state.result = Some(DialogResponse::Primary);
                    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                } else if idx == 1 && state.config.secondary_button.is_some() {
                    state.result = Some(DialogResponse::Secondary);
                    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                }
            }
            0
        }

        WM_KEYDOWN => {
            if wparam == VK_ESCAPE as usize {
                state.result = Some(DialogResponse::Cancel);
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            } else if wparam == VK_RETURN as usize {
                state.result = Some(DialogResponse::Primary);
                unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
            }
            0
        }

        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }

        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn update_dialog_surface(hwnd: HWND, state: &mut DialogState) {
    let width = state.layout.win_w;
    let height = state.layout.win_h;

    let image_info = ImageInfo::new(
        (width, height),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );

    let row_bytes = (width * 4) as usize;
    let mut surface = match surfaces::wrap_pixels(
        &image_info,
        unsafe {
            std::slice::from_raw_parts_mut(state.pixel_ptr as *mut u8, row_bytes * height as usize)
        },
        Some(row_bytes),
        None,
    ) {
        Some(s) => s,
        None => return,
    };

    render_dialog(surface.canvas(), state);

    unsafe {
        let hdc_screen = GetDC(0 as _);
        let mut blend: BLENDFUNCTION = std::mem::zeroed();
        blend.BlendOp = AC_SRC_OVER as u8;
        blend.SourceConstantAlpha = 255;
        blend.AlphaFormat = AC_SRC_ALPHA as u8;

        let mut size = SIZE {
            cx: width,
            cy: height,
        };
        let mut pt_src = POINT { x: 0, y: 0 };

        UpdateLayeredWindow(
            hwnd,
            hdc_screen,
            null(),
            &mut size,
            state.hdc_mem,
            &mut pt_src,
            0,
            &blend,
            ULW_ALPHA,
        );

        ReleaseDC(0 as _, hdc_screen);
    }
}

pub fn show_dialog(config: DialogConfig) -> DialogResponse {
    unsafe {
        let hinstance = GetModuleHandleW(null());

        let class_name = to_wide("OptiNotchModernDialog");
        if !DIALOG_CLASS_REGISTERED.swap(true, Ordering::SeqCst) {
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.style = CS_HREDRAW | CS_VREDRAW;
            wc.lpfnWndProc = Some(dialog_wnd_proc);
            wc.hInstance = hinstance;
            wc.hCursor = LoadCursorW(0 as _, IDC_ARROW);
            wc.lpszClassName = class_name.as_ptr();
            RegisterClassExW(&wc);
        }

        let dpi = GetDpiForSystem() as f32;
        let scale = dpi / 96.0;
        let layout = compute_layout(&config, scale);

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_x = (screen_w - layout.win_w) / 2;
        let win_y = (screen_h - layout.win_h) / 2;

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_APPWINDOW,
            class_name.as_ptr(),
            to_wide(&config.title).as_ptr(),
            WS_POPUP,
            win_x,
            win_y,
            layout.win_w,
            layout.win_h,
            0 as _,
            0 as _,
            hinstance,
            null(),
        );

        let hdc_screen = GetDC(0 as _);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = layout.win_w;
        bmi.bmiHeader.biHeight = -layout.win_h; // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut pixel_ptr: *mut c_void = null_mut();
        let hbitmap = CreateDIBSection(hdc_screen, &bmi, DIB_RGB_COLORS, &mut pixel_ptr, 0 as _, 0);

        SelectObject(hdc_mem, hbitmap);
        ReleaseDC(0 as _, hdc_screen);

        let mut state = Box::new(DialogState {
            config,
            layout,
            hovered_primary: false,
            hovered_secondary: false,
            hovered_close: false,
            hovered_card: None,
            result: None,
            hdc_mem,
            _hbitmap: hbitmap,
            pixel_ptr,
        });

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *state as *mut _ as isize);

        update_dialog_surface(hwnd, &mut state);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);

        // Modal message loop
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let final_result = state.result.unwrap_or(DialogResponse::Cancel);

        // Cleanup GDI
        DeleteObject(hbitmap);
        DeleteDC(hdc_mem);

        final_result
    }
}

/// Modern Setup Dialog (First Run / Installer)
pub fn show_setup_dialog() -> DialogResponse {
    let config = DialogConfig {
        title: "Welcome to OptiNotch".to_string(),
        subtitle: Some("Choose how you would like to run OptiNotch on your PC:".to_string()),
        icon: DialogIcon::AppLogo,
        cards: vec![
            DialogCardItem {
                badge: "🚀".to_string(),
                title: "Install to System (Recommended)".to_string(),
                description:
                    "Start Menu & Desktop shortcuts, Auto-Start on boot, Windows integration"
                        .to_string(),
            },
            DialogCardItem {
                badge: "💼".to_string(),
                title: "Run Portably".to_string(),
                description: "Run standalone from current folder without making system changes"
                    .to_string(),
            },
        ],
        primary_button: Some("Install OptiNotch".to_string()),
        secondary_button: Some("Run Portably".to_string()),
        show_close_button: true,
    };

    show_dialog(config)
}

/// Modern Success Dialog (Installed)
pub fn show_installed_success_dialog() {
    let config = DialogConfig {
        title: "OptiNotch Installed!".to_string(),
        subtitle: Some("Setup completed successfully:".to_string()),
        icon: DialogIcon::SuccessCheck,
        cards: vec![
            DialogCardItem {
                badge: "✓".to_string(),
                title: "Installed to %LOCALAPPDATA%\\OptiNotch".to_string(),
                description: "Shortcuts added to Start Menu & Desktop • Auto-Start enabled"
                    .to_string(),
            },
            DialogCardItem {
                badge: "⚡".to_string(),
                title: "Running in System Tray".to_string(),
                description: "Press Win + \\ to expand your Dynamic Notch anytime".to_string(),
            },
        ],
        primary_button: Some("Launch & Enjoy".to_string()),
        secondary_button: None,
        show_close_button: false,
    };

    let _ = show_dialog(config);
}

/// Modern Uninstall Confirmation / Success Dialog
pub fn show_uninstalled_dialog() {
    let config = DialogConfig {
        title: "OptiNotch Uninstalled".to_string(),
        subtitle: Some("OptiNotch has been removed from your system:".to_string()),
        icon: DialogIcon::InfoNotice,
        cards: vec![DialogCardItem {
            badge: "🗑️".to_string(),
            title: "Shortcuts & registry removed".to_string(),
            description: "All application data and registry entries were cleanly deleted."
                .to_string(),
        }],
        primary_button: Some("Close".to_string()),
        secondary_button: None,
        show_close_button: false,
    };

    let _ = show_dialog(config);
}

/// Modern Update Available Dialog
pub fn show_update_available_dialog(version: &str, release_notes: &str) -> bool {
    let clean_notes = if release_notes.trim().is_empty() {
        "Includes latest performance optimizations and bug fixes.".to_string()
    } else {
        release_notes.lines().take(2).collect::<Vec<_>>().join(" ")
    };

    let config = DialogConfig {
        title: "Update Available".to_string(),
        subtitle: Some(format!(
            "A new version of OptiNotch is ready (v{})",
            version
        )),
        icon: DialogIcon::UpdateRocket,
        cards: vec![DialogCardItem {
            badge: "✨".to_string(),
            title: format!("OptiNotch v{}", version),
            description: clean_notes,
        }],
        primary_button: Some("Update Now".to_string()),
        secondary_button: Some("Later".to_string()),
        show_close_button: true,
    };

    show_dialog(config) == DialogResponse::Primary
}

/// Modern Up to Date Dialog
pub fn show_up_to_date_dialog(version: &str) {
    let config = DialogConfig {
        title: "You're Up to Date!".to_string(),
        subtitle: Some(format!(
            "OptiNotch v{} is currently the newest version.",
            version
        )),
        icon: DialogIcon::SuccessCheck,
        cards: vec![DialogCardItem {
            badge: "✓".to_string(),
            title: "Latest Release Installed".to_string(),
            description: "No new updates were found on GitHub.".to_string(),
        }],
        primary_button: Some("Got It".to_string()),
        secondary_button: None,
        show_close_button: false,
    };

    let _ = show_dialog(config);
}

/// Modern Alert / Error Dialog
pub fn show_alert_dialog(title: &str, message: &str) {
    let config = DialogConfig {
        title: title.to_string(),
        subtitle: None,
        icon: DialogIcon::ErrorAlert,
        cards: vec![DialogCardItem {
            badge: "ℹ️".to_string(),
            title: "Notice".to_string(),
            description: message.to_string(),
        }],
        primary_button: Some("OK".to_string()),
        secondary_button: None,
        show_close_button: true,
    };

    let _ = show_dialog(config);
}
