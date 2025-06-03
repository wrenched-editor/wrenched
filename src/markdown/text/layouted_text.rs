use std::{fmt, ops::Range};

use kurbo::{Affine, BezPath, Cap, Join, Line, Point, Rect, Size, Stroke, Vec2};
use masonry::core::BrushIndex;
use parley::{
    Affinity, Alignment, Cluster, Cursor, Decoration, GlyphRun, Layout, LineMetrics, PositionedLayoutItem, RangedBuilder, RunMetrics
};
use peniko::{BlendMode, Fill, Image};
use vello::{peniko::Color, Scene};

use crate::markdown::context::LayoutContext;

#[derive(Clone, Debug)]
pub struct Brush {
    color: Color,
    underline_color: Color,
    curly_underline: bool,
}

impl Brush {
    pub fn new(
        color: Color,
        underline_color: Color,
        curly_underline: bool,
    ) -> Brush {
        Brush {
            color,
            underline_color,
            curly_underline,
        }
    }
    pub fn just_text(color: Color) -> Brush {
        Brush {
            color,
            underline_color: color,
            curly_underline: false,
        }
    }
}

#[derive(Clone, Debug,PartialEq, Eq)]
pub struct Selection {
    // Range of start and end bytes in text. (NOT the char index).
    indices: Range<usize>,
}

impl Selection {
    pub fn new(indices: Range<usize>) -> Selection {
        Selection {
            indices,
        }
    }

    pub fn empty() -> Selection {
        Selection {
            indices: 0..0,
        }
    }
}

#[derive(Clone)]
pub struct LayoutedText {
    text: String,
    layout: Layout<BrushIndex>,
    selection: Option<Selection>,
    cursor: Option<Cursor>,
}

impl fmt::Debug for LayoutedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkdownText {{ str: {} }}", self.text)
    }
}

impl LayoutedText {
    pub fn new(str: String) -> Self {
        Self {
            text: str,
            layout: Layout::new(),
            selection: None,
            cursor: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            text: String::new(),
            layout: Layout::new(),
            selection: None,
            cursor: None,
        }
    }

    pub fn cursor_position(&self, point: &Point) -> Cursor {
        Cursor::from_point(&self.layout, point.x as f32, point.y as f32)
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = Some(selection);
    }

    pub fn remove_selection(&mut self) {
        self.selection = None;
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.into();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn height(&self) -> f64 {
        self.layout.height() as f64
    }

    pub fn full_width(&self) -> f64 {
        self.layout.full_width() as f64
    }

    pub fn build_layout<F>(
        &mut self,
        text_ctx: &mut LayoutContext,
        scale: f32,
        max_advance: Option<f64>,
        style: F,
    ) where
        F: FnOnce(&mut RangedBuilder<'_, BrushIndex>),
    {
        // TODO: This is a bit fishy place to load images
        let mut builder: RangedBuilder<'_, BrushIndex> = text_ctx
            .layout_ctx
            .ranged_builder(text_ctx.font_ctx, &self.text, scale);
        style(&mut builder);
        self.layout = builder.build(&self.text);
        self.layout.break_all_lines(max_advance.map(|v| v as f32));
    }
    pub fn align(
        &mut self,
        container_width: Option<f32>,
        alignment: Alignment,
        align_when_overflowing: bool,
    ) {
        self.layout
            .align(container_width, alignment, align_when_overflowing);
    }

    pub fn draw_text<'a, F>(
        &self,
        scene: &mut Scene,
        scene_size: &Size,
        position: &Vec2,
        get_image: F,
        brushes: &[Brush],
    ) where
        F: Fn(u64) -> Option<&'a Image>,
    {
        draw_text(
            &self.layout,
            scene,
            scene_size,
            position,
            &self.selection,
            self.cursor,
            get_image,
            brushes,
        );
    }
}

pub fn draw_text<'a, F>(
    layout: &Layout<BrushIndex>,
    scene: &mut Scene,
    scene_size: &Size,
    position: &Vec2,
    selection: &Option<Selection>,
    cursor: Option<Cursor>,
    get_image: F,
    brushes: &[Brush],
) where
    F: Fn(u64) -> Option<&'a Image>,
{
    let transform: Affine = Affine::translate(*position);

    if let Some(selection) = selection {
    }

    if let Some(cursor) = cursor {
        let cursor_rect = cursor.geometry(layout, 1.5);
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &cursor_rect);
    }

    // The start_y is in layout coordinates.
    let start_y = if position.y < 0.0 {
        -position.y as f32
    } else {
        0.0
    };
    // The stop_y is in layout coordinates.
    let stop_y = scene_size.height as f32 + start_y;

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
                    let brush: Color = brushes[style.brush.0].color;

                    let run = glyph_run.run();
                    // TODO: This needs to be some kind of a flow layout.
                    let font = run.font();
                    let font_size = run.font_size();
                    let synthesis = run.synthesis();
                    let glyph_xform = synthesis.skew().map(|angle| {
                        Affine::skew(angle.to_radians().tan() as f64, 0.0)
                    });
                    let run_metrics = run.metrics();
                    if let Some(underline) = &style.underline {
                        draw_underline(
                            scene,
                            underline,
                            &glyph_run,
                            run_metrics,
                            line_metrics,
                            &transform,
                            &brushes[underline.brush.0],
                        );
                    }

                    let coords = run.normalized_coords();
                    scene
                        .draw_glyphs(font)
                        .brush(brush)
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

                    if let Some(strikethrough) = &style.strikethrough {
                        draw_strikethrough(
                            scene,
                            strikethrough,
                            &glyph_run,
                            run_metrics,
                            &transform,
                            brushes,
                        );
                    }
                }
                PositionedLayoutItem::InlineBox(positioned_inline_box) => {
                    // TODO: What to do when this thing fails???
                    let image = get_image(positioned_inline_box.id);
                    if let Some(image) = image {
                        let image_translation = *position
                            + Vec2::new(
                                positioned_inline_box.x as f64,
                                positioned_inline_box.y as f64,
                            );
                        // TODO: The unwrap is not nice...
                        let transform: Affine = Affine::translate(image_translation);
                        scene.draw_image(image, transform);
                    }
                }
            }
        }
        top_line_index += 1;
    }
}

fn draw_underline(
    scene: &mut Scene,
    underline: &Decoration<BrushIndex>,
    glyph_run: &GlyphRun<'_, BrushIndex>,
    run_metrics: &RunMetrics,
    line_metrics: &LineMetrics,
    transform: &Affine,
    brush: &Brush,
) {
    if brush.curly_underline {
        draw_curly_underline(
            scene,
            underline,
            glyph_run,
            run_metrics,
            line_metrics,
            transform,
            brush,
        );
    } else {
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

        let brush: Color = brush.underline_color;
        scene.stroke(
            &stroke,
            *transform,
            brush,
            Some(Affine::IDENTITY),
            &underline_shape,
        );
    }
}

fn draw_curly_underline(
    scene: &mut Scene,
    underline: &Decoration<BrushIndex>,
    glyph_run: &GlyphRun<'_, BrushIndex>,
    run_metrics: &RunMetrics,
    line_metrics: &LineMetrics,
    transform: &Affine,
    brush: &Brush,
) {
    let offset = underline.offset.unwrap_or(run_metrics.underline_offset) as f64;
    let stroke_size = underline.size.unwrap_or(run_metrics.underline_size) as f64;
    let y_top = glyph_run.baseline() as f64 - offset;
    let y_bottom = glyph_run.baseline() as f64 + line_metrics.descent as f64;
    let left = glyph_run.offset() as f64;
    let right = (glyph_run.offset() + glyph_run.advance()) as f64;

    let stroke = Stroke {
        width: stroke_size,
        join: Join::Bevel,
        miter_limit: 4.0,
        start_cap: Cap::Round,
        end_cap: Cap::Round,
        dash_pattern: Default::default(),
        dash_offset: 0.0,
    };

    scene.push_layer(
        BlendMode::default(),
        1.,
        *transform,
        &Rect::new(left, y_top, right, y_bottom),
    );
    let curly_path = curly_path(left, right, y_top, y_bottom, 0.0);

    scene.stroke(
        &stroke,
        *transform,
        brush.underline_color,
        Some(Affine::IDENTITY),
        &curly_path,
    );

    scene.pop_layer();
}

/// This function is doing overdraw on y axis it is up to the user to make
/// sure everything fits together.
fn curly_path(
    left: f64,
    right: f64,
    top: f64,
    botton: f64,
    stroke_size: f64,
) -> BezPath {
    let height_middle = (top + botton) / 2.0;
    // Little bit of black magic borrowed from:
    // https://math.stackexchange.com/questions/4235124/getting-the-most-accurate-bezier-curve-that-plots-a-sine-wave
    // The sine wave is oscillating around the x axis so the amplitude is
    // 2, thus we need to divide the scale by 2.
    let height = botton - top;
    let width = right - left;
    let sin_scale: f64 = (height - stroke_size) / 2.0;
    let sin_len: f64 = 2.0 * std::f64::consts::PI * sin_scale;
    let scaled_v: f64 = 3.4641 * sin_scale;
    let scaled_u: f64 = 2.9361 * sin_scale;

    let mut path = BezPath::new();
    path.move_to((left, height_middle));
    // Add individual sines
    for i in 0..((width / sin_len).ceil() as u32) {
        let i = i as f64;
        let x = left + (sin_len * i);
        let x_next = left + (sin_len * (i + 1.0));
        let p2: Point = (x + scaled_u, height_middle + scaled_v).into();
        let p3: Point = (x_next - scaled_u, height_middle - scaled_v).into();
        let p4: Point = (x_next, height_middle).into();
        path.curve_to(p2, p3, p4);
    }
    path
}

fn draw_strikethrough(
    scene: &mut Scene,
    strikethrough: &Decoration<BrushIndex>,
    glyph_run: &GlyphRun<'_, BrushIndex>,
    run_metrics: &RunMetrics,
    transform: &Affine,
    brushes: &[Brush],
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

    let brush: Color = brushes[strikethrough.brush.0].underline_color;

    scene.stroke(
        &stroke,
        *transform,
        brush,
        Some(Affine::IDENTITY),
        &strikethrough_shape,
    );
}

impl From<String> for LayoutedText {
    fn from(value: String) -> Self {
        LayoutedText::new(value)
    }
}
