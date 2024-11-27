use std::sync::{Arc, Mutex};

use accesskit::{NodeBuilder, Role};
use masonry::{
    text::{TextBrush, TextLayout},
    AccessCtx, AccessEvent, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, Point,
    PointerEvent, RegisterCtx, Size, TextEvent, Update, UpdateCtx, Widget, WidgetId,
};
use smallvec::SmallVec;
use tracing::debug;
use vello::Scene;
use xilem::{
    core::{Message, MessageResult, View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{buffer::BufferView, theme::get_theme};

pub struct CodeWidget {
    text_changed: bool,
    text_layout: TextLayout,
    buffer_view: Arc<Mutex<BufferView>>,
    wrap_word: bool,
}

impl CodeWidget {
    pub fn new(buffer_view: &Arc<Mutex<BufferView>>) -> Self {
        let theme = get_theme();
        let mut text_layout = TextLayout::new(theme.text_size as f32);
        let brush: TextBrush = theme.text_color.into();
        text_layout.set_brush(brush);
        Self {
            text_changed: false,
            text_layout,
            buffer_view: buffer_view.clone(),
            wrap_word: true,
        }
    }

    pub fn buffer_view(&self) -> &Arc<Mutex<BufferView>> {
        &self.buffer_view
    }
}

// --- MARK: IMPL WIDGET ---
impl Widget for CodeWidget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        debug!("CodeWidget::on_pointer_event: {event:?}");
        if let PointerEvent::PointerUp(_pointer_button, _pointer_state) = event {
            ctx.request_focus();
            ctx.set_handled();
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        debug!("CodeWidget::on_text_event: {event:?}");
        match event {
            TextEvent::KeyboardKey(key_event, _modifiers_state) => {
                match &key_event.logical_key {
                    winit::keyboard::Key::Named(named_key) => {
                        debug!("winit::keyboard::Key::Named: {:?}", named_key)
                    }
                    winit::keyboard::Key::Character(str) => {
                        debug!("winit::keyboard::Key::Character: {}", str);
                        ctx.request_paint_only();
                    }
                    winit::keyboard::Key::Unidentified(native_key) => {
                        debug!(
                            "winit::keyboard::Key::Unidentified: {:?}",
                            native_key
                        )
                    }
                    winit::keyboard::Key::Dead(dead) => {
                        debug!("winit::keyboard::Key::Dead: {:?}", dead)
                    }
                }
            }
            TextEvent::Ime(ime) => {
                debug!("TextEvent::Ime: {:?}", ime)
            }
            TextEvent::ModifierChange(modifiers_state) => {
                debug!("TextEvent::ModifierChange: {:?}", modifiers_state)
            }
            TextEvent::FocusChange(focus) => {
                debug!("TextEvent::FocusChange: {}", focus)
            }
        }
    }

    fn on_access_event(&mut self, _ctx: &mut EventCtx, event: &AccessEvent) {
        debug!("CodeWidget::on_access_event: {event:?}");
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {
        debug!("CodeWidget::register_children");
        // Register scroll bars
        // And possilby line count gutter???
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, event: &Update) {
        debug!("CodeWidget::update: {event:?}");
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        debug!("CodeWidget::layout: {bc:?}");
        if self.text_layout.needs_rebuild() || self.text_changed {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            let text: String = self.buffer_view.lock().unwrap().buffer().rope.slice(..).into();
            self.text_layout
                .rebuild(font_ctx, layout_ctx, &text, self.text_changed);
            self.text_changed = false;
        }

        bc.max()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, scene: &mut Scene) {
        debug!("CodeWidget::paint");
        if self.text_layout.needs_rebuild() {
            panic!(
                "Called {name}::paint with invalid layout",
                name = self.short_type_name()
            );
        }
        self.text_layout.draw(scene, Point::new(0.0, 0.0));
    }

    fn accessibility_role(&self) -> Role {
        Role::TextInput
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut NodeBuilder) {
        debug!("CodeWidget::accessibility");
        // node.set_name(???);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        debug!("CodeWidget::children_ids");
        SmallVec::new()
    }

    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn get_debug_text(&self) -> Option<String> {
        Some("CodeWidget".into())
    }

    fn on_anim_frame(&mut self, _ctx: &mut UpdateCtx, interval: u64) {
        debug!("CodeWidget::on_anim_frame interval: {interval}");
    }

    fn compose(&mut self, _ctx: &mut masonry::ComposeCtx) {
        debug!("CodeWidget::compose");
    }

    fn accepts_focus(&self) -> bool {
        debug!("CodeWidget::accepts_focus");
        true
    }

    fn accepts_text_input(&self) -> bool {
        debug!("CodeWidget::accepts_text_input");
        true
    }

    fn get_cursor(
        &self,
        _ctx: &masonry::QueryCtx,
        pos: Point,
    ) -> masonry::CursorIcon {
        debug!("CodeWidget::get_cursor: {pos:?}");
        masonry::CursorIcon::Text
    }
}

pub struct CodeView<F> {
    buffer_view: Arc<Mutex<BufferView>>,
    code_updated: F,
}

pub fn code_view<State, Action>(
    buffer_view: &Arc<Mutex<BufferView>>,
    code_updated: impl Fn(&mut State) -> Action + Send + 'static,
) -> CodeView<impl for<'a> Fn(&'a mut State) -> MessageResult<Action> + Send + 'static>
{
    CodeView {
        buffer_view: buffer_view.clone(),
        code_updated: move |state: &mut State| {
            MessageResult::Action(code_updated(state))
        },
    }
}

impl<F> ViewMarker for CodeView<F> {}
impl<F, State, Action> View<State, Action, ViewCtx> for CodeView<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State) -> MessageResult<Action> + Send + Sync + 'static,
{
    type Element = Pod<CodeWidget>;

    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        debug!("CodeView::build");
        ctx.with_leaf_action_widget(|ctx| {
            ctx.new_pod(CodeWidget::new(&self.buffer_view))
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
        app_state: &mut State,
    ) -> xilem::core::MessageResult<Action, Box<dyn Message>> {
        debug!("CodeView::message");
        match message.downcast::<masonry::Action>() {
            Ok(action) => {
                if let masonry::Action::TextChanged(_text) = *action {
                    (self.code_updated)(app_state)
                } else {
                    tracing::error!(
                        "Wrong action type in CodeView::message: {action:?}"
                    );
                    MessageResult::Stale(action)
                }
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
