use std::sync::Arc;

use masonry::core::BrushIndex;
use parley::FontContext;
use usvg::fontdb;

use crate::theme::Theme;

pub struct SvgContext {
    pub fontdb: Arc<fontdb::Database>,
}

pub struct MarkdownContext<'a, 'b> {
    pub svg_ctx: &'a SvgContext,
    pub layout_ctx: &'a mut LayoutContext<'b>,
    pub theme: &'a Theme,
}

pub struct LayoutContext<'a> {
    pub font_ctx: &'a mut parley::FontContext,
    pub layout_ctx: &'a mut parley::LayoutContext<BrushIndex>,
}

impl<'a> LayoutContext<'a> {
    pub fn new(
        font_ctx: &'a mut FontContext,
        layout_ctx: &'a mut parley::LayoutContext<BrushIndex>,
    ) -> LayoutContext<'a> {
        LayoutContext {
            font_ctx,
            layout_ctx,
        }
    }
}

pub struct TextContext<'a, 'b> {
    pub layout_ctx: &'a mut LayoutContext<'b>,
    pub svg_ctx: &'a SvgContext,
    pub theme: &'a Theme,
}

impl SvgContext {
    pub fn new(fontdb: Arc<fontdb::Database>) -> SvgContext {
        SvgContext { fontdb }
    }
}

impl<'a, 'b> TextContext<'a, 'b> {
    pub fn new(
        svg_ctx: &'a SvgContext,
        layout_ctx: &'a mut LayoutContext<'b>,
        theme: &'a Theme,
    ) -> TextContext<'a, 'b> {
        TextContext {
            svg_ctx,
            layout_ctx,
            theme,
        }
    }
}

impl<'a, 'b> MarkdownContext<'a, 'b> {
    pub fn new(
        svg_ctx: &'a SvgContext,
        layout_ctx: &'a mut LayoutContext<'b>,
        theme: &'a Theme,
    ) -> MarkdownContext<'a, 'b> {
        MarkdownContext {
            svg_ctx,
            layout_ctx,
            theme,
        }
    }
}
