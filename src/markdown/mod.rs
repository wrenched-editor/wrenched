pub mod elements;
pub mod parser;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use accesskit::Role;
use elements::{
    draw_flow, MarkdownBrush, MarkdownContent, MarkdownContext, SvgContext,
};
use kurbo::{Affine, Vec2};
use masonry::core::{EventCtx, PointerEvent, RegisterCtx, Widget};
use parley::LayoutContext;
use parser::parse_markdown;
use peniko::BlendMode;
use smallvec::SmallVec;
use tracing::{debug, info};
use usvg::fontdb;
use xilem::{
    core::{Message, MessageResult, View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{layout_flow::LayoutFlow, scene_utils::SizedScene, theme::get_theme};
pub struct MarkdowWidget {
    markdown_layout: LayoutFlow<MarkdownContent>,
    layout_ctx: LayoutContext<MarkdownBrush>,
    max_advance: f64,
    dirty: bool,
    scroll: Vec2,
    svg_context: SvgContext,
}

impl MarkdowWidget {
    pub fn new<P: AsRef<Path>>(markdown_file: P) -> Self {
        // TODO: Ehm... unwraps...
        let content: String =
            String::from_utf8(std::fs::read(&markdown_file).unwrap()).unwrap();
        let markdown_layout = parse_markdown(&content);
        // TODO: This one should be "global".
        let mut fontdb = fontdb::Database::default();
        fontdb.load_system_fonts();

        // TODO: Add default fonts into the package so they are always present.
        fontdb.set_serif_family("Times New Roman");
        fontdb.set_sans_serif_family("Arial");
        fontdb.set_cursive_family("Comic Sans MS");
        fontdb.set_fantasy_family("Impact");
        fontdb.set_monospace_family("Courier New");

        // FIXME: FIXME FIXME: I'm not sure about the legality of the fonts
        // being committed in the repo. Needs to be resolved ASAP.
        fontdb.load_fonts_dir("./fonts/");

        let svg_context: SvgContext = SvgContext {
            fontdb: Arc::new(fontdb),
        };
        Self {
            markdown_layout,
            dirty: true,
            layout_ctx: LayoutContext::new(),
            max_advance: 0.0,
            scroll: Vec2::new(0.0, 0.0),
            svg_context,
        }
    }
}
impl Widget for MarkdowWidget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        info!("event: {event:?} >>> ctx: {}", ctx.size());
        if let PointerEvent::MouseWheel(delta, _) = event {
            const SCROLLING_SPEED: f64 = 3.0;
            let delta =
                Vec2::new(delta.x * SCROLLING_SPEED, delta.y * SCROLLING_SPEED);
            self.scroll += delta;
            // TODO: horizontal scrolling
            self.scroll.x = 0.0;
            let bounding_box = ctx.bounding_rect();
            info!("widget height: {}", bounding_box);
            self.scroll.y = self.scroll.y.min(0.0);
            self.scroll.y = self.scroll.y.max(
                -self.markdown_layout.height() + bounding_box.height()
                    - ctx.window_origin().y,
            );
            info!("scrolling new scroll: {} , self.markdown_layout.height() {}, ctx.size() {}", self.scroll, self.markdown_layout.height(), ctx.size());
            if let Some(bla) = self.markdown_layout.flow.last() {
                info!("bla.offset: {}", bla.offset);
            }
            ctx.request_paint_only();
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {}

    fn compose(&mut self, ctx: &mut masonry::core::ComposeCtx) {
        info!("compose called: size: {}, baseline_offset: {}, window_origin: {}, layout_rect: {}", ctx.size(), ctx.baseline_offset(), ctx.window_origin(), ctx.layout_rect());
    }

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
        bc: &masonry::core::BoxConstraints,
    ) -> kurbo::Size {
        debug!("cool layout");
        let size = bc.max();
        let theme = &get_theme();

        let (font_ctx, _layout_ctx) = ctx.text_contexts();
        let mut markdown_ctx: MarkdownContext = MarkdownContext {
            svg_ctx: &mut self.svg_context,
            font_ctx,
            layout_ctx: &mut self.layout_ctx,
            theme,
        };

        if self.dirty || self.max_advance != size.width {
            self.markdown_layout.apply_to_all(|(i, data)| {
                data.layout(&mut markdown_ctx, size.width, i == 0);
            });
        }

        self.max_advance = size.width;
        self.dirty = false;
        info!("size: {}", size);
        size
    }

    fn paint(
        &mut self,
        ctx: &mut masonry::core::PaintCtx,
        scene: &mut vello::Scene,
    ) {
        scene.push_layer(
            BlendMode::default(),
            1.,
            Affine::IDENTITY,
            &ctx.size().to_rect(),
        );
        let mut scene = SizedScene::new(scene, ctx.size());
        let theme = &get_theme();
        let (font_ctx, _layout_ctx) = ctx.text_contexts();
        let mut markdown_ctx: MarkdownContext = MarkdownContext {
            svg_ctx: &mut self.svg_context,
            font_ctx,
            layout_ctx: &mut self.layout_ctx,
            theme,
        };
        draw_flow(
            &mut scene,
            &mut markdown_ctx,
            &self.scroll,
            &self.markdown_layout,
        );
        scene.pop_layer();
    }

    fn accessibility_role(&self) -> accesskit::Role {
        Role::Document
    }

    fn accessibility(
        &mut self,
        _ctx: &mut masonry::core::AccessCtx,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> SmallVec<[masonry::core::WidgetId; 16]> {
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
        match message.downcast::<masonry::core::Action>() {
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
