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
    // fn transform(&mut self) -> &mut Transform2D;
    // fn init(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn input(&mut self, input: UIInput, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn update(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {
    }
    fn render(&mut self, transform: &Transform2D, draw: &mut Draw2D, state: &S) {}
    // fn clean(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {}
}

impl_downcast!(UIElement<S>);

pub struct UIRoot;
impl<S> UIElement<S> for UIRoot {}
