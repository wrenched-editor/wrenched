use kurbo::{Affine, Size, Vec2};
use parley::{
    fontique::{Collection, CollectionOptions, Style}, layout::Cursor, style::{FontFamily, GenericFamily, StyleProperty}, FontContext, FontStack, Layout, LayoutContext, PositionedLayoutItem, RangedBuilder
};
use peniko::BlendMode;
use vello::{
    kurbo::Point,
    peniko::{self, Color, Fill, Gradient}, Scene,
};
use xilem::TextWeight;

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
#[derive(Clone, Debug, PartialEq)]
pub enum CodeTextBrush {
    Normal(peniko::Brush),
    Highlight {
        text: peniko::Brush,
        fill: peniko::Brush,
    },
}

impl From<peniko::Brush> for CodeTextBrush {
    fn from(value: peniko::Brush) -> Self {
        Self::Normal(value)
    }
}

impl From<Gradient> for CodeTextBrush {
    fn from(value: Gradient) -> Self {
        Self::Normal(value.into())
    }
}

impl From<Color> for CodeTextBrush {
    fn from(value: Color) -> Self {
        Self::Normal(value.into())
    }
}

// Parley requires their Brush implementations to implement Default
impl Default for CodeTextBrush {
    fn default() -> Self {
        Self::Normal(Default::default())
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

        // Workaround for how parley treats empty lines.
        //let text = if !text.is_empty() { text } else { " " };

        let mut builder = self.text_layout_ctx.ranged_builder(&mut self.font_ctx, text, theme.scale);
        builder.push_default(StyleProperty::Brush(theme.text_color.into()));
        builder.push_default(StyleProperty::FontSize(theme.text_size as f32));
        builder.push_default(StyleProperty::FontStack(self.font.clone()));
        builder.push_default(StyleProperty::FontWeight(TextWeight::NORMAL));
        builder.push_default(StyleProperty::FontStyle(Style::Normal));

        // Currently, this is used for:
        // - underlining IME suggestions
        // - applying a brush to selected text.
        let mut builder = attributes(builder);
        builder.build_into(&mut self.layout, text);
        self.layout
            .break_all_lines(self.max_advance);
    }

    pub fn scroll(&mut self, delta: Vec2) {
        const SCROLLING_SPEED: f64 = 2.0;
        // TODO: Horizontal scroll
        let delta = Vec2::new(delta.x * -SCROLLING_SPEED, delta.y * -SCROLLING_SPEED);
        if self.scroll + delta.y < 0.0 {
            self.scroll = 0.0;
        }
        self.scroll += delta.y;
    }

    pub fn draw(&mut self, scene: &mut Scene, cursor_position: usize, size: Size) {
        let cursor = Cursor::from_byte_index(&self.layout, cursor_position, parley::Affinity::Upstream);
        let transform = Affine::IDENTITY;
        let cursor_rect = cursor.geometry(&self.layout, 1.5);
        println!("self.scroll: {}", self.scroll);
        // TODO: Selection
        scene.fill(Fill::NonZero, transform, Color::WHITE, None, &cursor_rect);
        scene.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &size.to_rect());
        for line in self.layout.lines() {
            let metrics = line.metrics();
            let up = metrics.baseline + metrics.ascent;
            let down = metrics.baseline + metrics.descent;
            if (down as f64) < self.scroll {
                continue;
            }
            if (up  as f64) > self.scroll + size.height{
                break;
            }
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline() - (self.scroll as f32);
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let coords = run
                    .normalized_coords()
                    .iter()
                    .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                    .collect::<Vec<_>>();
                scene
                    .draw_glyphs(font)
                    .brush(Color::WHITE)
                    .hint(true)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(&coords)
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|glyph| {
                            let gx = x + glyph.x;
                            let gy = y - glyph.y;
                            x += glyph.advance;
                            vello::Glyph {
                                id: glyph.id as _,
                                x: gx,
                                y: gy,
                            }
                        }),
                    );
            }
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
