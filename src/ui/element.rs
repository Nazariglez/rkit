use crate::ui::events::UIEventQueue;
use corelib::input::MouseButton;
use downcast_rs::{impl_downcast, Downcast};
use draw::{Draw2D, Transform2D};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum UIInput {
    Hover,
    Enter,
    Leave,
    Pressed(MouseButton),
    Released(MouseButton),
    Clicked(MouseButton),
    GlobalRelease(MouseButton),
}

pub trait UIElement<S>: Downcast + Send + Sync {
    fn transform(&mut self) -> &mut Transform2D;
    fn input(&mut self, input: UIInput, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn update(&mut self, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn render(&mut self, draw: &mut Draw2D, state: &S) {}
}

impl_downcast!(UIElement<S>);

pub struct UIRoot {
    pub transform: Transform2D,
}
impl<S> UIElement<S> for UIRoot {
    fn transform(&mut self) -> &mut Transform2D {
        &mut self.transform
    }
}
