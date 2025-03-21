use std::sync::{LazyLock, RwLock, RwLockReadGuard};

use parley::{FontFamily, FontStack, GenericFamily};
use vello::peniko::Color;

use crate::generation::Generation;

static THEME: LazyLock<RwLock<Theme>> = LazyLock::new(|| RwLock::new(Theme::new()));

#[derive(Debug, Clone)]
pub struct Theme {
    pub scale: f32,
    pub text: TextTheme,
    pub markdown: MarkdowTheme,
    pub generation: Generation,
}

impl Theme {
    fn new() -> Theme {
        // Create first generation
        let mut generation = Generation::default();
        generation.nudge();
        Theme {
            scale: 1.0,
            text: TextTheme::new(),
            markdown: MarkdowTheme::new(),
            generation,
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

    pub indentation_horizonatl_margin: f64,
    pub indentation_vertical_margin: f64,
    pub indentation_line_margine: f64,
    pub indentation_line_width: f64,

    pub indentation_box_margin: f64,
    pub indentation_box_line_width: f64,

    pub indentation_color: Color,
    pub indentation_note_color: Color,
    pub indentation_important_color: Color,
    pub indentation_tip_color: Color,
    pub indentation_warning_color: Color,
    pub indentation_caution_color: Color,

    pub indentation_note_sign: String,
    pub indentation_important_sign: String,
    pub indentation_tip_sign: String,
    pub indentation_warning_sign: String,
    pub indentation_caution_sign: String,

    pub indentation_sign_top_padding: f64,
    pub indentation_sign_horizontal_padding: f64,

    pub paragraph_top_margin: f64,

    pub horizontal_line_height: f64,
    pub horizontal_line_vertical_margin: f64,
    pub horizontal_line_horizontal_margin: f64,
    pub horizontal_line_color: Color,

    pub horizontal_code_block_margin: f64,
    pub code_block_margin: f64,

    pub header_line_height: f32,
}

impl MarkdowTheme {
    fn new() -> MarkdowTheme {
        MarkdowTheme {
            // TODO: These should scale with text size somehow
            bullet_list_indentation: 10.0,
            numbered_list_indentation: 10.0,
            list_after_indentation: 5.0,
            list_top_margin: 10.0,

            indentation_horizonatl_margin: 10.0,
            indentation_vertical_margin: 10.0,
            indentation_line_margine: 5.0,
            indentation_line_width: 4.0,

            indentation_box_margin: 10.0,
            indentation_box_line_width: 2.0,

            indentation_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),
            indentation_note_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),
            indentation_important_color: Color::from_rgb8(254, 100, 11),
            indentation_tip_color: Color::from_rgb8(4, 165, 229),
            indentation_warning_color: Color::from_rgb8(223, 142, 29),
            indentation_caution_color: Color::from_rgb8(210, 15, 57),

            indentation_note_sign: "".to_string(),
            indentation_important_sign: "".to_string(),
            indentation_tip_sign: "󰛨".to_string(),
            indentation_warning_sign: "".to_string(),
            //indentation_warning_sign: "--".to_string(),
            indentation_caution_sign: "".to_string(),

            indentation_sign_top_padding: 5.0,
            indentation_sign_horizontal_padding: 5.0,

            paragraph_top_margin: 10.0,

            horizontal_line_height: 2.0,
            horizontal_line_vertical_margin: 10.0,
            horizontal_line_horizontal_margin: 10.0,
            horizontal_line_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),

            horizontal_code_block_margin: 10.0,
            code_block_margin: 10.0,

            header_line_height: 2.0,
        }
    }
}

pub fn get_theme<'a>() -> RwLockReadGuard<'a, Theme> {
    (*THEME).read().unwrap()
}
