use crate::ui::{UIEvents, UIRawHandler};
use corelib::input::MouseButton;
use corelib::math::{Rect, Vec2};
use downcast_rs::{impl_downcast, Downcast};
use draw::{Draw2D, Transform2D};

#[derive(Debug, Copy, Clone)]
pub struct UINodeMetadata {
    pub handler: UIRawHandler,
    pub parent_handler: UIRawHandler,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum UIInput {
    Hover {
        pos: Vec2,
    },
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
    fn input(
        &mut self,
        _input: UIInput,
        _state: &mut S,
        _events: &mut UIEvents<S>,
        _metadata: UINodeMetadata,
    ) {
    }
    fn relayout(&mut self, _state: &mut S, _events: &mut UIEvents<S>, _parent_bounds: Rect) {}
    fn update(&mut self, _state: &mut S, _events: &mut UIEvents<S>, _metadata: UINodeMetadata) {}
    fn render(&mut self, _draw: &mut Draw2D, _state: &S, _metadata: UINodeMetadata) {}
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
