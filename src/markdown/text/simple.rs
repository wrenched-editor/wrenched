use kurbo::{Size, Vec2};
use parley::Alignment;
use vello::Scene;

use crate::markdown::context::TextContext;

use super::{layouted_text::LayoutedText, styles::BrushPalete};

#[derive(Clone, Debug)]
pub struct SimpleText {
    text: LayoutedText,
}

impl SimpleText {
    pub fn new(text: String) -> SimpleText {
        SimpleText {
            text: LayoutedText::new(text)
        }
    }

    pub fn empty() -> SimpleText {
        SimpleText {
            text: LayoutedText::new("".into())
        }
    }

    pub fn build_layout(
        &mut self,
        text_ctx: &mut TextContext,
        max_advance: Option<f64>,
    ) {
        self.text.build_layout(text_ctx.layout_ctx, text_ctx.theme.scale,max_advance ,|builder| {
            BrushPalete::fill_default_styles(text_ctx.theme, builder);
        });
    }

    pub fn draw_text(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        position: &Vec2,
        brush_palete: &BrushPalete,
    ) {
        self.text.draw_text(
            scene,
            scene_size,
            position,
            |_|{None},
            &brush_palete.palete,
        );
    }

    pub fn height(&self) -> f64 {
        self.text.height()
    }

    pub fn full_width(&self) -> f64 {
        self.text.full_width()
    }

    pub fn align(
        &mut self,
        container_width: Option<f32>,
        alignment: Alignment,
        align_when_overflowing: bool,
    ) {
        self.text.align(container_width, alignment, align_when_overflowing);
    }
}

impl From<String> for SimpleText {
    fn from(value: String) -> Self {
        SimpleText::new(value)
    }
}
