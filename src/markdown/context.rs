use parley::{FontContext, LayoutContext};
use usvg::fontdb;

use super::elements::MarkdownBrush;
use crate::theme::Theme;

pub struct SvgContext<'a> {
    pub fontdb: &'a fontdb::Database,
}

pub struct MarkdownContext<'a> {
    pub svg_ctx: &'a SvgContext<'a>,
    pub font_ctx: &'a mut FontContext,
    pub layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
    pub theme: &'a Theme,
}

pub struct TextContext<'a> {
    pub svg_ctx: &'a SvgContext<'a>,
    pub font_ctx: &'a mut FontContext,
    pub layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
    pub theme: &'a Theme,
}

impl<'a> SvgContext<'a> {
    pub fn new(fontdb: &'a fontdb::Database) -> SvgContext<'a> {
        SvgContext { fontdb }
    }
}

impl<'a> TextContext<'a> {
    pub fn new(
        svg_ctx: &'a SvgContext,
        font_ctx: &'a mut FontContext,
        layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
        theme: &'a Theme,
    ) -> TextContext<'a> {
        TextContext {
            svg_ctx,
            font_ctx,
            layout_ctx,
            theme,
        }
    }
}

impl<'a> MarkdownContext<'a> {
    pub fn new(
        svg_ctx: &'a SvgContext,
        font_ctx: &'a mut FontContext,
        layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
        theme: &'a Theme,
    ) -> MarkdownContext<'a> {
        MarkdownContext {
            svg_ctx,
            font_ctx,
            layout_ctx,
            theme,
        }
    }
}

impl <'a> From<MarkdownContext<'a>> for TextContext<'a> {
    fn from(value: MarkdownContext<'a>) -> TextContext<'a> {
        TextContext {
            svg_ctx: value.svg_ctx,
            font_ctx: value.font_ctx,
            layout_ctx: value.layout_ctx,
            theme: value.theme,
        }
    }
}
