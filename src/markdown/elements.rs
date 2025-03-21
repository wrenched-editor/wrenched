use core::fmt;

use image;
use kurbo::{Affine, Cap, Insets, Join, Line, Rect, Size, Stroke, Vec2};
use masonry::core::BrushIndex;
use parley::{Alignment, FontFamily, FontStack, StyleProperty};
use peniko::Color;
use pulldown_cmark::HeadingLevel;
use vello::Scene;
use xilem::FontWeight;

use super::{
    context::{MarkdownContext, TextContext},
    text::{layouted_text::LayoutedText, simple::SimpleText, styles::BrushPalete, MarkdownText},
};
use crate::{
    basic_types::{Height, Width},
    layout_flow::{LayoutData, LayoutFlow},
    theme::MarkdowTheme,
};

#[derive(Clone, Debug)]
struct Margin {
    insets: Insets,
}

impl Margin {
    fn new(left: f64, top: f64, right: f64, bottom: f64) -> Margin {
        Margin {
            insets: Insets::new(left, top, right, bottom),
        }
    }

    pub const ZERO: Margin = Margin {
        insets: Insets::ZERO,
    };

    /// This function takes width and `f` function that layouts the inside of
    /// the `Margin` based on inner width. The `f` function will get the width
    /// reduced by the margin. Also the returned value from `layout_by_width`
    /// will be modified by the margin.
    fn layout_by_width<F>(&self, width: Width, f: F) -> Height
    where
        F: FnOnce(Width) -> Height,
    {
        let new_width = width - self.insets.x_value();
        f(new_width) + self.insets.y_value()
    }

    /// This function takes paint Rect and adjusts it with the margin. The
    /// adjusted rect is then given to the `f` function for painting.
    fn paint<F>(&self, position: &Vec2, element_size: &Size, f: F)
    where
        F: FnOnce(&Vec2, &Size),
    {
        let new_position = *position + Vec2::new(self.insets.x0, self.insets.y0);
        let new_size =
            *element_size - Size::new(self.insets.x_value(), self.insets.y_value());
        f(&new_position, &new_size);
    }

    /// This function takes paint Rect and adjusts it with the margin. The
    /// adjusted rect is then given to the `f` function for painting.
    fn height<F>(&self, f: F) -> Height
    where
        F: FnOnce() -> Height,
    {
        f() + self.insets.y_value()
    }
}

#[derive(Clone, Debug)]
pub struct MarkdownList {
    margin: Margin,
    list: Vec<LayoutFlow<MarkdownContent>>,
    marker: ListMarker,
    indentation: f64,
    height: f64,
}

impl MarkdownList {
    pub fn new(
        list: Vec<LayoutFlow<MarkdownContent>>,
        marker: ListMarker,
    ) -> MarkdownList {
        Self {
            margin: Margin::ZERO,
            list,
            marker,
            indentation: 0.0,
            height: 0.0,
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: f64,
        reduce_top_margin: bool,
    ) -> Height {
        let mut text_ctx: TextContext = TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);
        self.indentation = match &mut self.marker {
            ListMarker::Symbol { symbol } => {
                symbol.build_layout(&mut text_ctx, None);
                symbol.full_width()
                    + ctx.theme.markdown.bullet_list_indentation
                    + ctx.theme.markdown.list_after_indentation
            }
            ListMarker::Numbers {
                start_number,
                layouted,
            } => {
                let mut max_marker_width: f64 = 0.0;
                layouted.clear();
                for k in 0..self.list.len() {
                    // Not ideal way to layout the numbered list, but works for now.
                    let mut str = (k as u32 + *start_number).to_string();
                    str.push('.');
                    let mut symbol: SimpleText = str.into();
                    symbol.align(None, Alignment::End, false);
                    let marker_width = symbol.full_width()
                        + ctx.theme.markdown.numbered_list_indentation
                        + ctx.theme.markdown.list_after_indentation;
                    if marker_width > max_marker_width {
                        max_marker_width = marker_width;
                    }
                    layouted.push(symbol);
                }
                max_marker_width
            }
        };

        self.margin.insets.y0 = ctx.theme.markdown.list_top_margin;
        if reduce_top_margin {
            self.margin.insets.y0 = 0.0;
        }

        self.height = self.margin.layout_by_width(width, |width| {
            let mut height = 0.0;
            for element in self.list.iter_mut() {
                element.apply_to_all(|(i, data)| {
                    data.layout(
                        ctx,
                        width - self.indentation,
                        i == 0 || (i == 1 && data.is_list()),
                    );
                });
                height += element.height();
            }
            height
        });
        self.height
    }

    fn height(&self) -> f64 {
        self.height
    }

    fn paint_one_element(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        index: usize,
        brush_palete: &BrushPalete,
        flow: &LayoutFlow<MarkdownContent>,
    ) {
        match &self.marker {
            ListMarker::Symbol { symbol } => {
                let marker_position = *position
                    + Vec2::new(ctx.theme.markdown.bullet_list_indentation, 0.0);
                symbol.draw_text(scene, scene_size, &marker_position, brush_palete);
            }
            ListMarker::Numbers {
                start_number: _,
                layouted,
            } => {
                let mut marker_position = *position;
                marker_position.x += self.indentation
                    - layouted[index].full_width()
                    - ctx.theme.markdown.list_after_indentation;
                layouted[index].draw_text(scene, scene_size, &marker_position, brush_palete);
            }
        }
        let item_position = *position + Vec2::new(self.indentation, 0.0);
        let item_size = *element_size - Size::new(self.indentation, 0.0);
        draw_flow(scene, scene_size, ctx, &item_position, &item_size, brush_palete, flow);
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        self.margin
            .paint(position, element_size, |position, element_size| {
                let mut position = *position;
                for (index, flow) in self.list.iter().enumerate() {
                    self.paint_one_element(
                        scene,
                        scene_size,
                        ctx,
                        &position,
                        element_size,
                        index,
                        brush_palete,
                        flow,
                    );
                    position.y += flow.height();
                }
            });
    }
}

#[derive(Clone)]
pub enum ListMarker {
    Symbol {
        symbol: SimpleText,
    },
    Numbers {
        start_number: u32,
        layouted: Vec<SimpleText>,
    },
}

impl fmt::Debug for ListMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListMarker::Symbol { symbol } => {
                write!(f, "ListMarker::Symbol {{ symbol: {:?} }}", symbol)
            }
            ListMarker::Numbers {
                start_number,
                layouted: _,
            } => write!(
                f,
                "ListMarker::Numbers {{ start_number: {} }}",
                start_number
            ),
        }
    }
}

enum ImageType {
    Svg,
    Rasterized(image::ImageFormat),
}

#[derive(Clone, Debug)]
pub struct Paragraph {
    text: MarkdownText,
    margin: Margin,
}

impl Paragraph {
    pub fn new(text: MarkdownText) -> Paragraph {
        Paragraph {
            text,
            margin: Margin::ZERO,
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: Width,
        reduce_top_margin: bool,
    ) -> Height {
        self.margin.insets.y0 = ctx.theme.markdown.paragraph_top_margin;
        if reduce_top_margin {
            self.margin.insets.y0 = 0.0;
        }

        self.margin.layout_by_width(width, |width| {
        let mut text_ctx: TextContext = TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);
            self.text
                .load_and_layout_text(&mut text_ctx, &[], &[], width);
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text
                    .draw_text(scene, scene_size, position, brush_palete);
            });
    }
}

#[derive(Clone, Debug)]
pub struct CodeBlock {
    text: MarkdownText,
    margin: Margin,
    // TODO: Use the language to do some syntax highlighting
    _language: Option<String>,
}

impl CodeBlock {
    pub fn new(str: String, language: Option<String>) -> CodeBlock {
        CodeBlock {
            text: MarkdownText::new(str, Vec::new(), Vec::new(), Vec::new()),
            margin: Margin::ZERO,
            _language: language,
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: Width,
    ) -> Height {
        self.margin.insets = Insets::uniform(ctx.theme.markdown.code_block_margin);

        let extra_default_styles = vec![
            StyleProperty::FontStack(ctx.theme.text.monospace_font_stack.clone()),
            StyleProperty::Brush(BrushPalete::CODE_BRUSH),
        ];

        let mut text_ctx: TextContext = TextContext {
            layout_ctx: ctx.layout_ctx,
            svg_ctx: ctx.svg_ctx,
            theme: ctx.theme,
        };

        self.margin.layout_by_width(width, |width| {
            self.text.load_and_layout_text(
                &mut text_ctx,
                &extra_default_styles,
                &[],
                width,
            );
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text
                    .draw_text(scene, scene_size, position, brush_palete);
            });
    }
}

#[derive(Clone, Debug)]
pub enum IndentationDecoration {
    Indentation,
    Note,
    Important,
    Tip,
    Warning,
    Caution,
}

impl IndentationDecoration {
    fn padding_and_symbol(
        &self,
        ctx: &mut MarkdownContext,
    ) -> (Margin, LayoutedText) {
        let theme = &ctx.theme.markdown;
        let left = match self {
            IndentationDecoration::Indentation => {
                theme.indentation_line_width + theme.indentation_line_margine * 2.0
            }
            _ => theme.indentation_box_margin + theme.indentation_box_line_width,
        };
        let top = match self {
            IndentationDecoration::Indentation => 0.0,
            _ => theme.indentation_box_margin + theme.indentation_box_line_width,
        };
        let right = match self {
            IndentationDecoration::Indentation => 0.0,
            _ => theme.indentation_box_margin + theme.indentation_box_line_width,
        };
        let bottom = match self {
            IndentationDecoration::Indentation => 0.0,
            _ => theme.indentation_box_margin + theme.indentation_box_line_width,
        };

        let mut symbol: LayoutedText = match self {
            IndentationDecoration::Indentation => "".to_string().into(),
            IndentationDecoration::Note => theme.indentation_note_sign.clone().into(),
            IndentationDecoration::Important => {
                theme.indentation_important_sign.clone().into()
            }
            IndentationDecoration::Tip => theme.indentation_tip_sign.clone().into(),
            IndentationDecoration::Warning => theme.indentation_warning_sign.clone().into(),
            IndentationDecoration::Caution => theme.indentation_caution_sign.clone().into(),
        };

        let (_color, brush) = self.color(theme);

        symbol.build_layout(ctx.layout_ctx, ctx.theme.scale, None, |builder|{
            BrushPalete::fill_default_styles(ctx.theme, builder);
            builder.push_default(StyleProperty::FontStack(FontStack::Single(
                FontFamily::Named("FiraCode Nerd Font".into()),
            )));
            builder.push_default(StyleProperty::Brush(brush));
        });

        let additional_left_padding = if symbol.is_empty() {
            0.0
        } else {
            // TODO: This should be themeable???
            symbol.full_width()
                + (theme.indentation_sign_horizontal_padding * 2.0)
        };

        (
            Margin::new(left + additional_left_padding, top, right, bottom),
            symbol
        )
    }

    fn color(&self, theme: &MarkdowTheme) -> (Color, BrushIndex)  {
        match self {
            IndentationDecoration::Indentation => (theme.indentation_color, BrushPalete::INDENTATION_BRUSH),
            IndentationDecoration::Note => (theme.indentation_note_color, BrushPalete::NOTE_BRUSH),
            IndentationDecoration::Important => (theme.indentation_important_color, BrushPalete::IMPORTANT_BRUSH),
            IndentationDecoration::Tip => (theme.indentation_tip_color, BrushPalete::TIP_BRUSH),
            IndentationDecoration::Warning => (theme.indentation_warning_color, BrushPalete::WARNING_BRUSH),
            IndentationDecoration::Caution => (theme.indentation_caution_color, BrushPalete::CAUTION_BRUSH),
        }
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        symbol: &LayoutedText,
        padding: &Margin,
        brush_palete: &BrushPalete,
    ) {
        let theme = &ctx.theme.markdown;
        let (color, _brush) = self.color(&ctx.theme.markdown);
        match self {
            IndentationDecoration::Indentation => {
                let x0 = theme.indentation_line_margine
                    + theme.indentation_line_width / 2.0;
                let y1 = 0.0;
                let y2 = element_size.height;
                let underline_shape = Line::new((x0, y1), (x0, y2));

                let stroke = Stroke {
                    width: theme.indentation_line_width,
                    join: Join::Bevel,
                    miter_limit: 4.0,
                    start_cap: Cap::Round,
                    end_cap: Cap::Round,
                    dash_pattern: Default::default(),
                    dash_offset: 0.0,
                };

                let transform = Affine::translate(*position);

                scene.stroke(
                    &stroke,
                    transform,
                    color,
                    Some(Affine::IDENTITY),
                    &underline_shape,
                );
            }
            _ => {
                let margin = theme.indentation_box_margin;
                let half_margin = margin / 2.0;
                let x0 = half_margin + (theme.indentation_line_width / 2.0);
                let y0 = half_margin;
                let x1 = element_size.width - half_margin;
                let y1 = element_size.height - half_margin;
                let box_shape = Rect::new(x0, y0, x1, y1);

                let stroke = Stroke {
                    width: theme.indentation_box_line_width,
                    join: Join::Bevel,
                    miter_limit: 4.0,
                    start_cap: Cap::Round,
                    end_cap: Cap::Round,
                    dash_pattern: Default::default(),
                    dash_offset: 0.0,
                };

                let transform = Affine::translate(*position);

                scene.stroke(
                    &stroke,
                    transform,
                    color,
                    Some(Affine::IDENTITY),
                    &box_shape,
                );

                let x1 =
                    padding.insets.x0 - (theme.indentation_box_line_width * 2.0);
                let box_shape = Rect::new(x0, y0, x1, y1);

                let stroke = Stroke {
                    width: ctx.theme.markdown.indentation_box_line_width,
                    join: Join::Bevel,
                    miter_limit: 4.0,
                    start_cap: Cap::Round,
                    end_cap: Cap::Round,
                    dash_pattern: Default::default(),
                    dash_offset: 0.0,
                };

                scene.stroke(
                    &stroke,
                    transform,
                    color,
                    Some(Affine::IDENTITY),
                    &box_shape,
                );

                let x = (padding.insets.x0
                    - symbol.full_width()
                    - theme.indentation_box_line_width)
                    / 2.0;
                let y = padding.insets.y0; //theme.indentation_sign_top_padding;

                symbol.draw_text(scene, scene_size, &(*position + Vec2::new(x, y)), |_|{None}, &brush_palete.palete);
            }
        }
    }
}

#[derive(Clone)]
pub struct Indented {
    margin: Margin,
    padding: Margin,
    decoration: IndentationDecoration,
    flow: LayoutFlow<MarkdownContent>,
    symbol: LayoutedText,
}

impl std::fmt::Debug for Indented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Margin")
            .field("margin", &self.margin)
            .field("padding", &self.padding)
            .field("flow", &self.flow)
            .finish()
    }
}

impl Indented {
    pub fn new(
        decoration: IndentationDecoration,
        flow: LayoutFlow<MarkdownContent>,
    ) -> Indented {
        Indented {
            decoration,
            flow,
            margin: Margin::ZERO,
            padding: Margin::ZERO,
            symbol: LayoutedText::empty(),
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: Width,
    ) -> Height {
        self.margin.insets.x0 = ctx.theme.markdown.indentation_horizonatl_margin;
        self.margin.insets.x1 = ctx.theme.markdown.indentation_horizonatl_margin;
        self.margin.insets.y0 = ctx.theme.markdown.indentation_vertical_margin;
        self.margin.insets.y1 = ctx.theme.markdown.indentation_vertical_margin;

        let (padding, symbol) = self.decoration.padding_and_symbol(ctx);
        self.padding = padding;
        self.symbol = symbol;

        self.margin.layout_by_width(width, |width| {
            self.flow.apply_to_all(|(i, data)| {
                self.padding
                    .layout_by_width(width, |width| data.layout(ctx, width, i == 0));
            });
            self.flow.height()
        })
    }

    fn height(&self) -> Height {
        self.margin
            .height(|| self.padding.height(|| self.flow.height()))
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        self.margin
            .paint(position, element_size, |position, element_size| {
                self.decoration.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    &self.symbol,
                    &self.padding,
                    brush_palete,
                );
                self.padding.paint(
                    position,
                    element_size,
                    |position, element_size| {
                        draw_flow(scene, scene_size, ctx, position, element_size, brush_palete, &self.flow);
                    },
                )
            });
    }
}

#[derive(Clone, Debug)]
pub struct Header {
    margin: Margin,
    text: MarkdownText,
    level: HeadingLevel,
}

impl Header {
    pub fn new(text: MarkdownText, level: HeadingLevel) -> Header {
        Header {
            margin: Margin::ZERO,
            text,
            level,
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: Width,
        reduce_top_margin: bool,
    ) -> Height {
        self.margin.insets.y0 = ctx.theme.markdown.paragraph_top_margin;
        if reduce_top_margin {
            self.margin.insets.y0 = 0.0;
        }
        let extra_default_styles = vec![
            StyleProperty::FontSize(match self.level {
                HeadingLevel::H1 => ctx.theme.text.text_size as f32 * 2.125,
                HeadingLevel::H2 => ctx.theme.text.text_size as f32 * 1.875,
                HeadingLevel::H3 => ctx.theme.text.text_size as f32 * 1.5,
                HeadingLevel::H4 => ctx.theme.text.text_size as f32 * 1.25,
                HeadingLevel::H5 => ctx.theme.text.text_size as f32 * 1.125,
                HeadingLevel::H6 => ctx.theme.text.text_size as f32,
            }),
            StyleProperty::LineHeight(ctx.theme.markdown.header_line_height),
            StyleProperty::FontWeight(FontWeight::BOLD),
        ];

        let mut text_ctx: TextContext = TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);

        self.margin.layout_by_width(width, |width| {
            self.text.load_and_layout_text(
                &mut text_ctx,
                &extra_default_styles,
                &[],
                width,
            );
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text
                    .draw_text(scene, scene_size, position, brush_palete);
            });
    }
}

#[derive(Clone, Debug)]
pub struct HorizontalLine {
    height: f64,
    margin: Margin,
}

impl HorizontalLine {
    pub fn new() -> HorizontalLine {
        HorizontalLine {
            margin: Margin::ZERO,
            height: 0.0,
        }
    }

    fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: Width,
        reduce_top_margin: bool,
    ) -> Height {
        self.margin = Margin::new(
            ctx.theme.markdown.horizontal_line_horizontal_margin,
            if reduce_top_margin {
                0.0
            } else {
                ctx.theme.markdown.horizontal_line_vertical_margin
            },
            ctx.theme.markdown.horizontal_line_horizontal_margin,
            ctx.theme.markdown.horizontal_line_vertical_margin,
        );
        self.height = ctx.theme.markdown.horizontal_line_height;
        self.margin.layout_by_width(width, |_width| self.height)
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.height)
    }

    fn paint(
        &self,
        scene: &mut Scene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                let y1 = ctx.theme.markdown.horizontal_line_height / 2.0;
                let x1 = 0.0;
                let x2 = element_size.width;
                let underline_shape = Line::new((x1, y1), (x2, y1));

                let stroke = Stroke {
                    width: ctx.theme.markdown.horizontal_line_height,
                    join: Join::Bevel,
                    miter_limit: 4.0,
                    start_cap: Cap::Round,
                    end_cap: Cap::Round,
                    dash_pattern: Default::default(),
                    dash_offset: 0.0,
                };

                let transform = Affine::translate(*position);

                scene.stroke(
                    &stroke,
                    transform,
                    ctx.theme.markdown.horizontal_line_color,
                    Some(Affine::IDENTITY),
                    &underline_shape,
                );
            });
    }
}

impl Default for HorizontalLine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum MarkdownContent {
    Indented(Indented),
    Header(Header),
    List(MarkdownList),
    Paragraph(Paragraph),
    CodeBlock(CodeBlock),
    HorizontalLine(HorizontalLine),
}

impl MarkdownContent {
    pub fn layout(
        &mut self,
        ctx: &mut MarkdownContext,
        width: f64,
        reduce_top_margin: bool,
    ) -> Height {
        match self {
            MarkdownContent::Paragraph(paragraph) => {
                paragraph.layout(ctx, width, reduce_top_margin)
            }
            MarkdownContent::CodeBlock(code_block) => code_block.layout(ctx, width),
            MarkdownContent::Indented(indented) => indented.layout(ctx, width),
            MarkdownContent::List(list) => {
                list.layout(ctx, width, reduce_top_margin)
            }
            MarkdownContent::HorizontalLine(horizontal_line) => {
                horizontal_line.layout(ctx, width, reduce_top_margin)
            }
            MarkdownContent::Header(header) => {
                header.layout(ctx, width, reduce_top_margin)
            }
        }
    }

    pub fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        brush_palete: &BrushPalete,
    ) {
        // TODO: Draw indentation decoration
        match self {
            MarkdownContent::Paragraph(paragraph) => {
                paragraph.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    brush_palete,
                );
            }
            // TODO: Add support for solo image
            MarkdownContent::CodeBlock(code_block) => {
                code_block.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    brush_palete,
                );
            }
            MarkdownContent::Indented(indented) => {
                indented.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    brush_palete,
                );
            }
            MarkdownContent::List(list) => {
                list.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    brush_palete,
                );
            }
            MarkdownContent::HorizontalLine(horizontal_line) => {
                horizontal_line.paint(scene, ctx, position, element_size);
            }
            MarkdownContent::Header(header) => {
                header.paint(
                    scene,
                    scene_size,
                    ctx,
                    position,
                    element_size,
                    brush_palete,
                );
            }
        }
    }

    pub fn height(&self) -> Height {
        match self {
            MarkdownContent::Indented(indented) => indented.height(),
            MarkdownContent::Header(header) => header.height(),
            MarkdownContent::List(markdown_list) => markdown_list.height(),
            MarkdownContent::Paragraph(paragraph) => paragraph.height(),
            MarkdownContent::CodeBlock(code_block) => code_block.height(),
            MarkdownContent::HorizontalLine(horizontal_line) => {
                horizontal_line.height()
            }
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, MarkdownContent::List(_))
    }
}

impl LayoutData for MarkdownContent {
    fn height(&self) -> Height {
        self.height()
    }
}

// TODO: Shoul this be a part of some markdown object??
pub fn draw_flow(
    scene: &mut Scene,
    scene_size: &Size,
    ctx: &mut MarkdownContext,
    position: &Vec2,
    element_size: &Size,
    brush_palete: &BrushPalete,
    flow: &LayoutFlow<MarkdownContent>,
) {
    let offset = if position.y < 0.0 { -position.y } else { 0.0 };
    let height = if position.y > 0.0 {
        scene_size.height - position.y
    } else {
        scene_size.height
    };
    let visible_parts = flow.get_visible_parts(offset, height);

    for visible_part in visible_parts.iter() {
        let part_position = *position + Vec2::new(0.0, visible_part.offset);
        let part_size = Size::new(element_size.width, visible_part.height);
        visible_part.data.paint(
            scene,
            scene_size,
            ctx,
            &part_position,
            &part_size,
            brush_palete,
        );
    }
}
