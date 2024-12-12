use crate::ui::events::UIEventQueue;
use downcast_rs::{impl_downcast, Downcast};
use draw::{Draw2D, Transform2D};

pub trait UIElement<S>: Downcast {
    // TODO: event?
    // fn init(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {}
    fn update(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {
    }
    fn render(&mut self, transform: &Transform2D, draw: &mut Draw2D, state: &S) {}
    // fn clean(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {}
}

impl_downcast!(UIElement<S>);

pub struct UIRoot;
impl<S> UIElement<S> for UIRoot {}
