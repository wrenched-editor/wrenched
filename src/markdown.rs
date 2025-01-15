use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use accesskit::Role;
use kurbo::{Affine, Cap, Join, Line, Size, Stroke};
use masonry::Widget;
use parley::{
    Cluster, Decoration, FontContext, FontStyle, GlyphRun, Layout, LayoutContext,
    PositionedLayoutItem, RangedBuilder, RunMetrics, StyleProperty,
};
use peniko::{BlendMode, Color, Fill, Image};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use smallvec::SmallVec;
use tracing::{debug, warn};
use vello::Scene;
use xilem::{
    core::{Message, MessageResult, View, ViewMarker},
    Pod, TextWeight, ViewCtx,
};

use crate::{
    layout_flow::{LayoutData, LayoutFlow},
    theme::{get_theme, Theme},
};

#[derive(Clone)]
pub enum MarkdownContent {
    Text {
        top_margine: f32,
        text: String,
        markers: Vec<TextMarker>,
        text_layout: Layout<Color>,
    },
    Image {
        uri: String,
        title: String,
        imge: Image,
    },
    CodeBlock {
        text: String,
        text_layout: Layout<Color>,
    },
}

impl LayoutData for MarkdownContent {
    fn height(&self) -> f32 {
        match self {
            MarkdownContent::Text {
                top_margine,
                text: _,
                markers: _,
                text_layout,
            } => text_layout.height() + top_margine,
            MarkdownContent::Image {
                uri: _,
                title: _,
                imge: image,
            } => image.height as f32,
            MarkdownContent::CodeBlock {
                text: _,
                text_layout,
            } => text_layout.height(),
        }
    }
}

#[derive(Clone)]
pub struct TextMarker {
    // TODO: Think about making it into range
    start_pos: usize,
    end_pos: usize,
    kind: MarkerKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MarkerKind {
    Bold,
    Italic,
    Header(HeadingLevel),
    Strikethrough,
    InlineCode,
}

fn parse_markdown(text: &str) -> LayoutFlow<MarkdownContent> {
    let mut res = LayoutFlow::new();

    // TODO: Does the tag stack makes sense to have???
    let mut tag_stack: SmallVec<[Tag; 4]> = SmallVec::new();

    let parser = Parser::new_ext(
        text,
        //Options::ENABLE_TABLES
        //| Options::ENABLE_FOOTNOTES
        //| Options::ENABLE_STRIKETHROUGH
        Options::ENABLE_STRIKETHROUGH, //| Options::ENABLE_TASKLISTS
                                       //| Options::ENABLE_HEADING_ATTRIBUTES,
    );

    let mut text = String::new();
    let mut text_markers: Vec<TextMarker> = Vec::new();

    // Currently running markers
    let mut running_markers: HashMap<MarkerKind, usize> = HashMap::new();

    // When the tag is image we want to ignore text because it is a title.
    // TODO: Think about adding the image title to the renderer
    let mut ignore_text: bool = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                if let Some(marker_kind) = tag_to_marker_kind(&tag) {
                    running_markers.insert(marker_kind, text.len());
                }
                match &tag {
                    Tag::Image {
                        link_type: _,
                        dest_url: _,
                        title: _,
                        id: _,
                    } => {
                        ignore_text = true;
                        // TODO: Reset the running_markers, text and push into the res.
                    }
                    Tag::CodeBlock(_kind) => {
                        // TODO: Reset the running_markers, text and push into the res.
                    }
                    Tag::Table(_alignments) => {
                        // TODO: Reset the running_markers, text and push into the res.
                    }
                    _ => {}
                }
                tag_stack.push(tag);
            }
            Event::End(_end_tag) => {
                if let Some(tag) = tag_stack.pop() {
                    if let Some(marker_kind) = tag_to_marker_kind(&tag) {
                        if let Some(start_pos) = running_markers.remove(&marker_kind)
                        {
                            text_markers.push(TextMarker {
                                start_pos,
                                end_pos: text.len(),
                                kind: marker_kind,
                            });
                        } else {
                            warn!("markdown markers are not paired");
                        }
                    }

                    match &tag {
                        Tag::Heading {
                            level: _,
                            id: _,
                            classes: _,
                            attrs: _,
                        } => text.push('\n'),
                        Tag::Paragraph => {
                            if !text.is_empty() {
                                for (&marker_kind, &start_pos) in
                                    running_markers.iter()
                                {
                                    text_markers.push(TextMarker {
                                        start_pos,
                                        end_pos: text.len(),
                                        kind: marker_kind,
                                    });
                                }
                                res.push(MarkdownContent::Text {
                                    // TODO: Make nice offset
                                    // TODO: This should be in theme as well
                                    top_margine: 12.0,
                                    text: text.clone(),
                                    markers: text_markers.clone(),
                                    text_layout: Layout::new(),
                                });
                            }
                            text.clear();
                            text_markers.clear();
                            for v in running_markers.values_mut() {
                                *v = 0;
                            }
                        }
                        Tag::CodeBlock(_kind) => {}
                        Tag::Image {
                            link_type: _,
                            dest_url: _,
                            title: _,
                            id: _,
                        } => {
                            ignore_text = false;
                            // TODO: Are there any link types that would change how the
                            // image is rendered?
                        }
                        _ => {}
                    }
                } else {
                    tracing::warn!("Unbalanced markdown tag")
                }
            }
            Event::Text(text_bit) => {
                if ignore_text {
                    continue;
                }
                text.push_str(&text_bit);
            }
            Event::Code(text_bit) => {
                text_markers.push(TextMarker {
                    start_pos: text.len(),
                    end_pos: text.len() + text_bit.len(),
                    kind: MarkerKind::InlineCode,
                });
                text.push_str(&text_bit);
            }
            Event::Html(text_bit) => {
                // TODO: This looks a bit fishy
                text_markers.push(TextMarker {
                    start_pos: text.len(),
                    end_pos: text.len() + text_bit.len(),
                    kind: MarkerKind::InlineCode,
                });
                text.push_str(&text_bit);
            }
            Event::HardBreak => {
                text.push('\n');
            }
            Event::SoftBreak => {
                text.push(' ');
            }
            Event::Rule => {}
            Event::FootnoteReference(_text) => {}
            Event::TaskListMarker(_text) => {}
            Event::InlineHtml(_) => {}
            Event::InlineMath(_) => {}
            Event::DisplayMath(_) => {}
        }
    }

    if !text.is_empty() {
        for (&marker_kind, &start_pos) in running_markers.iter() {
            text_markers.push(TextMarker {
                start_pos,
                end_pos: text.len(),
                kind: marker_kind,
            });
        }
        res.push(MarkdownContent::Text {
            // TODO: Make nice offset
            // TODO: This should be in theme as well
            // TODO: It should be relative to the font size
            top_margine: 12.0,
            text,
            markers: text_markers,
            text_layout: Layout::new(),
        });
    }

    res
}

fn tag_to_marker_kind(tag: &Tag) -> Option<MarkerKind> {
    match tag {
        Tag::Heading {
            level,
            id: _,
            classes: _,
            attrs: _,
        } => Some(MarkerKind::Header(*level)),
        Tag::BlockQuote(_block_quote) => None,
        Tag::CodeBlock(_) => None,
        Tag::Emphasis => Some(MarkerKind::Italic),
        Tag::Strong => Some(MarkerKind::Bold),
        Tag::Strikethrough => Some(MarkerKind::Strikethrough),
        Tag::Link {
            link_type: _,
            dest_url: _,
            title: _,
            id: _,
        } => {
            // TODO: Link support
            None
        }
        // TODO: Go through the tags and see what else can be done.
        _ => None,
    }
}

fn feed_marker_to_builder<'a>(
    builder: &'a mut RangedBuilder<Color>,
    text_marker: &TextMarker,
    theme: &'a Theme,
) {
    let rang = text_marker.start_pos..text_marker.end_pos;
    match text_marker.kind {
        MarkerKind::Bold => {
            builder.push(StyleProperty::FontWeight(TextWeight::BOLD), rang)
        }
        MarkerKind::Italic => {
            builder.push(StyleProperty::FontStyle(FontStyle::Italic), rang)
        }
        MarkerKind::Header(heading_level) => {
            let font_size = match heading_level {
                HeadingLevel::H1 => theme.text_size as f32 * 2.125,
                HeadingLevel::H2 => theme.text_size as f32 * 1.875,
                HeadingLevel::H3 => theme.text_size as f32 * 1.5,
                HeadingLevel::H4 => theme.text_size as f32 * 1.25,
                HeadingLevel::H5 => theme.text_size as f32 * 1.125,
                HeadingLevel::H6 => theme.text_size as f32,
            };
            let line_height = match heading_level {
                // TODO: Experiment with line height to get better results???
                HeadingLevel::H1 => 2.0,
                HeadingLevel::H2 => 2.0,
                HeadingLevel::H3 => 2.0,
                HeadingLevel::H4 => 2.0,
                HeadingLevel::H5 => 2.0,
                HeadingLevel::H6 => 2.0,
            };
            builder.push(StyleProperty::FontSize(font_size), rang.clone());
            builder.push(StyleProperty::LineHeight(line_height), rang.clone());
            builder.push(StyleProperty::FontWeight(TextWeight::BOLD), rang);
        }
        MarkerKind::Strikethrough => {
            builder.push(StyleProperty::Strikethrough(true), rang)
        }
        MarkerKind::InlineCode => {
            builder.push(
                StyleProperty::FontStack(theme.monospace_font_stack.clone()),
                rang.clone(),
            );
            builder.push(StyleProperty::Brush(theme.monospace_text_color), rang);
        }
    }
}

fn text_to_layout(
    text: &str,
    markers: &[TextMarker],
    font_ctx: &mut FontContext,
    layout_ctx: &mut LayoutContext<Color>,
) -> Layout<Color> {
    let theme = get_theme();

    let mut builder: RangedBuilder<'_, Color> =
        layout_ctx.ranged_builder(font_ctx, text, theme.scale);
    builder.push_default(StyleProperty::Brush(theme.text_color));
    builder.push_default(StyleProperty::FontSize(theme.text_size as f32));
    builder.push_default(StyleProperty::FontStack(theme.font_stack.clone()));
    builder.push_default(StyleProperty::FontWeight(TextWeight::NORMAL));
    builder.push_default(StyleProperty::FontStyle(FontStyle::Normal));
    builder.push_default(StyleProperty::LineHeight(1.1));
    for marker in markers.iter() {
        feed_marker_to_builder(&mut builder, marker, &theme);
    }
    builder.build(text)
}

pub struct MarkdowWidget {
    markdown_layout: LayoutFlow<MarkdownContent>,
    content: String,
    file: PathBuf,
    layout_ctx: LayoutContext<Color>,
    max_advance: f64,
    dirty: bool,
}

impl MarkdowWidget {
    pub fn new<P: AsRef<Path>>(markdown_file: P) -> Self {
        // TODO: Ehm... unwraps...
        let content: String =
            String::from_utf8(std::fs::read(&markdown_file).unwrap()).unwrap();
        let markdown_layout = parse_markdown(&content);
        Self {
            markdown_layout,
            file: markdown_file.as_ref().to_path_buf(),
            content,
            dirty: true,
            layout_ctx: LayoutContext::new(),
            max_advance: 0.0,
        }
    }
    fn draw_underline(
        scene: &mut Scene,
        underline: &Decoration<Color>,
        glyph_run: &GlyphRun<'_, Color>,
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
            underline.brush,
            Some(Affine::IDENTITY),
            &underline_shape,
        );
    }
    fn draw_strikethrough(
        scene: &mut Scene,
        strikethrough: &Decoration<Color>,
        glyph_run: &GlyphRun<'_, Color>,
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
            strikethrough.brush,
            Some(Affine::IDENTITY),
            &strikethrough_shape,
        );
    }

    fn draw(scene: &mut Scene, layout: &Layout<Color>, size: Size, scroll: f32) {
        let transform = Affine::translate((0.0, -scroll as f64));
        scene.push_layer(
            BlendMode::default(),
            1.,
            Affine::IDENTITY,
            &size.to_rect(),
        );

        let mut top_line_index =
            if let Some((cluster, _)) = Cluster::from_point(layout, 0.0, scroll) {
                cluster.path().line_index()
            } else {
                0
            };

        let height = scroll + (size.height as f32);

        while let Some(line) = layout.get(top_line_index) {
            let line_metrics = line.metrics();
            if line_metrics.min_coord > height {
                break;
            }
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let text_color = &style.brush;

                let run = glyph_run.run();
                // TODO: This needs to be some kind of a flow layout.
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let coords = run
                    .normalized_coords()
                    .iter()
                    .map(|coord| {
                        vello::skrifa::instance::NormalizedCoord::from_bits(*coord)
                    })
                    .collect::<Vec<_>>();
                scene
                    .draw_glyphs(font)
                    .brush(text_color)
                    .hint(true)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(&coords)
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
                    Self::draw_underline(
                        scene,
                        underline,
                        &glyph_run,
                        run_metrics,
                        &transform,
                    );
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

impl Widget for MarkdowWidget {
    fn register_children(&mut self, _ctx: &mut masonry::RegisterCtx) {}

    fn layout(
        &mut self,
        ctx: &mut masonry::LayoutCtx,
        bc: &masonry::BoxConstraints,
    ) -> kurbo::Size {
        let size = bc.max();
        // TODO: Think about putting the context into the theme??? Or somewhere else???
        let (font_ctx, _layout_ctx) = ctx.text_contexts();
        if self.dirty || self.max_advance != size.width {
            self.markdown_layout.apply_to_all(|data| match data {
                MarkdownContent::Text {
                    text,
                    markers,
                    top_margine: _,
                    text_layout,
                } => {
                    let mut layout = text_to_layout(
                        text,
                        markers,
                        font_ctx,
                        &mut self.layout_ctx,
                    );
                    layout.break_all_lines(Some(size.width as f32));
                    *text_layout = layout;
                }
                MarkdownContent::Image {
                    uri: _,
                    title: _,
                    imge: _,
                } => todo!(),
                MarkdownContent::CodeBlock {
                    text: _,
                    text_layout: _,
                } => todo!(),
            });
        }
        self.max_advance = size.width;
        self.dirty = false;
        size
    }

    fn paint(&mut self, ctx: &mut masonry::PaintCtx, scene: &mut vello::Scene) {
        // TODO: Make scroll work
        let scroll = 0.0;
        let visible_parts = self
            .markdown_layout
            .get_visible_parts(scroll, ctx.size().height as f32);
        for visible_part in visible_parts {
            match &visible_part.data {
                MarkdownContent::Text {
                    top_margine,
                    text: _,
                    markers: _,
                    text_layout,
                } => Self::draw(
                    scene,
                    text_layout,
                    ctx.size(),
                    scroll - visible_part.offset + top_margine,
                ),
                MarkdownContent::Image {
                    uri: _,
                    title: _,
                    imge: _,
                } => todo!(),
                MarkdownContent::CodeBlock {
                    text: _,
                    text_layout: _,
                } => todo!(),
            }
        }
    }

    fn accessibility_role(&self) -> accesskit::Role {
        Role::Document
    }

    fn accessibility(
        &mut self,
        _ctx: &mut masonry::AccessCtx,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[masonry::WidgetId; 16]> {
        SmallVec::new()
    }
}

///// Highlight the text in a richtext builder like it was a markdown codeblock
//pub fn highlight_as_code(
//    attr_list: &mut AttrsList,
//    default_attrs: Attrs,
//    language: Option<LapceLanguage>,
//    text: &str,
//    start_offset: usize,
//    config: &LapceConfig,
//) {
//    let syntax = language.map(Syntax::from_language);
//
//    let styles = syntax
//        .map(|mut syntax| {
//            syntax.parse(0, Rope::from(text), None);
//            syntax.styles
//        })
//        .unwrap_or(None);
//
//    if let Some(styles) = styles {
//        for (range, style) in styles.iter() {
//            if let Some(color) = style
//                .fg_color
//                .as_ref()
//                .and_then(|fg| config.style_color(fg))
//            {
//                attr_list.add_span(
//                    start_offset + range.start..start_offset + range.end,
//                    default_attrs.color(color),
//                );
//            }
//        }
//    }
//}

pub struct MarkdownView {
    path: PathBuf,
}

pub fn markdown_view(path: PathBuf) -> MarkdownView {
    MarkdownView { path }
}

impl ViewMarker for MarkdownView {}
impl<State, Action> View<State, Action, ViewCtx> for MarkdownView
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<MarkdowWidget>;

    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        debug!("CodeView::build");
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod(MarkdowWidget::new(&self.path))
        })
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: xilem::core::Mut<Self::Element>,
    ) {
        debug!("CodeView::rebuild");
    }

    fn teardown(
        &self,
        _view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<Self::Element>,
    ) {
        debug!("CodeView::teardown");
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[xilem::core::ViewId],
        message: Box<dyn Message>,
        _app_state: &mut State,
    ) -> xilem::core::MessageResult<Action, Box<dyn Message>> {
        debug!("CodeView::message");
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                tracing::error!(
                    "Wrong action type in CodeView::message: {action:?}"
                );
                MessageResult::Stale(action)
            }
            Err(message) => {
                tracing::error!(
                    "Wrong message type in Button::message: {message:?}"
                );
                MessageResult::Stale(message)
            }
        }
    }
}
