use crate::input::MouseButton;
use crate::math::Vec2;
use std::collections::VecDeque;

#[derive(Copy, Clone, Debug)]
pub enum Event {
    MouseMove { pos: Vec2, delta: Vec2 },
    MouseUp { btn: MouseButton },
    MouseDown { btn: MouseButton },
    MouseWheel { delta: Vec2 },
    MouseEnter,
    MouseLeave,
}

/// Event iterator queue
#[derive(Debug, Clone, Default)]
pub struct EventIterator(VecDeque<Event>);

impl EventIterator {
    pub fn new() -> Self {
        Default::default()
    }

    /// Remove and return the first element on the queue
    pub fn pop_front(&mut self) -> Option<Event> {
        self.0.pop_front()
    }

    /// Add an event at the end of the list
    pub fn push(&mut self, evt: Event) {
        self.0.push_back(evt);
    }

    /// Add an event at the beginning of the list
    pub fn push_front(&mut self, evt: Event) {
        self.0.push_front(evt);
    }

    /// Return the events and clear the list
    pub fn take_events(&mut self) -> EventIterator {
        EventIterator(std::mem::take(&mut self.0))
    }
}

impl Iterator for EventIterator {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_front()
    }
}
