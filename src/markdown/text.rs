use std::{fmt, fs, ops::Range, path::Path, sync::Arc};

use kurbo::{Affine, Cap, Join, Line, Stroke, Vec2};
use parley::{Cluster, Decoration, FontStyle, GlyphRun, InlineBox, Layout, PositionedLayoutItem, RangedBuilder, RunMetrics, StyleProperty};
use peniko::{Fill, Image, ImageFormat};
use vello::Scene;
use xilem::FontWeight;

use crate::{basic_types::Height, theme::Theme};

use super::{context::{SvgContext, TextContext}, elements::MarkdownBrush};


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

#[derive(Clone)]
pub struct MarkdownText {
    text: String,
    markers: Vec<TextMarker>,
    text_layout: Layout<MarkdownBrush>,
    inlined_images: Vec<InlinedImage>,
    links: Vec<Link>,
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

impl fmt::Debug for MarkdownText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkdownText {{ str: {} }}", self.text)
    }
}

enum ImageType {
    Svg,
    Rasterized(image::ImageFormat),
}

impl MarkdownText {
    pub fn new(
        str: String,
        markers: Vec<TextMarker>,
        inlined_images: Vec<InlinedImage>,
        links: Vec<Link>,
    ) -> Self {
        Self {
            text: str,
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
                                fontdb: Arc::new(svg_context.fontdb.clone()),
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

    fn get_default_styles(theme: &Theme) -> Vec<StyleProperty<'static, MarkdownBrush>>{
        vec![
            StyleProperty::Brush(MarkdownBrush(theme.text.text_color)),
            StyleProperty::FontSize(theme.text.text_size as f32),
            StyleProperty::<'static, MarkdownBrush>::FontStack(theme.text.font_stack.clone()),
            StyleProperty::FontWeight(FontWeight::NORMAL),
            StyleProperty::FontStyle(FontStyle::Normal),
            StyleProperty::LineHeight(1.0),
        ]
    }

    fn fill_default_styles(theme: &Theme, builder: &mut RangedBuilder<'_, MarkdownBrush>) {
        builder.push_default(StyleProperty::Brush(MarkdownBrush(theme.text.text_color)));
        builder.push_default(StyleProperty::FontSize(theme.text.text_size as f32));
        builder.push_default(theme.text.font_stack.clone());
        builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
        builder.push_default(StyleProperty::FontStyle(FontStyle::Normal));
        builder.push_default(StyleProperty::LineHeight(1.0));
    }

    fn build_layout(
        &mut self,
        text_ctx: &mut TextContext,
        extra_default_styles: &[StyleProperty<MarkdownBrush>],
        extra_styles: &[(StyleProperty<MarkdownBrush>, Range<usize>)],
    ) {
        // TODO: This is a bit fishy place to load images
        let mut builder: RangedBuilder<'_, MarkdownBrush> =
            text_ctx.layout_ctx.ranged_builder(text_ctx.font_ctx, &self.text, text_ctx.theme.scale);
        Self::fill_default_styles(&text_ctx.theme, &mut builder);
        for extra_default_style in extra_default_styles {
            builder.push_default(extra_default_style.clone());
        }
        for marker in self.markers.iter() {
            marker.feed_to_builder(&mut builder, &text_ctx.theme);
        }
        for (extra_style, range) in extra_styles {
            builder.push(extra_style.clone(), range.clone());
        }
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
        self.text_layout = builder.build(&self.text);
    }

    // Loads inlined images and layouts the text with prepared box reserved for
    // them.
    fn load_and_layout_text(
        &mut self,
        text_ctx: &mut TextContext,
        extra_default_styles: &[StyleProperty<MarkdownBrush>],
        extra_styles: &[(StyleProperty<MarkdownBrush>, Range<usize>)],
        width: f64,
     ) {
        self.load_images(text_ctx.svg_ctx);
        self.build_layout(text_ctx, extra_default_styles, extra_styles);
        self.text_layout.break_all_lines(Some(width as f32));
    }

//    fn load_and_layout_as_header(
//        &mut self,
//        ctx: &mut MarkdownContext,
//        width: f64,
//        level: HeadingLevel,
//    ) {
//        self.load_images(ctx.svg_ctx);
//        let mut builder = self.pre_fill_builder(ctx.font_ctx, ctx.layout_ctx);
//        let font_size = match level {
//            HeadingLevel::H1 => ctx.theme.text.text_size as f32 * 2.125,
//            HeadingLevel::H2 => ctx.theme.text.text_size as f32 * 1.875,
//            HeadingLevel::H3 => ctx.theme.text.text_size as f32 * 1.5,
//            HeadingLevel::H4 => ctx.theme.text.text_size as f32 * 1.25,
//            HeadingLevel::H5 => ctx.theme.text.text_size as f32 * 1.125,
//            HeadingLevel::H6 => ctx.theme.text.text_size as f32,
//        };
//        builder.push_default(StyleProperty::FontSize(font_size));
//        builder.push_default(StyleProperty::LineHeight(
//            ctx.theme.markdown.header_line_height,
//        ));
//        builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
//        self.text_layout = layout;
//    }
//
//
//    fn layout_as_code(&mut self, ctx: &mut MarkdownContext, width: f64) {
//        let mut builder = self.pre_fill_builder(ctx.font_ctx, ctx.layout_ctx);
//        builder.push_default(StyleProperty::FontStack(
//            ctx.theme.text.monospace_font_stack.clone(),
//        ));
//        builder.push_default(StyleProperty::Brush(MarkdownBrush(
//            ctx.theme.text.monospace_text_color,
//        )));
//        let mut layout = builder.build(&self.text);
//        layout.break_all_lines(Some(width as f32));
//        self.text_layout = layout;
//    }

    fn draw_text(&self, scene: &mut Scene, position: &Vec2) {
        draw_text(&self.text_layout, scene, position, &self.inlined_images);
    }

    fn height(&self) -> Height {
        self.text_layout.height() as f64
    }
}

fn draw_text(
    layout: &Layout<MarkdownBrush>,
    scene: &mut Scene,
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

fn draw_underline(
    scene: &mut Scene,
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
    scene: &mut Scene,
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

fn draw_image(scene: &mut Scene, image: &Image, translation: Vec2) {
    let transform: Affine = Affine::translate(translation);
    scene.draw_image(image, transform);
}
