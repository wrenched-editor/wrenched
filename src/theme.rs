use std::sync::{LazyLock, Mutex, MutexGuard};

use vello::peniko::Color;

static THEME: LazyLock<Mutex<Theme>> = LazyLock::new(|| Mutex::new(Theme::new()));

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub text_color: Color,
    pub text_size: u32,
    pub scale: f32,
}

impl Theme {
    pub fn new() -> Theme {
        Theme {
            text_color: Color::rgb8(0xf0, 0xf0, 0xea),
            text_size: 12,
            scale: 1.0,
        }
    }
}

pub fn get_theme<'a>() -> MutexGuard<'a, Theme> {
    (*THEME).lock().unwrap()
}
