use core::fmt;

use kurbo::{Affine, Cap, Insets, Join, Line, Point, Rect, Size, Stroke, Vec2};
use masonry::core::BrushIndex;
use parley::{Alignment, FontFamily, FontStack, StyleProperty};
use peniko::Color;
use pulldown_cmark::HeadingLevel;
use vello::Scene;
use xilem::FontWeight;

use super::{
    context::{MarkdownContext, TextContext},
    text::{
        layouted_text::LayoutedText, simple::SimpleText, styles::BrushPalete,
        MarkdownText,
    },
};
use crate::{
    basic_types::{Height, Width},
    layout_flow::{LayoutData, LayoutFlow},
    theme::{self, MarkdowTheme},
};

#[derive(Clone, Debug)]
struct Margin {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

impl Margin {
    fn new(top: f64, right: f64, bottom: f64, left: f64) -> Margin {
        Margin {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const ZERO: Margin = Margin {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    /// This function takes width and `f` function that layouts the inside of
    /// the `Margin` based on inner width. The `f` function will get the width
    /// reduced by the margin. Also the returned value from `layout_by_width`
    /// will be modified by the margin.
    fn layout_by_width<F>(&self, width: Width, f: F) -> Height
    where
        F: FnOnce(Width) -> Height,
    {
        let margin_width = self.left + self.right;
        let margin_height = self.top + self.bottom;
        let new_width = width - margin_width;
        f(new_width) + margin_height
    }

    /// This function takes paint Rect and adjusts it with the margin. The
    /// adjusted rect is then given to the `f` function for painting.
    fn paint<F>(&self, element_box: &Rect, f: F)
    where
        F: FnOnce(&Rect),
    {
        let element_box = Rect::new(
            element_box.x0 + self.left,
            element_box.y0 + self.top,
            element_box.x1 - self.right,
            element_box.y1 - self.bottom,
        );
        f(&element_box);
    }

    fn height(&self) -> Height {
        self.top + self.bottom
    }

    fn width(&self) -> Width {
        self.left + self.right
    }
}

impl From<theme::Margin> for Margin {
    fn from(value: theme::Margin) -> Self {
        Margin {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
        }
    }
}

impl From<theme::Padding> for Margin {
    fn from(value: theme::Padding) -> Self {
        Margin {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
        }
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
        let mut text_ctx: TextContext =
            TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);
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
                    symbol.build_layout(&mut text_ctx, None);
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

        self.margin.top = ctx.theme.markdown.list_top_margin;
        if reduce_top_margin {
            self.margin.top = 0.0;
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
        element_box: &Rect,
        index: usize,
        brush_palete: &BrushPalete,
        flow: &LayoutFlow<MarkdownContent>,
    ) {
        match &self.marker {
            ListMarker::Symbol { symbol } => {
                let marker_position = element_box.origin().to_vec2()
                    + Vec2::new(ctx.theme.markdown.bullet_list_indentation, 0.0);
                symbol.draw_text(scene, scene_size, &marker_position, brush_palete);
            }
            ListMarker::Numbers {
                start_number: _,
                layouted,
            } => {
                let mut marker_position = element_box.origin().to_vec2();
                marker_position.x += self.indentation
                    - layouted[index].full_width()
                    - ctx.theme.markdown.list_after_indentation;
                layouted[index].draw_text(
                    scene,
                    scene_size,
                    &marker_position,
                    brush_palete,
                );
            }
        }
        let element_box = element_box.inset(Insets::new(
            self.indentation,
            0.0,
            self.indentation,
            0.0,
        ));
        draw_flow(scene, scene_size, ctx, &element_box, brush_palete, flow);
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        self.margin.paint(element_box, |element_box: &Rect| {
            let mut element_box = *element_box;
            for (index, flow) in self.list.iter().enumerate() {
                self.paint_one_element(
                    scene,
                    scene_size,
                    ctx,
                    &element_box,
                    index,
                    brush_palete,
                    flow,
                );
                element_box.y0 += flow.height();
            }
        });
    }
}

#[derive(Clone)]
pub enum ListMarker {
    Symbol {
        symbol: Box<SimpleText>,
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
        self.margin.top = ctx.theme.markdown.paragraph_top_margin;
        if reduce_top_margin {
            self.margin.top = 0.0;
        }

        self.margin.layout_by_width(width, |width| {
            let mut text_ctx: TextContext =
                TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);
            self.text
                .load_and_layout_text(&mut text_ctx, &[], &[], width);
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height() + self.text.height()
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        self.margin.paint(element_box, |element_box| {
            self.text.draw_text(
                scene,
                scene_size,
                &element_box.origin().to_vec2(),
                brush_palete,
            );
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

    fn layout(&mut self, ctx: &mut MarkdownContext, width: Width) -> Height {
        let margin = ctx.theme.markdown.code_block_margin;
        self.margin = Margin::new(margin, margin, margin, margin);

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
        self.margin.height() + self.text.height()
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        self.margin.paint(element_box, |element_box: &Rect| {
            self.text.draw_text(
                scene,
                scene_size,
                &element_box.origin().to_vec2(),
                brush_palete,
            );
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndentationDecoration {
    Indentation,
    Note,
    Important,
    Tip,
    Warning,
    Caution,
}

impl IndentationDecoration {
    fn color(&self, theme: &MarkdowTheme) -> (Color, BrushIndex) {
        match self {
            IndentationDecoration::Indentation => (
                theme.standard_quotation.color,
                BrushPalete::INDENTATION_BRUSH,
            ),
            IndentationDecoration::Note => {
                (theme.box_quotation.note_color, BrushPalete::NOTE_BRUSH)
            }
            IndentationDecoration::Important => (
                theme.box_quotation.important_color,
                BrushPalete::IMPORTANT_BRUSH,
            ),
            IndentationDecoration::Tip => {
                (theme.box_quotation.tip_color, BrushPalete::TIP_BRUSH)
            }
            IndentationDecoration::Warning => (
                theme.box_quotation.warning_color,
                BrushPalete::WARNING_BRUSH,
            ),
            IndentationDecoration::Caution => (
                theme.box_quotation.caution_color,
                BrushPalete::CAUTION_BRUSH,
            ),
        }
    }
}

#[derive(Clone)]
pub struct Indented {
    margin: Margin,
    padding: Margin,
    decoration_margin: Margin,
    decoration: IndentationDecoration,
    flow: LayoutFlow<MarkdownContent>,
    symbol: LayoutedText,
    height: Height,
}

impl std::fmt::Debug for Indented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Margin")
            .field("margin", &self.margin)
            .field("padding", &self.padding)
            .field("decoration_margin", &self.decoration_margin)
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
            decoration_margin: Margin::ZERO,
            symbol: LayoutedText::empty(),
            height: 0.0,
        }
    }

    fn layout(&mut self, ctx: &mut MarkdownContext, width: Width) -> Height {
        let theme = &ctx.theme.markdown;
        if IndentationDecoration::Indentation == self.decoration {
            let theme = &ctx.theme.markdown.standard_quotation;
            self.margin = theme.margine.clone().into();
            self.padding = Margin::new(0.0, 0.0, 0.0, theme.line_horizontal_padding);
            self.decoration_margin = Margin::new(0.0, 0.0, 0.0, theme.line_width);
        } else {
            let theme = &ctx.theme.markdown.box_quotation;

            self.margin = theme.margin.clone().into();
            self.padding = theme.box_padding.clone().into();

            let mut symbol: LayoutedText = match self.decoration {
                IndentationDecoration::Indentation => "".to_string().into(),
                IndentationDecoration::Note => theme.note_sign.clone().into(),
                IndentationDecoration::Important => {
                    theme.important_sign.clone().into()
                }
                IndentationDecoration::Tip => theme.tip_sign.clone().into(),
                IndentationDecoration::Warning => theme.warning_sign.clone().into(),
                IndentationDecoration::Caution => theme.caution_sign.clone().into(),
            };

            let (_color, brush) = self.decoration.color(&ctx.theme.markdown);

            symbol.build_layout(ctx.layout_ctx, ctx.theme.scale, None, |builder| {
                BrushPalete::fill_default_styles(ctx.theme, builder);
                builder.push_default(StyleProperty::FontStack(FontStack::Single(
                    // TODO: This should be sourced from theme
                    FontFamily::Named("Symbols Nerd Font".into()),
                )));
                builder.push_default(StyleProperty::Brush(brush));
            });

            self.symbol = symbol;

            self.decoration_margin = Margin::new(
                theme.box_line_width,
                theme.box_line_width,
                theme.box_line_width,
                (theme.box_line_width * 2.0)
                    + theme.symbol_padding.left
                    + theme.symbol_padding.right
                    + self.symbol.full_width(),
            );
        }

        self.margin.layout_by_width(width, |width| {
            self.decoration_margin.layout_by_width(width, |width| {
                self.padding.layout_by_width(width, |width| {
                    self.flow.apply_to_all(|(i, data)| {
                        data.layout(ctx, width, i == 0);
                    });
                    self.flow.height()
                })
            })
        });

        let symbol_padding: Margin =
            theme.box_quotation.symbol_padding.clone().into();

        let box_height = self.padding.height()
            + self.flow.height()
            + self.decoration_margin.height()
            + self.margin.height();
        let symbol_height = symbol_padding.height()
            + self.symbol.height()
            + self.decoration_margin.height()
            + self.margin.height();
        self.height = box_height.max(symbol_height);
        self.height
    }

    fn height(&self) -> Height {
        self.height
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        ctx: &mut MarkdownContext,
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        self.margin.paint(element_box, |element_box: &Rect| {
            let theme = &ctx.theme.markdown;

            let (color, _brush) = self.decoration.color(&ctx.theme.markdown);

            match self.decoration {
                IndentationDecoration::Indentation => {
                    let theme = &theme.standard_quotation;
                    let x0 = theme.line_width / 2.0;
                    let y1 = 0.0;
                    let y2 = element_box.height();
                    let underline_shape = Line::new((x0, y1), (x0, y2));

                    let stroke = Stroke {
                        width: theme.line_width,
                        join: Join::Bevel,
                        miter_limit: 4.0,
                        start_cap: Cap::Round,
                        end_cap: Cap::Round,
                        dash_pattern: Default::default(),
                        dash_offset: 0.0,
                    };

                    let transform =
                        Affine::translate(element_box.origin().to_vec2());

                    scene.stroke(
                        &stroke,
                        transform,
                        color,
                        Some(Affine::IDENTITY),
                        &underline_shape,
                    );
                }
                _ => {
                    let theme = &theme.box_quotation;

                    let symbol_padding: Margin = theme.symbol_padding.clone().into();
                    let half_line_width = theme.box_line_width / 2.0;
                    let x0 = half_line_width;
                    let y0 = half_line_width;
                    let x1 = element_box.width() - half_line_width;
                    let y1 = element_box.height() - half_line_width;
                    let box_shape = Rect::new(x0, y0, x1, y1);

                    let stroke = Stroke {
                        width: theme.box_line_width,
                        join: Join::Bevel,
                        miter_limit: 4.0,
                        start_cap: Cap::Round,
                        end_cap: Cap::Round,
                        dash_pattern: Default::default(),
                        dash_offset: 0.0,
                    };

                    let transform =
                        Affine::translate(element_box.origin().to_vec2());

                    scene.stroke(
                        &stroke,
                        transform,
                        color,
                        Some(Affine::IDENTITY),
                        &box_shape,
                    );

                    let x0 = theme.box_line_width
                        + symbol_padding.width()
                        + self.symbol.full_width()
                        + half_line_width;
                    let box_shape =
                        Line::new(Point::new(x0, y0), Point::new(x0, y1));

                    let stroke = Stroke {
                        width: theme.box_line_width,
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

                    let x = theme.box_line_width + symbol_padding.left;
                    let y = theme.box_line_width + symbol_padding.top;

                    self.symbol.draw_text(
                        scene,
                        scene_size,
                        &(element_box.origin().to_vec2() + Vec2::new(x, y)),
                        |_| None,
                        &brush_palete.palete,
                    );
                }
            };
            self.decoration_margin.paint(element_box, |element_box| {
                self.padding.paint(element_box, |element_box| {
                    draw_flow(
                        scene,
                        scene_size,
                        ctx,
                        element_box,
                        brush_palete,
                        &self.flow,
                    )
                })
            })
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
        self.margin.top = ctx.theme.markdown.paragraph_top_margin;
        if reduce_top_margin {
            self.margin.top = 0.0;
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

        let mut text_ctx: TextContext =
            TextContext::new(ctx.svg_ctx, ctx.layout_ctx, ctx.theme);

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
        self.margin.height() + self.text.height()
    }

    fn paint(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        _ctx: &mut MarkdownContext,
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        self.margin.paint(element_box, |element_box: &Rect| {
            self.text.draw_text(
                scene,
                scene_size,
                &element_box.origin().to_vec2(),
                brush_palete,
            );
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
            if reduce_top_margin {
                0.0
            } else {
                ctx.theme.markdown.horizontal_line_vertical_margin
            },
            ctx.theme.markdown.horizontal_line_horizontal_margin,
            ctx.theme.markdown.horizontal_line_vertical_margin,
            ctx.theme.markdown.horizontal_line_horizontal_margin,
        );
        self.height = ctx.theme.markdown.horizontal_line_height;
        self.margin.layout_by_width(width, |_width| self.height)
    }

    fn height(&self) -> Height {
        self.margin.height() + self.height
    }

    fn paint(
        &self,
        scene: &mut Scene,
        ctx: &mut MarkdownContext,
        element_box: &Rect,
    ) {
        self.margin.paint(element_box, |element_box: &Rect| {
            let y1 = ctx.theme.markdown.horizontal_line_height / 2.0;
            let x1 = 0.0;
            let x2 = element_box.width();
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

            let transform = Affine::translate(element_box.origin().to_vec2());

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
        element_box: &Rect,
        brush_palete: &BrushPalete,
    ) {
        match self {
            MarkdownContent::Paragraph(paragraph) => {
                paragraph.paint(scene, scene_size, ctx, element_box, brush_palete);
            }
            MarkdownContent::CodeBlock(code_block) => {
                code_block.paint(scene, scene_size, ctx, element_box, brush_palete);
            }
            MarkdownContent::Indented(indented) => {
                indented.paint(scene, scene_size, ctx, element_box, brush_palete);
            }
            MarkdownContent::List(list) => {
                list.paint(scene, scene_size, ctx, element_box, brush_palete);
            }
            MarkdownContent::HorizontalLine(horizontal_line) => {
                horizontal_line.paint(scene, ctx, element_box);
            }
            MarkdownContent::Header(header) => {
                header.paint(scene, scene_size, ctx, element_box, brush_palete);
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
    element_box: &Rect,
    brush_palete: &BrushPalete,
    flow: &LayoutFlow<MarkdownContent>,
) {
    let position = element_box.origin();
    let offset = if position.y < 0.0 { -position.y } else { 0.0 };
    let height = if position.y > 0.0 {
        scene_size.height - position.y
    } else {
        scene_size.height
    };
    let visible_parts = flow.get_visible_parts(offset, height);

    for visible_part in visible_parts.iter() {
        let element_box: Rect = Rect::new(
            position.x,
            position.y + visible_part.offset,
            element_box.x1,
            position.y + visible_part.offset + visible_part.height,
        );
        visible_part
            .data
            .paint(scene, scene_size, ctx, &element_box, brush_palete);
    }
}
