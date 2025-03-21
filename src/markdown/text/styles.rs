use masonry::core::BrushIndex;
use parley::{FontStyle, RangedBuilder, StyleProperty};
use xilem::FontWeight;

use crate::theme::Theme;

use super::layouted_text::Brush;

#[derive(Clone, Debug)]
pub struct BrushPalete {
    pub palete: Vec<Brush>,
}

impl BrushPalete {
    pub fn new(theme: &Theme) -> BrushPalete {
        BrushPalete {
            palete: vec![
                Brush::just_text(theme.text.text_color),
                Brush::just_text(theme.text.monospace_text_color),
                Brush::just_text(theme.markdown.indentation_color),
                Brush::just_text(theme.markdown.indentation_note_color),
                Brush::just_text(theme.markdown.indentation_important_color),
                Brush::just_text(theme.markdown.indentation_tip_color),
                Brush::just_text(theme.markdown.indentation_warning_color),
                Brush::just_text(theme.markdown.indentation_caution_color),
            ],
        }
    }

    pub fn palete(&self) -> &[Brush]{
        &self.palete
    }

    // TODO: Maybe enum would be better but it is hard to say how worth it is
    // to dig into this direction.
    pub const TEXT_BRUSH: BrushIndex = BrushIndex(0);
    pub const CODE_BRUSH: BrushIndex = BrushIndex(1);
    pub const INDENTATION_BRUSH: BrushIndex = BrushIndex(2);
    pub const NOTE_BRUSH: BrushIndex = BrushIndex(3);
    pub const IMPORTANT_BRUSH: BrushIndex = BrushIndex(4);
    pub const TIP_BRUSH: BrushIndex = BrushIndex(5);
    pub const WARNING_BRUSH: BrushIndex = BrushIndex(6);
    pub const CAUTION_BRUSH: BrushIndex = BrushIndex(7);

    pub fn fill_default_styles(
        theme: &Theme,
        builder: &mut RangedBuilder<'_, BrushIndex>,
    ) {
        builder
            .push_default(StyleProperty::Brush(BrushPalete::TEXT_BRUSH));
        builder.push_default(StyleProperty::FontSize(theme.text.text_size as f32));
        builder.push_default(theme.text.font_stack.clone());
        builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
        builder.push_default(StyleProperty::FontStyle(FontStyle::Normal));
        builder.push_default(StyleProperty::LineHeight(1.0));
    }
}

#[derive(Clone)]
pub struct TextMarker {
    // TODO: Think about making it into range
    pub start_pos: usize,
    pub end_pos: usize,
    pub kind: MarkerKind,
}

impl TextMarker {
    pub fn feed_to_builder<'a>(
        &self,
        builder: &'a mut RangedBuilder<BrushIndex>,
        theme: &'a Theme,
    ) {
        let rang = self.start_pos..self.end_pos;
        match self.kind {
            MarkerKind::Bold => {
                builder.push(StyleProperty::FontWeight(FontWeight::BOLD), rang)
            }
            MarkerKind::Italic => {
                builder.push(StyleProperty::FontStyle(FontStyle::Italic), rang)
            }
            MarkerKind::Strikethrough => {
                builder.push(StyleProperty::Strikethrough(true), rang)
            }
            MarkerKind::InlineCode => {
                // TODO: Draw additional decorations??? Maybe into the brush???
                builder.push(
                    StyleProperty::FontStack(
                        theme.text.monospace_font_stack.clone(),
                    ),
                    rang.clone(),
                );
                builder.push(
                    StyleProperty::Brush(BrushPalete::CODE_BRUSH),
                    rang,
                );
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    Bold,
    Italic,
    Strikethrough,
    InlineCode,
}

