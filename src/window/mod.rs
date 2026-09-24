pub mod state;
pub mod tray;
pub mod window;

pub use state::{NotchConfig, NotchController, NotchState};
pub use tray::TrayIcon;
pub use window::NotchWindow;
