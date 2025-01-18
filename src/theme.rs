use std::sync::{LazyLock, RwLock, RwLockReadGuard};

use parley::{FontFamily, FontStack, GenericFamily};
use vello::peniko::Color;

static THEME: LazyLock<RwLock<Theme>> = LazyLock::new(|| RwLock::new(Theme::new()));

#[derive(Debug, Clone)]
pub struct Theme {
    pub text_color: Color,
    pub text_size: u32,
    pub scale: f32,
    pub font_stack: FontStack<'static>,
    pub monospace_font_stack: FontStack<'static>,
    pub monospace_text_color: Color,
    pub markdown_bullet_list_indentation: f32,
    pub markdown_numbered_list_indentation: f32,
    pub markdown_list_after_indentation: f32,
}

impl Theme {
    fn new() -> Theme {
        Theme {
            text_color: Color::rgb8(0xf0, 0xf0, 0xea),
            text_size: 16,
            scale: 1.0,
            font_stack: FontStack::Single(FontFamily::Generic(
                GenericFamily::SansSerif,
            )),
            monospace_font_stack: FontStack::Single(FontFamily::Generic(
                GenericFamily::Monospace,
            )),
            monospace_text_color: Color::rgb8(0xFF, 0x8C, 0x00),
            // TODO: These should scale with text size somehow
            markdown_bullet_list_indentation: 10.0,
            markdown_numbered_list_indentation: 5.0,
            markdown_list_after_indentation: 5.0,
        }
    }
}

pub fn get_theme<'a>() -> RwLockReadGuard<'a, Theme> {
    (*THEME).read().unwrap()
}
