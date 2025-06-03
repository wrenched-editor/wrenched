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
    // Time within the multiple click on mouse button will register...
    // Used by double click and triple clicks.
    pub multi_click_register_time: f64,
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
            multi_click_register_time: 0.25,
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
    pub cursor_color: Color,
    pub selection_color: Color,
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
            cursor_color: Color::from_rgb8(0x55, 0x55, 0x55),
            selection_color: Color::from_rgb8(0x15, 0x15, 0x15),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarkdowTheme {
    pub bullet_list_indentation: f64,
    pub numbered_list_indentation: f64,
    pub list_after_indentation: f64,
    pub list_top_margin: f64,

    pub standard_quotation: StandardQuotation,
    pub box_quotation: BoxQuotation,

    pub paragraph_top_margin: f64,

    pub horizontal_line_height: f64,
    pub horizontal_line_vertical_margin: f64,
    pub horizontal_line_horizontal_margin: f64,
    pub horizontal_line_color: Color,

    pub horizontal_code_block_margin: f64,
    pub code_block_margin: f64,

    pub header_line_height: f32,

    pub link_color: Color,
}

impl MarkdowTheme {
    fn new() -> MarkdowTheme {
        MarkdowTheme {
            // TODO: These should scale with text size somehow
            bullet_list_indentation: 10.0,
            numbered_list_indentation: 10.0,
            list_after_indentation: 5.0,
            list_top_margin: 10.0,

            standard_quotation: StandardQuotation {
                margine: Margin {
                    top: 10.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0,
                },
                line_horizontal_padding: 5.0,
                line_width: 4.0,
                color: Color::from_rgb8(0x4D, 0x4D, 0x4D),
            },
            box_quotation: BoxQuotation {
                margin: Margin {
                    top: 10.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0,
                },
                box_padding: Padding {
                    top: 5.0,
                    right: 5.0,
                    bottom: 5.0,
                    left: 5.0,
                },
                symbol_padding: Padding {
                    top: 5.0,
                    right: 5.0,
                    bottom: 5.0,
                    left: 5.0,
                },
                box_line_width: 2.0,
                note_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),
                important_color: Color::from_rgb8(254, 100, 11),
                tip_color: Color::from_rgb8(4, 165, 229),
                warning_color: Color::from_rgb8(223, 142, 29),
                caution_color: Color::from_rgb8(210, 15, 57),
                note_sign: "".to_string(),
                important_sign: "".to_string(),
                tip_sign: "󰛨".to_string(),
                warning_sign: "".to_string(),
                caution_sign: "".to_string(),
            },

            paragraph_top_margin: 10.0,

            horizontal_line_height: 2.0,
            horizontal_line_vertical_margin: 10.0,
            horizontal_line_horizontal_margin: 10.0,
            horizontal_line_color: Color::from_rgb8(0x4D, 0x4D, 0x4D),

            horizontal_code_block_margin: 10.0,
            code_block_margin: 10.0,

            header_line_height: 2.0,

            link_color: Color::from_rgb8(0x00, 0x4D, 0x00),
        }
    }
}

// TODO: I guess moving this into some commone types would be usefull???
#[derive(Debug, Clone)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

// TODO: I guess moving this into some commone types would be usefull???
#[derive(Debug, Clone)]
pub struct Padding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

// Style for standard quotation
//
// Some text
//
//   |
//   | Some cool qotation
//   |
//
#[derive(Debug, Clone)]
pub struct StandardQuotation {
    pub margine: Margin,
    pub line_horizontal_padding: f64,
    pub line_width: f64,
    pub color: Color,
}

// Style for box quotation (note/warning/highlight)
//
// Some text
//
//  +---+-------------------+
//  | W | Some cool warning |
//  +---+-------------------+
//
#[derive(Debug, Clone)]
pub struct BoxQuotation {
    pub margin: Margin,
    pub box_padding: Padding,
    pub symbol_padding: Padding,
    pub box_line_width: f64,

    pub note_color: Color,
    pub important_color: Color,
    pub tip_color: Color,
    pub warning_color: Color,
    pub caution_color: Color,

    pub note_sign: String,
    pub important_sign: String,
    pub tip_sign: String,
    pub warning_sign: String,
    pub caution_sign: String,
}

pub fn get_theme<'a>() -> RwLockReadGuard<'a, Theme> {
    (*THEME).read().unwrap()
}
