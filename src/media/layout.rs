use crate::window::state::MediaAction;
use skia_safe::{Contains, Point, Rect};

#[derive(Debug, Clone, Copy, Default)]
pub struct MediaLayout {
    pub art_rect: Rect,
    pub info_x: f32,
    pub info_y: f32,
    pub bar_rect: Rect,
    pub prev_btn: Rect,
    pub play_btn: Rect,
    pub next_btn: Rect,
}

impl MediaLayout {
    pub fn compute(pill_x: f32, pill_y: f32, current_w: f32, current_h: f32, scale: f32) -> Self {
        let art_size = 75.0 * scale;
        let left_margin = 20.0 * scale;
        let art_x = pill_x + left_margin;
        let art_y = pill_y + (current_h - art_size) / 1.4;
        let art_rect = Rect::from_xywh(art_x, art_y, art_size, art_size);

        // Gap between album art and media info
        let art_gap = 10.0 * scale;
        let info_x = art_x + art_size + art_gap;
        let info_y = art_y + (art_size * 0.1);
        let media_w = current_w * 0.5;
        let max_w = (pill_x + media_w - info_x - (8.0 * scale)).max(60.0 * scale);

        let bar_y = info_y + (42.0 * scale);
        let bar_w = (175.0 * scale).min(max_w);
        let bar_rect = Rect::from_xywh(info_x, bar_y, bar_w, 3.5 * scale);

        let controls_cx = info_x + bar_w / 2.0;
        let controls_cy = bar_y + (30.0 * scale);
        let spacing = 38.0 * scale;
        let btn_size = 32.0 * scale;

        let play_btn = Rect::from_xywh(
            controls_cx - btn_size / 2.0,
            controls_cy - btn_size / 2.0,
            btn_size,
            btn_size,
        );

        let prev_btn = Rect::from_xywh(
            controls_cx - spacing - btn_size / 2.0,
            controls_cy - btn_size / 2.0,
            btn_size,
            btn_size,
        );

        let next_btn = Rect::from_xywh(
            controls_cx + spacing - btn_size / 2.0,
            controls_cy - btn_size / 2.0,
            btn_size,
            btn_size,
        );

        Self {
            art_rect,
            info_x,
            info_y,
            bar_rect,
            prev_btn,
            play_btn,
            next_btn,
        }
    }

    /// Check which media button was clicked
    pub fn hit_test(&self, local_x: f32, local_y: f32) -> MediaAction {
        let pt = Point::new(local_x, local_y);

        if self.play_btn.contains(pt) {
            MediaAction::TogglePlayPause
        } else if self.prev_btn.contains(pt) {
            MediaAction::SkipPrevious
        } else if self.next_btn.contains(pt) {
            MediaAction::SkipNext
        } else {
            MediaAction::None
        }
    }
}
