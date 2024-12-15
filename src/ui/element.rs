use crate::ui::events::UIEventQueue;
use corelib::input::MouseButton;
use corelib::math::{Rect, Vec2};
use downcast_rs::{impl_downcast, Downcast};
use draw::{Draw2D, Transform2D};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum UIInput {
    Hover,
    CursorEnter,
    CursorLeave,
    ButtonPressed(MouseButton),
    ButtonDown(MouseButton),
    ButtonReleased(MouseButton),
    ButtonClick(MouseButton),
    ButtonReleasedAnywhere(MouseButton),
    Scroll {
        delta: Vec2,
    },
    DragStart {
        pos: Vec2,
        btn: MouseButton,
    },
    Dragging {
        start_pos: Vec2,
        pos: Vec2,
        frame_delta: Vec2,
        btn: MouseButton,
    },
    DragEnd {
        pos: Vec2,
        btn: MouseButton,
    },
}

pub trait UIElement<S>: Downcast + Send + Sync {
    fn transform(&self) -> &Transform2D;
    fn transform_mut(&mut self) -> &mut Transform2D;
    fn input_box(&self) -> Rect {
        Rect::new(Vec2::ZERO, self.transform().size())
    }
    fn input(&mut self, input: UIInput, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn update(&mut self, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn render(&mut self, draw: &mut Draw2D, state: &S) {}
}

impl_downcast!(UIElement<S>);

pub struct UIRoot {
    pub transform: Transform2D,
}
impl<S> UIElement<S> for UIRoot {
    fn transform(&self) -> &Transform2D {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform2D {
        &mut self.transform
    }
}
