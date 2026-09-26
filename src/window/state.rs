use skia_safe::Contains;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotchState {
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaAction {
    TogglePlayPause,
    SkipNext,
    SkipPrevious,
    SwitchMonitor,
    CalendarSelectDay(usize),
    CalendarPrevWeek,
    CalendarNextWeek,
    CalendarOpenMonthPicker,
    CalendarCloseMonthPicker,
    CalendarPrevMonth,
    CalendarNextMonth,
    CalendarSelectPickerDate { year: u32, month: u32, day: u32 },
    CalendarConnectGoogle,
    None,
}

#[derive(Debug, Clone)]
pub struct NotchConfig {
    pub collapsed_width: f32,
    pub collapsed_height: f32,
    pub expanded_width: f32,
    pub expanded_height: f32,
    pub top_padding: f32,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub scale_factor: f32,
}

impl NotchConfig {
    pub fn new(scale_factor: f32) -> Self {
        let collapsed_width = 100.0 * scale_factor;
        let collapsed_height = 28.0 * scale_factor;
        let expanded_width = 570.0 * scale_factor;
        let expanded_height = 185.0 * scale_factor;
        let top_padding = 6.0 * scale_factor;

        let canvas_width = expanded_width + 40.0 * scale_factor;
        let canvas_height = expanded_height + 40.0 * scale_factor;

        Self {
            collapsed_width,
            collapsed_height,
            expanded_width,
            expanded_height,
            top_padding,
            canvas_width,
            canvas_height,
            scale_factor,
        }
    }
}

/// Damped harmonic spring physics model
#[derive(Debug, Clone)]
pub struct Spring {
    pub current: f32,
    pub target: f32,
    pub velocity: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    pub fn new(initial: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
            stiffness,
            damping,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn update(&mut self, dt: f32) -> bool {
        let dt = dt.min(0.032);

        let displacement = self.current - self.target;
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * self.velocity;
        let acceleration = spring_force + damping_force;

        self.velocity += acceleration * dt;
        self.current += self.velocity * dt;

        let is_settled = displacement.abs() < 0.005 && self.velocity.abs() < 0.01;
        if is_settled {
            self.current = self.target;
            self.velocity = 0.0;
        }

        !is_settled
    }
}

#[derive(Debug, Clone)]
pub struct ButtonAnimations {
    pub play_scale: Spring,
    pub play_morph: Spring, // 0.0 = Play, 1.0 = Pause
    pub prev_scale: Spring,
    pub prev_nudge: Spring,
    pub next_scale: Spring,
    pub next_nudge: Spring,
    pub monitor_scale: Spring,
    pub monitor_glow: Spring,
    pub monitor_rotate: Spring,
}

impl ButtonAnimations {
    pub fn new() -> Self {
        Self {
            play_scale: Spring::new(1.0, 520.0, 26.0),
            play_morph: Spring::new(0.0, 360.0, 28.0),
            prev_scale: Spring::new(1.0, 520.0, 26.0),
            prev_nudge: Spring::new(0.0, 480.0, 24.0),
            next_scale: Spring::new(1.0, 520.0, 26.0),
            next_nudge: Spring::new(0.0, 480.0, 24.0),
            monitor_scale: Spring::new(1.0, 520.0, 26.0),
            monitor_glow: Spring::new(0.0, 380.0, 24.0),
            monitor_rotate: Spring::new(0.0, 440.0, 22.0),
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        let a1 = self.play_scale.update(dt);
        let a2 = self.play_morph.update(dt);
        let a3 = self.prev_scale.update(dt);
        let a4 = self.prev_nudge.update(dt);
        let a5 = self.next_scale.update(dt);
        let a6 = self.next_nudge.update(dt);
        let a7 = self.monitor_scale.update(dt);
        let a8 = self.monitor_glow.update(dt);
        let a9 = self.monitor_rotate.update(dt);

        a1 || a2 || a3 || a4 || a5 || a6 || a7 || a8 || a9
    }
}

pub struct NotchController {
    pub state: NotchState,
    pub config: NotchConfig,
    pub width_spring: Spring,
    pub height_spring: Spring,
    pub opacity_spring: Spring, // 0.0 = completely transparent/hidden, 1.0 = fully visible
    pub scale_spring: Spring,   // 0.85 = micro-scale when hidden, 1.0 = normal
    pub is_hidden: bool,
    pub btn_anims: ButtonAnimations,
    pub calendar: crate::ui::calendar::CalendarState,
    pub current_monitor: usize,
    pub total_monitors: usize,
    pub is_animating: bool,
    pub last_frame_time: Option<Instant>,
}

impl NotchController {
    pub fn new(config: NotchConfig) -> Self {
        let initial_w = config.collapsed_width;
        let initial_h = config.collapsed_height;

        let width_spring = Spring::new(initial_w, 400.0, 35.0);
        let height_spring = Spring::new(initial_h, 500.0, 35.0);
        let opacity_spring = Spring::new(1.0, 950.0, 42.0);
        let scale_spring = Spring::new(1.0, 850.0, 38.0);

        Self {
            state: NotchState::Collapsed,
            config,
            width_spring,
            height_spring,
            opacity_spring,
            scale_spring,
            is_hidden: false,
            btn_anims: ButtonAnimations::new(),
            calendar: crate::ui::calendar::CalendarState::new(),
            current_monitor: 0,
            total_monitors: 1,
            is_animating: false,
            last_frame_time: None,
        }
    }

    pub fn current_width(&self) -> f32 {
        self.width_spring.current
    }

    pub fn current_height(&self) -> f32 {
        self.height_spring.current
    }

    pub fn expand(&mut self) {
        if self.state != NotchState::Expanded {
            self.calendar.reset_to_today();
            self.state = NotchState::Expanded;
            self.width_spring.set_target(self.config.expanded_width);
            self.height_spring.set_target(self.config.expanded_height);
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    pub fn collapse(&mut self) {
        if self.state != NotchState::Collapsed {
            self.state = NotchState::Collapsed;
            self.width_spring.set_target(self.config.collapsed_width);
            self.height_spring.set_target(self.config.collapsed_height);
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    /// Trigger micro-click animations
    pub fn trigger_play_press(&mut self, is_playing: bool) {
        self.btn_anims.play_scale.current = 0.78;
        self.btn_anims.play_morph.set_target(if is_playing { 1.0 } else { 0.0 });
        self.is_animating = true;
        self.last_frame_time = Some(Instant::now());
    }

    pub fn trigger_prev_press(&mut self) {
        self.btn_anims.prev_scale.current = 0.82;
        self.btn_anims.prev_nudge.current = -5.0 * self.config.scale_factor;
        self.is_animating = true;
        self.last_frame_time = Some(Instant::now());
    }

    pub fn trigger_next_press(&mut self) {
        self.btn_anims.next_scale.current = 0.82;
        self.btn_anims.next_nudge.current = 5.0 * self.config.scale_factor;
        self.is_animating = true;
        self.last_frame_time = Some(Instant::now());
    }

    pub fn trigger_monitor_press(&mut self) {
        self.btn_anims.monitor_scale.current = 0.70;
        self.btn_anims.monitor_glow.current = 1.0;
        self.btn_anims.monitor_rotate.current = 14.0;
        self.is_animating = true;
        self.last_frame_time = Some(Instant::now());
    }

    pub fn update_scale(&mut self, new_scale_factor: f32) {
        let prev_scale = self.config.scale_factor;
        if (prev_scale - new_scale_factor).abs() < 0.001 {
            return;
        }

        let ratio = new_scale_factor / prev_scale;
        self.config = NotchConfig::new(new_scale_factor);

        self.width_spring.current *= ratio;
        self.width_spring.target = match self.state {
            NotchState::Collapsed => self.config.collapsed_width,
            NotchState::Expanded => self.config.expanded_width,
        };

        self.height_spring.current *= ratio;
        self.height_spring.target = match self.state {
            NotchState::Collapsed => self.config.collapsed_height,
            NotchState::Expanded => self.config.expanded_height,
        };

        self.is_animating = true;
        self.last_frame_time = Some(Instant::now());
    }

    pub fn sync_play_state(&mut self, is_playing: bool) {
        let target = if is_playing { 1.0 } else { 0.0 };
        if (self.btn_anims.play_morph.target - target).abs() > 0.01 {
            self.btn_anims.play_morph.set_target(target);
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        if self.is_hidden != hidden {
            self.is_hidden = hidden;
            self.opacity_spring.set_target(if hidden { 0.0 } else { 1.0 });
            self.scale_spring.set_target(if hidden { 0.88 } else { 1.0 });
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    pub fn step_animation(&mut self) -> bool {
        if !self.is_animating {
            return false;
        }

        let now = Instant::now();
        let dt = self
            .last_frame_time
            .map(|last| (now - last).as_secs_f32())
            .unwrap_or(0.00833);
        self.last_frame_time = Some(now);

        let w_active = self.width_spring.update(dt);
        let h_active = self.height_spring.update(dt);
        let op_active = self.opacity_spring.update(dt);
        let sc_active = self.scale_spring.update(dt);
        let btn_active = self.btn_anims.update(dt);
        let scroll_active = self.calendar.update_scroll(dt);

        self.is_animating = w_active || h_active || op_active || sc_active || btn_active || scroll_active;
        self.is_animating
    }

    pub fn is_inside_pill(&self, local_x: f32, local_y: f32) -> bool {
        let cur_w = self.current_width();
        let cur_h = self.current_height();
        let pill_x = (self.config.canvas_width - cur_w) / 2.0;
        let pill_y = 0.0;

        local_x >= pill_x && local_x <= pill_x + cur_w && local_y >= pill_y && local_y <= pill_y + cur_h
    }

    pub fn is_inside_screen_rect(&self, window_x: i32, window_y: i32, screen_x: i32, screen_y: i32) -> bool {
        let local_x = (screen_x - window_x) as f32;
        let local_y = (screen_y - window_y) as f32;
        self.is_inside_pill(local_x, local_y)
    }

    pub fn check_media_click(&self, local_x: f32, local_y: f32) -> MediaAction {
        if self.state != NotchState::Expanded {
            return MediaAction::None;
        }

        let scale = self.config.scale_factor;
        let current_w = self.current_width();
        let current_h = self.current_height();
        let pill_x = (self.config.canvas_width - current_w) / 2.0;
        let pill_y = 0.0;

        // 1. Check Header Monitor Switch Button (Top Right)
        let mon_btn_w = 44.0 * scale;
        let mon_btn_h = 28.0 * scale;
        let mon_btn_x = pill_x + current_w - (56.0 * scale);
        let mon_btn_y = pill_y + (8.0 * scale);
        let mon_rect = skia_safe::Rect::from_xywh(mon_btn_x, mon_btn_y, mon_btn_w, mon_btn_h);
        if mon_rect.contains(skia_safe::Point::new(local_x, local_y)) {
            return MediaAction::SwitchMonitor;
        }

        // 2. Check Media Controls
        let media_layout =
            crate::ui::media::MediaLayout::compute(pill_x, pill_y, current_w, current_h, scale);
        let media_act = media_layout.hit_test(local_x, local_y);
        if media_act != MediaAction::None {
            return media_act;
        }

        // 3. Check Calendar Controls
        let cal_layout = crate::ui::calendar::CalendarLayout::compute(
            pill_x,
            pill_y,
            current_w,
            current_h,
            scale,
            &self.calendar,
        );
        match cal_layout.hit_test(local_x, local_y, self.calendar.view_mode) {
            crate::ui::calendar::CalendarAction::SelectDay(idx) => MediaAction::CalendarSelectDay(idx),
            crate::ui::calendar::CalendarAction::PrevWeek => MediaAction::CalendarPrevWeek,
            crate::ui::calendar::CalendarAction::NextWeek => MediaAction::CalendarNextWeek,
            crate::ui::calendar::CalendarAction::OpenMonthPicker => {
                MediaAction::CalendarOpenMonthPicker
            }
            crate::ui::calendar::CalendarAction::CloseMonthPicker => {
                MediaAction::CalendarCloseMonthPicker
            }
            crate::ui::calendar::CalendarAction::PrevMonth => MediaAction::CalendarPrevMonth,
            crate::ui::calendar::CalendarAction::NextMonth => MediaAction::CalendarNextMonth,
            crate::ui::calendar::CalendarAction::SelectPickerDate { year, month, day } => {
                MediaAction::CalendarSelectPickerDate { year, month, day }
            }
            crate::ui::calendar::CalendarAction::ConnectGoogle => MediaAction::CalendarConnectGoogle,
            crate::ui::calendar::CalendarAction::None => MediaAction::None,
        }
    }
}
