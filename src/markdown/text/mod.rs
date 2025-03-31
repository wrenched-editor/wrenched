pub mod layouted_text;
pub mod simple;
pub mod styles;

use std::{cmp::Ordering, f64, fmt, fs, ops::Range, path::Path};

use kurbo::{Size, Vec2};
use layouted_text::LayoutedText;
use masonry::core::BrushIndex;
use parley::{InlineBox, StyleProperty};
use peniko::{Image, ImageFormat};
use styles::{BrushPalete, TextMarker};
use vello::Scene;

use super::context::{SvgContext, TextContext};
use crate::basic_types::Height;

#[derive(Clone)]
pub struct MarkdownText {
    text: LayoutedText,
    markers: Vec<TextMarker>,
    inlined_images: Vec<InlinedImage>,
    links: Vec<Link>,
    hovered_link: Option<usize>,
}

#[derive(Clone)]
pub struct Link {
    pub url: String,
    pub index_range: Range<usize>,
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
        write!(f, "MarkdownText {{ text: {:?} }}", self.text)
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
            text: LayoutedText::new(str),
            markers,
            inlined_images,
            links,
            hovered_link: None,
        }
    }

    pub fn on_mouse_move(
        &mut self,
        text_ctx: &mut TextContext,
        extra_default_styles: &[StyleProperty<BrushIndex>],
        extra_styles: &[(StyleProperty<BrushIndex>, Range<usize>)],
        width: f64,
        point: &Vec2,
    ) {
        let cursor = self.text.cursor_position(point);
        let index = cursor.index();
        let hovered_link = self
            .links
            .binary_search_by(|v| {
                // TODO: This comparison should probably use epsilon
                if v.index_range.start <= index && v.index_range.end >= index {
                    Ordering::Equal
                } else if v.index_range.start < index {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .ok();

        if self.hovered_link != hovered_link {
            self.build_layout(text_ctx, extra_default_styles, extra_styles, width);
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

    fn build_layout(
        &mut self,
        text_ctx: &mut TextContext,
        extra_default_styles: &[StyleProperty<BrushIndex>],
        extra_styles: &[(StyleProperty<BrushIndex>, Range<usize>)],
        width: f64,
    ) {
        self.text.build_layout(
            text_ctx.layout_ctx,
            text_ctx.theme.scale,
            Some(width),
            |builder| {
                BrushPalete::fill_default_styles(text_ctx.theme, builder);
                for extra_default_style in extra_default_styles {
                    builder.push_default(extra_default_style.clone());
                }
                for marker in self.markers.iter() {
                    marker.feed_to_builder(builder, text_ctx.theme);
                }
                for (extra_style, range) in extra_styles {
                    builder.push(extra_style.clone(), range.clone());
                }
                for (image_index, inlined_image) in
                    self.inlined_images.iter().enumerate()
                {
                    if let Some(data) = &inlined_image.data {
                        builder.push_inline_box(InlineBox {
                            id: image_index as u64,
                            index: inlined_image.text_index,
                            width: data.width as f32,
                            height: data.height as f32,
                        });
                    }
                }
            },
        );
    }

    // Loads inlined images and layouts the text with prepared box reserved for
    // them.
    pub fn load_and_layout_text(
        &mut self,
        text_ctx: &mut TextContext,
        extra_default_styles: &[StyleProperty<BrushIndex>],
        extra_styles: &[(StyleProperty<BrushIndex>, Range<usize>)],
        width: f64,
    ) {
        self.load_images(text_ctx.svg_ctx);
        self.build_layout(text_ctx, extra_default_styles, extra_styles, width);
    }

    pub fn draw_text(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        position: &Vec2,
        brush_palate: &BrushPalete,
    ) {
        self.text.draw_text(
            scene,
            scene_size,
            position,
            |index| {
                let i = self.inlined_images.get(index as usize)?;
                i.data.as_ref()
            },
            &brush_palate.palete,
        );
    }

    pub fn height(&self) -> Height {
        self.text.height()
    }
}
