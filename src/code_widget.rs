use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use accesskit::{Node, Role};
use kurbo::{Point, Size, Vec2};
use masonry::core::{
    AccessCtx, AccessEvent, BoxConstraints, ComposeCtx, EventCtx, LayoutCtx,
    PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, WidgetId,
};
use parley::StyleProperty;
use smallvec::SmallVec;
use tracing::debug;
use vello::{peniko::Color, Scene};
use winit::window::CursorIcon;
use xilem::{
    core::{Message, MessageResult, View, ViewMarker},
    view::PointerButton,
    FontWeight, Pod, ViewCtx,
};

use crate::{
    buffer::BufferView,
    code_text_layout::{CodeTextBrush, CodeTextLayout},
};

pub struct CodeWidget {
    text_changed: bool,
    text_layout: CodeTextLayout,
    buffer_view: Arc<Mutex<BufferView>>,
    wrap_word: bool,
}

impl CodeWidget {
    pub fn new(buffer_view: &Arc<Mutex<BufferView>>) -> Self {
        let text_layout = CodeTextLayout::new();
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

// TODO: List of decorations for code editor:
//
// * Text color
// * BG color
// * Bold
// * Underline in color
// * Ghost text
// * Syntax
//   * Next bracket
//   * Next word
// * Empty trailing spaces
// * Indentation guides (vertical lines indication indentation)

// --- MARK: IMPL WIDGET ---
impl Widget for CodeWidget {
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        debug!("CodeWidget::on_pointer_event: {event:?}");
        if let PointerEvent::PointerDown(PointerButton::Primary, pointer_state) =
            event
        {
            let point = pointer_state.position;
            let window_origin = ctx.window_origin();
            debug!("CodeWidget::on_pointer_event; point: {point:?}");
            let cursor_point = self.text_layout.cursor_for_point(
                (point.x - window_origin.x, point.y - window_origin.y).into(),
            );
            let mut buffer_view = self.buffer_view().lock().unwrap();

            debug!("CodeWidget::on_pointer_event; cursor_point: {cursor_point:?}");
            buffer_view.set_position_bytes(cursor_point.index());
            ctx.request_focus();
            ctx.request_paint_only();
            ctx.set_handled();
        } else if let PointerEvent::MouseWheel(delta, _) = event {
            self.text_layout.scroll(Vec2::new(delta.x, delta.y));
            ctx.request_paint_only();
            ctx.set_handled();
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        debug!("CodeWidget::on_text_event: {event:?}");
        macro_rules! process_key {
            ($action:ident) => {
                self.text_changed = true;
                let mut buffer_view = self.buffer_view().lock().unwrap();
                buffer_view.$action();
                ctx.request_layout();
                ctx.set_handled();
            };
            ($action:ident, $param:expr) => {
                self.text_changed = true;
                let mut buffer_view = self.buffer_view().lock().unwrap();
                buffer_view.$action($param);
                ctx.request_layout();
                ctx.set_handled();
            };
        }
        match event {
            TextEvent::KeyboardKey(key_event, _modifiers_state) => {
                if !key_event.state.is_pressed() {
                    return;
                }
                match &key_event.logical_key {
                    winit::keyboard::Key::Named(named_key) => {
                        debug!("winit::keyboard::Key::Named: {:?}", named_key);
                        match named_key {
                            winit::keyboard::NamedKey::Enter => {
                                process_key!(insert_new_line);
                            }
                            winit::keyboard::NamedKey::Tab => {
                                process_key!(insert_at_point, "\t");
                            }
                            winit::keyboard::NamedKey::Space => {
                                process_key!(insert_at_point, " ");
                            }
                            winit::keyboard::NamedKey::ArrowUp => {
                                process_key!(move_point_forward_line);
                            }
                            winit::keyboard::NamedKey::ArrowDown => {
                                process_key!(move_point_backward_line);
                            }
                            winit::keyboard::NamedKey::ArrowLeft => {
                                process_key!(move_point_backward_char);
                            }
                            winit::keyboard::NamedKey::ArrowRight => {
                                process_key!(move_point_forward_char);
                            }
                            winit::keyboard::NamedKey::Delete => {
                                process_key!(delete_at_point);
                            }
                            winit::keyboard::NamedKey::Backspace => {
                                self.text_changed = true;
                                let mut buffer_view =
                                    self.buffer_view().lock().unwrap();
                                buffer_view.move_point_backward_char();
                                buffer_view.delete_at_point();
                                ctx.request_layout();
                                ctx.set_handled();
                            }
                            _ => {
                                debug!(
                                    "CodeView unimplemented Key::Named: {:?}",
                                    named_key
                                )
                            }
                        }
                    }
                    winit::keyboard::Key::Character(str) => {
                        debug!("winit::keyboard::Key::Character: {}", str);
                        process_key!(insert_at_point, str);
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
            TextEvent::WindowFocusChange(focus) => {
                debug!("TextEvent::WindowFocusChange: {}", focus)
            }
        }
    }

    fn on_access_event(
        &mut self,
        _ctx: &mut EventCtx,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        debug!("CodeWidget::on_access_event: {event:?}");
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx) {
        debug!("CodeWidget::register_children");
        // Register scroll bars
        // And possilby line count gutter???
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _props: &mut PropertiesMut<'_>,
        event: &Update,
    ) {
        debug!("CodeWidget::update: {event:?}");
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let text: String = self
            .buffer_view
            .lock()
            .unwrap()
            .buffer()
            .rope
            .slice(..)
            .into();
        let size = bc.max();
        self.text_layout.set_max_advance(Some(size.width as f32));
        let start = Instant::now();
        let curly_brush = Some(CodeTextBrush {
            text: Color::from_rgb8(0xf0, 0x00, 0x00).into(),
            backgroud: None,
            curly_underline: true,
        });
        self.text_layout.rebuild_with_attributes(&text, |mut b| {
            b.push(StyleProperty::Underline(true), 0..100);
            b.push(
                StyleProperty::Brush(Color::from_rgb8(0xff, 0x00, 0xff).into()),
                40..100,
            );
            b.push(
                StyleProperty::UnderlineBrush(Some(
                    Color::from_rgb8(0xf0, 0x50, 0x10).into(),
                )),
                0..100,
            );
            b.push(StyleProperty::FontWeight(FontWeight::BOLD), 100..200);
            b.push(
                StyleProperty::Brush(Color::from_rgb8(0x10, 0xf0, 0x10).into()),
                100..200,
            );
            b.push(StyleProperty::Strikethrough(true), 200..300);
            b.push(
                StyleProperty::StrikethroughBrush(Some(
                    Color::from_rgb8(0x50, 0x50, 0xf0).into(),
                )),
                200..300,
            );
            b.push(StyleProperty::StrikethroughSize(Some(3.0)), 200..250);
            b.push(
                StyleProperty::Brush(Color::from_rgb8(0xA0, 0xA0, 0xA0).into()),
                300..350,
            );
            b.push(StyleProperty::Underline(true), 300..332);
            b.push(StyleProperty::UnderlineBrush(curly_brush), 300..332);
            b
        });
        let since_the_epoch = start.elapsed();
        println!(
            "Time of text layouting: {:?}s",
            since_the_epoch.as_secs_f32()
        );
        size
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx,
        _props: &PropertiesRef<'_>,
        scene: &mut Scene,
    ) {
        debug!("CodeWidget::paint");
        let position = {
            let buffer_view = self.buffer_view().lock().unwrap();
            buffer_view.position_bytes()
        };
        self.text_layout.draw(scene, position, ctx.size());
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

    fn on_anim_frame(
        &mut self,
        _ctx: &mut UpdateCtx,
        _props: &mut PropertiesMut<'_>,
        interval: u64,
    ) {
        debug!("CodeWidget::on_anim_frame interval: {interval}");
    }

    fn compose(&mut self, _ctx: &mut ComposeCtx) {
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

    fn get_cursor(&self, _ctx: &QueryCtx, pos: Point) -> CursorIcon {
        debug!("CodeWidget::get_cursor: {pos:?}");
        CursorIcon::Text
    }

    fn accessibility_role(&self) -> Role {
        debug!("CodeWidget::accessibility_role");
        Role::TextInput
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
        debug!("CodeWidget::accessibility");
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
        match message.downcast::<masonry::core::Action>() {
            Ok(action) => {
                if let masonry::core::Action::TextChanged(_text) = *action {
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
