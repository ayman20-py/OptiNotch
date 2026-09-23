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
        let expanded_width = 500.0 * scale_factor;
        let expanded_height = 180.0 * scale_factor;
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

/// Damped harmonic spring physics model (Apple-style dynamic fluid motion)
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

    /// Step spring physics by dt seconds
    pub fn update(&mut self, dt: f32) -> bool {
        let dt = dt.min(0.032);

        let displacement = self.current - self.target;
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * self.velocity;
        let acceleration = spring_force + damping_force;

        self.velocity += acceleration * dt;
        self.current += self.velocity * dt;

        let is_settled = displacement.abs() < 0.25 && self.velocity.abs() < 0.5;
        if is_settled {
            self.current = self.target;
            self.velocity = 0.0;
        }

        !is_settled
    }
}

pub struct NotchController {
    pub state: NotchState,
    pub config: NotchConfig,
    pub width_spring: Spring,
    pub height_spring: Spring,
    pub is_animating: bool,
    pub last_frame_time: Option<Instant>,
}

impl NotchController {
    pub fn new(config: NotchConfig) -> Self {
        let initial_w = config.collapsed_width;
        let initial_h = config.collapsed_height;

        let width_spring = Spring::new(initial_w, 400.0, 35.0);
        let height_spring = Spring::new(initial_h, 500.0, 35.0);

        Self {
            state: NotchState::Collapsed,
            config,
            width_spring,
            height_spring,
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

    /// Explicitly expand the notch
    pub fn expand(&mut self) {
        if self.state != NotchState::Expanded {
            self.state = NotchState::Expanded;
            self.width_spring.set_target(self.config.expanded_width);
            self.height_spring.set_target(self.config.expanded_height);
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    /// Explicitly collapse the notch
    pub fn collapse(&mut self) {
        if self.state != NotchState::Collapsed {
            self.state = NotchState::Collapsed;
            self.width_spring.set_target(self.config.collapsed_width);
            self.height_spring.set_target(self.config.collapsed_height);
            self.is_animating = true;
            self.last_frame_time = Some(Instant::now());
        }
    }

    /// Advance physics using delta time
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

        self.is_animating = w_active || h_active;
        self.is_animating
    }

    /// Hit-test: Check if local window coordinates (x, y) fall inside the active notch shape
    pub fn is_inside_pill(&self, local_x: f32, local_y: f32) -> bool {
        let cur_w = self.current_width();
        let cur_h = self.current_height();
        let pill_x = (self.config.canvas_width - cur_w) / 2.0;
        let pill_y = 0.0;

        local_x >= pill_x && local_x <= pill_x + cur_w && local_y >= pill_y && local_y <= pill_y + cur_h
    }

    /// Hit-test from global screen coordinates (screen_x, screen_y)
    pub fn is_inside_screen_rect(&self, window_x: i32, window_y: i32, screen_x: i32, screen_y: i32) -> bool {
        let local_x = (screen_x - window_x) as f32;
        let local_y = (screen_y - window_y) as f32;
        self.is_inside_pill(local_x, local_y)
    }

    /// Check if a click hit a media playback control button (Play/Pause, Previous, Next)
    pub fn check_media_click(&self, local_x: f32, local_y: f32) -> MediaAction {
        if self.state != NotchState::Expanded {
            return MediaAction::None;
        }

        let scale = self.config.scale_factor;
        let pill_x = (self.config.canvas_width - self.current_width()) / 2.0;
        let pill_y = 0.0;
        let current_w = self.current_width();
        let current_h = self.current_height();

        let art_size = 70.0 * scale;
        let art_x = pill_x + (current_w * 0.05);
        let art_y = pill_y + (current_h - art_size) / 2.0;
        let info_x = art_x + (art_size * 1.15);
        let info_y = art_y + (art_size * 0.2);
        let bar_y = info_y + (36.0 * scale);
        let bar_w = (200.0 * scale).min(pill_x + current_w - info_x - (20.0 * scale));
        let controls_cx = info_x + bar_w / 2.0;
        let controls_cy = bar_y + (30.0 * scale);
        let spacing = 36.0 * scale;

        let hit_circle = |cx: f32, cy: f32, radius: f32| -> bool {
            let dx = local_x - cx;
            let dy = local_y - cy;
            (dx * dx + dy * dy) <= (radius * radius)
        };

        if hit_circle(controls_cx, controls_cy, 20.0 * scale) {
            MediaAction::TogglePlayPause
        } else if hit_circle(controls_cx - spacing, controls_cy, 18.0 * scale) {
            MediaAction::SkipPrevious
        } else if hit_circle(controls_cx + spacing, controls_cy, 18.0 * scale) {
            MediaAction::SkipNext
        } else {
            MediaAction::None
        }
    }
}
