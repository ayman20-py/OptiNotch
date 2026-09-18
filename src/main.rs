use floem::{ 
    Application, IntoView, peniko::Color, reactive::create_effect, views::{ Decorators, container }, window::WindowConfig
};

mod notch_config;
use crate::notch_config::{calculate_notch_layout, enable_smooth_transparency};

fn app_view() -> impl IntoView {
    create_effect(move |_| {
        enable_smooth_transparency();
    });

    let notch_content = (
        "Welcome to OptiNotch",
    );

    container(notch_content)
        .style(|s| { 
            s.width_full()
                .height_full()
                .background(Color::rgb8(24, 24, 27))
                .color(Color::WHITE)
                .border_radius(14.0)
                .items_center()
                .justify_center()
        })
}

fn main() {
    let (notch_w, notch_h, position) = calculate_notch_layout();

    let window_conf = WindowConfig::default()
        .size((notch_w, notch_h))
        .title("OptiNotch")
        .resizable(false)
        .position(position)
        .window_level(floem::window::WindowLevel::AlwaysOnTop)
        .undecorated(true)
        .with_transparent(true)
        .undecorated_shadow(false)
        .show_titlebar(false);

    Application::new()
        .window(move |_| app_view(), Some(window_conf))
        .run();
}