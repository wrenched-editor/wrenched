use core::{f32, f64};

use kurbo::{Affine, BezPath, Cap, Join, Line, Rect, Size, Stroke, Vec2};
use parley::{
    fontique::{Collection, CollectionOptions},
    layout::Cursor,
    style::{FontFamily, GenericFamily, StyleProperty},
    Cluster, Decoration, FontContext, FontStack, FontStyle, GlyphRun, Layout,
    LayoutContext, LineMetrics, PositionedLayoutItem, RangedBuilder, RunMetrics,
};
use peniko::BlendMode;
use vello::{
    kurbo::Point,
    peniko::{self, Color, Fill, Gradient},
    Scene,
};
use xilem::FontWeight;

use crate::theme::get_theme;

pub struct CodeTextLayout {
    font: FontStack<'static>,
    max_advance: Option<f32>,
    layout: Layout<CodeTextBrush>,
    text_hinting: bool,
    text_layout_ctx: LayoutContext<CodeTextBrush>,
    font_ctx: FontContext,
    scroll: f64,
}

/// A custom brush for `Parley`, enabling using Parley to pass-through
/// which glyphs are selected/highlighted
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CodeTextBrush {
    pub text: peniko::Brush,
    pub backgroud: Option<peniko::Brush>,
    pub curly_underline: bool,
}

impl From<peniko::Brush> for CodeTextBrush {
    fn from(value: peniko::Brush) -> Self {
        Self {
            text: value,
            backgroud: None,
            curly_underline: false,
        }
    }
}

impl From<Gradient> for CodeTextBrush {
    fn from(value: Gradient) -> Self {
        Self {
            text: value.into(),
            backgroud: None,
            curly_underline: false,
        }
    }
}

impl From<Color> for CodeTextBrush {
    fn from(value: Color) -> Self {
        Self {
            text: value.into(),
            backgroud: None,
            curly_underline: false,
        }
    }
}

impl CodeTextLayout {
    /// Create a new `TextLayout` object.
    pub fn new() -> Self {
        CodeTextLayout {
            font: FontStack::Single(FontFamily::Generic(GenericFamily::SansSerif)),

            max_advance: None,

            layout: Layout::new(),
            text_hinting: true,
            text_layout_ctx: LayoutContext::new(),
            font_ctx: FontContext {
                collection: Collection::new(CollectionOptions {
                    system_fonts: true,
                    ..Default::default()
                }),
                source_cache: Default::default(),
            },
            scroll: 0.0,
        }
    }

    /// Set the width at which to wrap words.
    ///
    /// You may pass `None` to disable word wrapping
    /// (the default behaviour).
    pub fn set_max_advance(&mut self, max_advance: Option<f32>) {
        let max_advance = max_advance.map(|it| it.max(0.0));
        if self.max_advance.is_some() != max_advance.is_some()
            || self
                .max_advance
                .zip(max_advance)
                // 1e-4 is an arbitrary small-enough value that we don't care to rewrap
                .map(|(old, new)| (old - new).abs() >= 1e-4)
                .unwrap_or(false)
        {
            self.max_advance = max_advance;
        }
    }

    /// Returns the inner Parley [`Layout`] value.
    pub fn layout(&self) -> &Layout<CodeTextBrush> {
        &self.layout
    }

    pub fn cursor_for_point(&self, point: Point) -> Cursor {
        // TODO: This is a mostly good first pass, but doesn't handle cursor positions in
        // grapheme clusters within a parley cluster.
        // We can also try
        Cursor::from_point(&self.layout, point.x as f32, point.y as f32)
    }

    /// Rebuild the inner layout as needed, adding attributes to the underlying layout.
    ///
    /// See [`Self::rebuild`] for more information
    pub fn rebuild_with_attributes(
        &mut self,
        text: &str,
        attributes: impl for<'b> FnOnce(
            RangedBuilder<'b, CodeTextBrush>,
        ) -> RangedBuilder<'b, CodeTextBrush>,
    ) {
        // TODO - check against self.last_text_start
        let theme = get_theme();

        let mut builder = self.text_layout_ctx.ranged_builder(
            &mut self.font_ctx,
            text,
            theme.scale,
        );
        builder.push_default(StyleProperty::Brush(theme.text_color.into()));
        builder.push_default(StyleProperty::FontSize(theme.text_size as f32));
        builder.push_default(StyleProperty::FontStack(self.font.clone()));
        builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
        builder.push_default(StyleProperty::FontStyle(FontStyle::Normal));

        let mut builder = attributes(builder);
        builder.build_into(&mut self.layout, text);
        self.layout.break_all_lines(self.max_advance);
    }

    pub fn scroll(&mut self, delta: Vec2) {
        const SCROLLING_SPEED: f64 = 2.0;
        // TODO: Horizontal scroll
        let delta =
            Vec2::new(delta.x * -SCROLLING_SPEED, delta.y * -SCROLLING_SPEED);
        if self.scroll + delta.y < 0.0 {
            self.scroll = 0.0;
        }
        self.scroll += delta.y;
    }

    fn draw_underline(
        scene: &mut Scene,
        underline: &Decoration<CodeTextBrush>,
        glyph_run: &GlyphRun<'_, CodeTextBrush>,
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
            &underline.brush.text,
            Some(Affine::IDENTITY),
            &underline_shape,
        );
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
        let sin_len: f64 = 2.0 * f64::consts::PI * sin_scale;
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

    fn draw_curly_underline(
        scene: &mut Scene,
        underline: &Decoration<CodeTextBrush>,
        glyph_run: &GlyphRun<'_, CodeTextBrush>,
        run_metrics: &RunMetrics,
        line_metrics: &LineMetrics,
        transform: &Affine,
    ) {
        let offset = underline.offset.unwrap_or(run_metrics.underline_offset) as f64;
        let stroke_size =
            underline.size.unwrap_or(run_metrics.underline_size) as f64;
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
        let curly_path = Self::curly_path(left, right, y_top, y_bottom, 0.0);

        scene.stroke(
            &stroke,
            *transform,
            &underline.brush.text,
            Some(Affine::IDENTITY),
            &curly_path,
        );

        scene.pop_layer();
    }

    fn draw_strikethrough(
        scene: &mut Scene,
        strikethrough: &Decoration<CodeTextBrush>,
        glyph_run: &GlyphRun<'_, CodeTextBrush>,
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
            &strikethrough.brush.text,
            Some(Affine::IDENTITY),
            &strikethrough_shape,
        );
    }

    pub fn draw(&mut self, scene: &mut Scene, cursor_position: usize, size: Size) {
        let cursor = Cursor::from_byte_index(
            &self.layout,
            cursor_position,
            parley::Affinity::Upstream,
        );
        let cursor_rect = cursor.geometry(&self.layout, 1.5);
        println!("self.scroll: {}", self.scroll);
        let transform = Affine::translate((0.0, -self.scroll));
        // TODO: Selection
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &cursor_rect);
        scene.push_layer(
            BlendMode::default(),
            1.,
            Affine::IDENTITY,
            &size.to_rect(),
        );

        let mut top_line_index = if let Some((cluster, _)) =
            Cluster::from_point(&self.layout, 0.0, self.scroll as f32)
        {
            cluster.path().line_index()
        } else {
            0
        };

        let height = (self.scroll + size.height) as f32;

        while let Some(line) = self.layout.get(top_line_index) {
            let line_metrics = line.metrics();
            if line_metrics.min_coord > height {
                break;
            }
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let text_color = &style.brush.text;

                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let coords = run.normalized_coords();
                scene
                    .draw_glyphs(font)
                    .brush(text_color)
                    .hint(true)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(coords)
                    .draw(
                        Fill::NonZero,
                        glyph_run.positioned_glyphs().map(|glyph| vello::Glyph {
                            id: glyph.id as _,
                            x: glyph.x,
                            y: glyph.y,
                        }),
                    );

                let run_metrics = run.metrics();
                if let Some(underline) = &style.underline {
                    if underline.brush.curly_underline {
                        Self::draw_curly_underline(
                            scene,
                            underline,
                            &glyph_run,
                            run_metrics,
                            line_metrics,
                            &transform,
                        );
                    } else {
                        Self::draw_underline(
                            scene,
                            underline,
                            &glyph_run,
                            run_metrics,
                            &transform,
                        );
                    }
                }

                if let Some(strikethrough) = &style.strikethrough {
                    Self::draw_strikethrough(
                        scene,
                        strikethrough,
                        &glyph_run,
                        run_metrics,
                        &transform,
                    );
                }
            }
            top_line_index += 1;
        }
        scene.pop_layer();
    }
}

impl std::fmt::Debug for CodeTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextLayout")
            .field("font", &self.font)
            .field("max_advance", &self.max_advance)
            .field("text_hinting", &self.text_hinting)
            .finish_non_exhaustive()
    }
}

impl Default for CodeTextLayout {
    fn default() -> Self {
        Self::new()
    }
}
