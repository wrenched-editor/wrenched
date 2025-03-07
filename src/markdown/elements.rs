use core::fmt;
use std::{fs, ops::Range, path::Path, sync::Arc};

use image;
use kurbo::{Affine, Cap, Insets, Join, Line, Rect, Size, Stroke, Vec2};
use parley::{
    Alignment, Cluster, Decoration, FontContext, FontFamily, FontStack, FontStyle,
    GlyphRun, InlineBox, Layout, LayoutContext, PositionedLayoutItem, RangedBuilder,
    RunMetrics, StyleProperty,
};
use peniko::{Color, Fill, Image, ImageFormat};
use pulldown_cmark::HeadingLevel;
use usvg::fontdb;
use xilem::FontWeight;

use crate::{
    layout_flow::{LayoutData, LayoutFlow},
    scene_utils::SizedScene,
    theme::{get_theme, MarkdowTheme, Theme},
};

type Height = f64;
type Width = f64;

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
        self.indentation = match &mut self.marker {
            ListMarker::Symbol { symbol, layout } => {
                let mut builder =
                    str_to_builder(symbol, &[], ctx.font_ctx, ctx.layout_ctx);
                let mut marker_layout = builder.build(&symbol);
                marker_layout.break_all_lines(None);
                *layout = Box::new(marker_layout);
                layout.full_width() as f64
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
                    let mut builder =
                        str_to_builder(&str, &[], ctx.font_ctx, ctx.layout_ctx);
                    let mut marker_layout = builder.build(&str);
                    marker_layout.break_all_lines(None);
                    marker_layout.align(None, Alignment::End, false);
                    let marker_width = marker_layout.full_width() as f64
                        + ctx.theme.markdown.numbered_list_indentation
                        + ctx.theme.markdown.list_after_indentation;
                    if marker_width > max_marker_width {
                        max_marker_width = marker_width;
                    }
                    layouted.push(marker_layout);
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
        scene: &mut SizedScene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        index: usize,
        flow: &LayoutFlow<MarkdownContent>,
    ) {
        match &self.marker {
            ListMarker::Symbol { symbol: _, layout } => {
                let marker_position = *position
                    + Vec2::new(ctx.theme.markdown.bullet_list_indentation, 0.0);
                draw_text(layout, scene, &marker_position, &[]);
            }
            ListMarker::Numbers {
                start_number: _,
                layouted,
            } => {
                let mut marker_position = *position;
                marker_position.x += self.indentation
                    - layouted[index].full_width() as f64
                    - ctx.theme.markdown.list_after_indentation;
                draw_text(&layouted[index], scene, &marker_position, &[]);
            }
        }
        let item_position = *position + Vec2::new(self.indentation, 0.0);
        let item_size = *element_size - Size::new(self.indentation, 0.0);
        draw_flow(scene, ctx, &item_position, &item_size, flow);
    }

    fn paint(
        &self,
        scene: &mut SizedScene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, element_size| {
                let mut position = *position;
                for (index, flow) in self.list.iter().enumerate() {
                    self.paint_one_element(
                        scene,
                        ctx,
                        &position,
                        element_size,
                        index,
                        flow,
                    );
                    position.y += flow.height();
                }
            });
    }
}
pub struct SvgContext {
    pub fontdb: Arc<fontdb::Database>,
}

pub struct MarkdownContext<'a> {
    pub svg_ctx: &'a SvgContext,
    pub font_ctx: &'a mut FontContext,
    pub layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
    pub theme: &'a Theme,
}

#[derive(Clone)]
pub enum ListMarker {
    Symbol {
        symbol: String,
        layout: Box<Layout<MarkdownBrush>>,
    },
    Numbers {
        start_number: u32,
        layouted: Vec<Layout<MarkdownBrush>>,
    },
}

impl fmt::Debug for ListMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListMarker::Symbol { symbol, layout: _ } => {
                write!(f, "ListMarker::Symbol {{ symbol: {} }}", symbol)
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

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownBrush(Color);

impl Default for MarkdownBrush {
    fn default() -> Self {
        MarkdownBrush(Color::from_rgb8(0x00, 0x00, 0x00))
    }
}

enum ImageType {
    Svg,
    Rasterized(image::ImageFormat),
}

#[derive(Clone)]
pub struct Link {
    url: String,
    index_range: Range<usize>,
}

impl Link {
    pub fn new(url: String, index_range: Range<usize>) -> Self {
        Self { url, index_range }
    }
}

#[derive(Clone)]
pub struct InlinedImage {
    url: String,
    data: Option<Image>,
    text_index: usize,
}

impl InlinedImage {
    pub fn new(url: String, text_index: usize) -> Self {
        Self {
            url,
            text_index,
            data: None,
        }
    }
}

#[derive(Clone)]
pub struct MarkdownText {
    str: String,
    markers: Vec<TextMarker>,
    text_layout: Layout<MarkdownBrush>,
    inlined_images: Vec<InlinedImage>,
    links: Vec<Link>,
}

impl fmt::Debug for MarkdownText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkdownText {{ str: {} }}", self.str)
    }
}

impl MarkdownText {
    pub fn new(
        str: String,
        markers: Vec<TextMarker>,
        inlined_images: Vec<InlinedImage>,
        links: Vec<Link>,
    ) -> Self {
        Self {
            str,
            markers,
            text_layout: Layout::new(),
            inlined_images,
            links,
        }
    }

    fn load_images(&mut self, svg_context: &SvgContext) {
        for inlined_image in self.inlined_images.iter_mut() {
            if inlined_image.data.is_none() {
                // TODO: Do something about unwraps
                // Maybe show broken link image or something and add something
                // to some error feed???
                // TODO: Add some cache and make image loading asynchronous.

                // This conditions most likely means it is a local file link.
                let (raw_data, image_type) = if !inlined_image.url.contains("://") {
                    let path: &Path = inlined_image.url.as_ref();
                    let buf = fs::read(&inlined_image.url).unwrap();
                    let extension = path.extension().unwrap();
                    let image_type = if extension.eq_ignore_ascii_case("svg") {
                        ImageType::Svg
                    } else {
                        ImageType::Rasterized(
                            image::ImageFormat::from_extension(extension).unwrap(),
                        )
                    };
                    (buf, image_type)
                } else {
                    let mut response = ureq::get(&inlined_image.url).call().unwrap();
                    let mime_type = response.body().mime_type().unwrap();
                    let image_type = if mime_type == "image/svg+xml" {
                        ImageType::Svg
                    } else {
                        ImageType::Rasterized(
                            image::ImageFormat::from_mime_type(mime_type).unwrap(),
                        )
                    };
                    let buf = response.body_mut().read_to_vec().unwrap();
                    (buf, image_type)
                };

                let image_data: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                    match image_type {
                        ImageType::Svg => {
                            let svg_str = String::from_utf8(raw_data).unwrap();
                            let options = usvg::Options {
                                fontdb: svg_context.fontdb.clone(),
                                ..usvg::Options::default()
                            };

                            let svg_tree =
                                usvg::Tree::from_str(&svg_str, &options).unwrap();
                            let width = svg_tree.size().width().ceil() as u32;
                            let height = svg_tree.size().height().ceil() as u32;
                            let mut pixmap =
                                tiny_skia::Pixmap::new(width, height).unwrap();
                            resvg::render(
                                &svg_tree,
                                tiny_skia::Transform::identity(),
                                &mut pixmap.as_mut(),
                            );
                            image::ImageBuffer::from_raw(
                                width,
                                height,
                                pixmap.take(),
                            )
                            .unwrap()
                        }
                        ImageType::Rasterized(format) => {
                            match image::load_from_memory_with_format(
                                &raw_data, format,
                            ) {
                                Ok(image) => image.to_rgba8(),
                                Err(_) => {
                                    // Try to fallback to automatic format recognition.
                                    image::load_from_memory(&raw_data)
                                        .unwrap_or_else(
                                        |err| {
                                            panic!("ERROR: Loading image with path {} failed with error: {}", inlined_image.url, err)
                                        }).to_rgba8()
                                }
                            }
                        }
                    };

                let (width, height) = image_data.dimensions();
                inlined_image.data = Some(Image::new(
                    image_data.to_vec().into(),
                    ImageFormat::Rgba8,
                    width,
                    height,
                ));
            }
        }
    }

    fn pre_fill_builder<'a>(
        &'a self,
        font_ctx: &'a mut FontContext,
        layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
    ) -> RangedBuilder<'a, MarkdownBrush> {
        // TODO: This is a bit fishy place to load images
        let mut builder =
            str_to_builder(&self.str, &self.markers, font_ctx, layout_ctx);
        for (image_index, inlined_image) in self.inlined_images.iter().enumerate() {
            if let Some(data) = &inlined_image.data {
                builder.push_inline_box(InlineBox {
                    id: image_index as u64,
                    index: inlined_image.text_index,
                    width: data.width as f32,
                    height: data.height as f32,
                });
            }
        }
        builder
    }

    fn load_and_layout_as_header(
        &mut self,
        ctx: &mut MarkdownContext,
        width: f64,
        level: HeadingLevel,
    ) {
        self.load_images(ctx.svg_ctx);
        let mut builder = self.pre_fill_builder(ctx.font_ctx, ctx.layout_ctx);
        let font_size = match level {
            HeadingLevel::H1 => ctx.theme.text.text_size as f32 * 2.125,
            HeadingLevel::H2 => ctx.theme.text.text_size as f32 * 1.875,
            HeadingLevel::H3 => ctx.theme.text.text_size as f32 * 1.5,
            HeadingLevel::H4 => ctx.theme.text.text_size as f32 * 1.25,
            HeadingLevel::H5 => ctx.theme.text.text_size as f32 * 1.125,
            HeadingLevel::H6 => ctx.theme.text.text_size as f32,
        };
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::LineHeight(
            ctx.theme.markdown.header_line_height,
        ));
        builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        let mut layout = builder.build(&self.str);
        layout.break_all_lines(Some(width as f32));
        self.text_layout = layout;
    }

    // Loads inlined images and layouts the text with prepared box reserved for
    // them.
    fn load_and_layout_text(&mut self, ctx: &mut MarkdownContext, width: f64) {
        self.load_images(ctx.svg_ctx);
        let mut builder = self.pre_fill_builder(ctx.font_ctx, ctx.layout_ctx);
        let mut layout = builder.build(&self.str);
        layout.break_all_lines(Some(width as f32));
        self.text_layout = layout;
    }

    fn layout_as_code(&mut self, ctx: &mut MarkdownContext, width: f64) {
        let mut builder = self.pre_fill_builder(ctx.font_ctx, ctx.layout_ctx);
        builder.push_default(StyleProperty::FontStack(
            ctx.theme.text.monospace_font_stack.clone(),
        ));
        builder.push_default(StyleProperty::Brush(MarkdownBrush(
            ctx.theme.text.monospace_text_color,
        )));
        let mut layout = builder.build(&self.str);
        layout.break_all_lines(Some(width as f32));
        self.text_layout = layout;
    }

    fn draw_text(&self, scene: &mut SizedScene, position: &Vec2) {
        draw_text(&self.text_layout, scene, position, &self.inlined_images);
    }

    fn height(&self) -> Height {
        self.text_layout.height() as f64
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
        self.margin.insets.y0 = ctx.theme.markdown.paragraph_top_margin;
        if reduce_top_margin {
            self.margin.insets.y0 = 0.0;
        }

        self.margin.layout_by_width(width, |width| {
            self.text.load_and_layout_text(ctx, width);
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut SizedScene,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text.draw_text(scene, position);
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
        self.margin.insets = Insets::uniform(ctx.theme.markdown.code_block_margin);

        self.margin.layout_by_width(width, |width| {
            self.text.layout_as_code(ctx, width);
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut SizedScene,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text.draw_text(scene, position);
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
    ) -> (Margin, Layout<MarkdownBrush>) {
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

        let symbol = match self {
            IndentationDecoration::Indentation => "".to_string(),
            IndentationDecoration::Note => theme.indentation_note_sign.clone(),
            IndentationDecoration::Important => {
                theme.indentation_important_sign.clone()
            }
            IndentationDecoration::Tip => theme.indentation_tip_sign.clone(),
            IndentationDecoration::Warning => theme.indentation_warning_sign.clone(),
            IndentationDecoration::Caution => theme.indentation_caution_sign.clone(),
        };

        let color = self.color(theme);

        let layout = if symbol.is_empty() {
            Layout::new()
        } else {
            let mut builder =
                str_to_builder(&symbol, &[], ctx.font_ctx, ctx.layout_ctx);
            builder.push_default(StyleProperty::FontStack(FontStack::Single(
                FontFamily::Named("FiraCode Nerd Font".into()),
            )));
            builder.push_default(StyleProperty::Brush(MarkdownBrush(color)));
            let mut layout = builder.build(&symbol);
            layout.break_all_lines(None);
            layout
        };

        let additional_left_padding = if symbol.is_empty() {
            0.0
        } else {
            // TODO: This should be themeable???
            layout.full_width() as f64
                + (theme.indentation_sign_horizontal_padding * 2.0)
        };

        (
            Margin::new(left + additional_left_padding, top, right, bottom),
            layout,
        )
    }

    fn color(&self, theme: &MarkdowTheme) -> Color {
        match self {
            IndentationDecoration::Indentation => theme.indentation_color,
            IndentationDecoration::Note => theme.indentation_note_color,
            IndentationDecoration::Important => theme.indentation_important_color,
            IndentationDecoration::Tip => theme.indentation_tip_color,
            IndentationDecoration::Warning => theme.indentation_warning_color,
            IndentationDecoration::Caution => theme.indentation_caution_color,
        }
    }

    fn paint(
        &self,
        scene: &mut SizedScene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
        symbol_layout: &Layout<MarkdownBrush>,
        padding: &Margin,
    ) {
        let theme = &ctx.theme.markdown;
        let color = self.color(&ctx.theme.markdown);
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
                    - symbol_layout.full_width() as f64
                    - theme.indentation_box_line_width)
                    / 2.0;
                let y = padding.insets.y0; //theme.indentation_sign_top_padding;

                draw_text(symbol_layout, scene, &(*position + Vec2::new(x, y)), &[]);
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
    symbol_layout: Layout<MarkdownBrush>,
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
            symbol_layout: Layout::new(),
        }
    }

    fn layout(&mut self, ctx: &mut MarkdownContext, width: Width) -> Height {
        self.margin.insets.x0 = ctx.theme.markdown.indentation_horizonatl_margin;
        self.margin.insets.x1 = ctx.theme.markdown.indentation_horizonatl_margin;
        self.margin.insets.y0 = ctx.theme.markdown.indentation_vertical_margin;
        self.margin.insets.y1 = ctx.theme.markdown.indentation_vertical_margin;

        let (padding, layout) = self.decoration.padding_and_symbol(ctx);
        self.padding = padding;
        self.symbol_layout = layout;

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
        scene: &mut SizedScene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, element_size| {
                self.decoration.paint(
                    scene,
                    ctx,
                    position,
                    element_size,
                    &self.symbol_layout,
                    &self.padding,
                );
                self.padding.paint(
                    position,
                    element_size,
                    |position, element_size| {
                        draw_flow(scene, ctx, position, element_size, &self.flow);
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

        self.margin.layout_by_width(width, |width| {
            self.text.load_and_layout_as_header(ctx, width, self.level);
            self.text.height()
        })
    }

    fn height(&self) -> Height {
        self.margin.height(|| self.text.height())
    }

    fn paint(
        &self,
        scene: &mut SizedScene,
        _ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        self.margin
            .paint(position, element_size, |position, _element_size| {
                self.text.draw_text(scene, position);
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
        scene: &mut SizedScene,
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
        scene: &mut SizedScene,
        ctx: &mut MarkdownContext,
        position: &Vec2,
        element_size: &Size,
    ) {
        // TODO: Draw indentation decoration
        match self {
            MarkdownContent::Paragraph(paragraph) => {
                paragraph.paint(scene, ctx, position, element_size);
            }
            // TODO: Add support for solo image
            MarkdownContent::CodeBlock(code_block) => {
                code_block.paint(scene, ctx, position, element_size);
            }
            MarkdownContent::Indented(indented) => {
                indented.paint(scene, ctx, position, element_size);
            }
            MarkdownContent::List(list) => {
                list.paint(scene, ctx, position, element_size);
            }
            MarkdownContent::HorizontalLine(horizontal_line) => {
                horizontal_line.paint(scene, ctx, position, element_size);
            }
            MarkdownContent::Header(header) => {
                header.paint(scene, ctx, position, element_size);
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

fn draw_text(
    layout: &Layout<MarkdownBrush>,
    scene: &mut SizedScene,
    position: &Vec2,
    inlined_images: &[InlinedImage],
) {
    let transform: Affine = Affine::translate(*position);

    // The start_y is in layout coordinates.
    let start_y = if position.y < 0.0 {
        -position.y as f32
    } else {
        0.0
    };
    // The stop_y is in layout coordinates.
    let stop_y = layout.height() + start_y;

    let mut top_line_index =
        if let Some((cluster, _)) = Cluster::from_point(layout, 0.0, start_y) {
            cluster.path().line_index()
        } else {
            0
        };

    while let Some(line) = layout.get(top_line_index) {
        let line_metrics = line.metrics();
        if line_metrics.min_coord > stop_y {
            break;
        }
        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let style = glyph_run.style();
                    let text_color = &style.brush;

                    let run = glyph_run.run();
                    // TODO: This needs to be some kind of a flow layout.
                    let font = run.font();
                    let font_size = run.font_size();
                    let synthesis = run.synthesis();
                    let glyph_xform = synthesis.skew().map(|angle| {
                        Affine::skew(angle.to_radians().tan() as f64, 0.0)
                    });
                    let coords = run.normalized_coords();
                    scene
                        .draw_glyphs(font)
                        .brush(text_color.0)
                        .hint(true)
                        .transform(transform)
                        .glyph_transform(glyph_xform)
                        .font_size(font_size)
                        .normalized_coords(coords)
                        .draw(
                            Fill::NonZero,
                            glyph_run.positioned_glyphs().map(|glyph| {
                                vello::Glyph {
                                    id: glyph.id as _,
                                    x: glyph.x,
                                    y: glyph.y,
                                }
                            }),
                        );

                    let run_metrics = run.metrics();
                    if let Some(underline) = &style.underline {
                        draw_underline(
                            scene,
                            underline,
                            &glyph_run,
                            run_metrics,
                            &transform,
                        );
                    }

                    if let Some(strikethrough) = &style.strikethrough {
                        draw_strikethrough(
                            scene,
                            strikethrough,
                            &glyph_run,
                            run_metrics,
                            &transform,
                        );
                    }
                }
                PositionedLayoutItem::InlineBox(positioned_inline_box) => {
                    // TODO: What to do when this thing fails???
                    let image = &inlined_images[positioned_inline_box.id as usize];
                    let image_translation = *position
                        + Vec2::new(
                            positioned_inline_box.x as f64,
                            positioned_inline_box.y as f64,
                        );
                    // TODO: The unwrap is not nice...
                    draw_image(
                        scene,
                        image.data.as_ref().unwrap(),
                        image_translation,
                    );
                }
            }
        }
        top_line_index += 1;
    }
}

impl LayoutData for MarkdownContent {
    fn height(&self) -> Height {
        self.height()
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
    fn feed_to_builder<'a>(
        &self,
        builder: &'a mut RangedBuilder<MarkdownBrush>,
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
                    StyleProperty::Brush(MarkdownBrush(
                        theme.text.monospace_text_color,
                    )),
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

pub struct MarkerState {
    pub bold_start: usize,
    pub italic_start: usize,
    pub strikethrough_start: usize,
    pub markers: Vec<TextMarker>,
}

impl MarkerState {
    pub fn new() -> Self {
        Self {
            bold_start: 0,
            italic_start: 0,
            strikethrough_start: 0,
            markers: Vec::new(),
        }
    }
}

impl Default for MarkerState {
    fn default() -> Self {
        Self::new()
    }
}

fn draw_image(scene: &mut SizedScene, image: &Image, translation: Vec2) {
    let transform: Affine = Affine::translate(translation);
    scene.draw_image(image, transform);
}

// TODO: Shoul this be a part of some markdown object??
pub fn draw_flow(
    scene: &mut SizedScene,
    ctx: &mut MarkdownContext,
    position: &Vec2,
    element_size: &Size,
    flow: &LayoutFlow<MarkdownContent>,
) {
    let offset = if position.y < 0.0 { -position.y } else { 0.0 };
    let height = if position.y > 0.0 {
        scene.size.height - position.y
    } else {
        scene.size.height
    };
    let visible_parts = flow.get_visible_parts(offset, height);

    for visible_part in visible_parts.iter() {
        let part_position = *position + Vec2::new(0.0, visible_part.offset);
        let part_size = Size::new(element_size.width, visible_part.height);
        visible_part
            .data
            .paint(scene, ctx, &part_position, &part_size);
    }
}

fn draw_underline(
    scene: &mut SizedScene,
    underline: &Decoration<MarkdownBrush>,
    glyph_run: &GlyphRun<'_, MarkdownBrush>,
    run_metrics: &RunMetrics,
    transform: &Affine,
) {
    let offset = underline.offset.unwrap_or(run_metrics.underline_offset);
    let stroke_size = underline.size.unwrap_or(run_metrics.underline_size);
    let y1 = glyph_run.baseline() - offset - (stroke_size / 2.0);
    let x1 = glyph_run.offset();
    let x2 = x1 + glyph_run.advance();
    let underline_shape = Line::new((x1, y1), (x2, y1));

    let stroke = Stroke {
        width: stroke_size as f64,
        join: Join::Bevel,
        miter_limit: 4.0,
        start_cap: Cap::Butt,
        end_cap: Cap::Butt,
        dash_pattern: Default::default(),
        dash_offset: 0.0,
    };

    scene.stroke(
        &stroke,
        *transform,
        underline.brush.0,
        Some(Affine::IDENTITY),
        &underline_shape,
    );
}

fn draw_strikethrough(
    scene: &mut SizedScene,
    strikethrough: &Decoration<MarkdownBrush>,
    glyph_run: &GlyphRun<'_, MarkdownBrush>,
    run_metrics: &RunMetrics,
    transform: &Affine,
) {
    let offset = strikethrough
        .offset
        .unwrap_or(run_metrics.strikethrough_offset);
    let size = strikethrough.size.unwrap_or(run_metrics.strikethrough_size);
    // FIXME: This offset looks fishy... I think I should add it instead.
    let y1 = glyph_run.baseline() - offset - (size / 2.0);
    let x1 = glyph_run.offset();
    let x2 = x1 + glyph_run.advance();
    let strikethrough_shape = Line::new((x1, y1), (x2, y1));

    let stroke = Stroke {
        width: size as f64,
        join: Join::Bevel,
        miter_limit: 4.0,
        start_cap: Cap::Butt,
        end_cap: Cap::Butt,
        dash_pattern: Default::default(),
        dash_offset: 0.0,
    };

    scene.stroke(
        &stroke,
        *transform,
        strikethrough.brush.0,
        Some(Affine::IDENTITY),
        &strikethrough_shape,
    );
}

// TODO: I don't like this function. I'll need to think of something better.
fn str_to_builder<'a>(
    text: &'a str,
    markers: &[TextMarker],
    font_ctx: &'a mut FontContext,
    layout_ctx: &'a mut LayoutContext<MarkdownBrush>,
) -> RangedBuilder<'a, MarkdownBrush> {
    // TODO: Pass theme from ctx...
    let theme = get_theme();

    let mut builder: RangedBuilder<'_, MarkdownBrush> =
        layout_ctx.ranged_builder(font_ctx, text, theme.scale);
    builder.push_default(StyleProperty::Brush(MarkdownBrush(theme.text.text_color)));
    builder.push_default(StyleProperty::FontSize(theme.text.text_size as f32));
    builder.push_default(StyleProperty::FontStack(theme.text.font_stack.clone()));
    builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
    builder.push_default(StyleProperty::FontStyle(FontStyle::Normal));
    builder.push_default(StyleProperty::LineHeight(1.0));
    for marker in markers.iter() {
        marker.feed_to_builder(&mut builder, &theme);
    }
    builder
}
