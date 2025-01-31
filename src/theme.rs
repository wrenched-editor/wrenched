use std::sync::{LazyLock, RwLock, RwLockReadGuard};

use parley::{FontFamily, FontStack, GenericFamily};
use vello::peniko::Color;

static THEME: LazyLock<RwLock<Theme>> = LazyLock::new(|| RwLock::new(Theme::new()));

#[derive(Debug, Clone)]
pub struct Theme {
    pub scale: f32,
    pub text: TextTheme,
    pub markdown: MarkdowTheme,
}

impl Theme {
    fn new() -> Theme {
        Theme {
            scale: 1.0,
            text: TextTheme::new(),
            markdown: MarkdowTheme::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextTheme {
    pub font_stack: FontStack<'static>,
    pub monospace_font_stack: FontStack<'static>,
    pub text_color: Color,
    pub monospace_text_color: Color,
    pub text_size: u32,
    pub monospace_text_size: u32,
}

impl TextTheme {
    fn new() -> TextTheme {
        TextTheme {
            font_stack: FontStack::Single(FontFamily::Generic(
                GenericFamily::SansSerif,
            )),
            monospace_font_stack: FontStack::Single(FontFamily::Generic(
                GenericFamily::Monospace,
            )),
            text_color: Color::from_rgb8(0xf0, 0xf0, 0xea),
            monospace_text_color: Color::from_rgb8(0xFF, 0x8C, 0x00),
            text_size: 16,
            monospace_text_size: 16,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarkdowTheme {
    pub bullet_list_indentation: f64,
    pub numbered_list_indentation: f64,
    pub list_after_indentation: f64,
    pub list_top_margin: f64,
    pub indentation_decoration_width: f64,
    pub paragraph_top_margin: f64,
    pub horizontal_line_height: f64,
    pub horizontal_line_vertical_margin: f64,
    pub horizontal_line_horizontal_margin: f64,
    pub horizontal_line_color: Color,
    pub horizontal_code_block_margin: f64,
    pub code_block_margin: f64,
}

impl MarkdowTheme {
    fn new() -> MarkdowTheme {
        MarkdowTheme {
            // TODO: These should scale with text size somehow
            bullet_list_indentation: 10.0,
            numbered_list_indentation: 10.0,
            list_after_indentation: 5.0,
            list_top_margin: 10.0,

            indentation_decoration_width: 10.0,

            paragraph_top_margin: 10.0,

            horizontal_line_height: 2.0,
            horizontal_line_vertical_margin: 10.0,
            horizontal_line_horizontal_margin: 10.0,
            horizontal_line_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),

            horizontal_code_block_margin: 10.0,

            code_block_margin: 10.0,
        }
    }
}

pub fn get_theme<'a>() -> RwLockReadGuard<'a, Theme> {
    (*THEME).read().unwrap()
}
