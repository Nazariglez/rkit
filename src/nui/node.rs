use corelib::math::vec2;
use corelib::math::Vec2;
use taffy::Layout;

use super::{style::Style, NuiContext};
use crate::draw::*;
use crate::random;

pub trait NuiNode {
    fn style(&self) -> Style {
        Style::default()
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout) {}

    fn add_with_children<D, F: FnOnce(&mut NuiContext<D>)>(self, ctx: &mut NuiContext<D>, cb: F)
    where
        Self: Sized + 'static,
    {
        ctx.add_node_with(self, cb);
    }

    fn add<D>(self, ctx: &mut NuiContext<D>)
    where
        Self: Sized + 'static,
    {
        ctx.add_node(self);
    }
}

pub trait NuiNodeWithData<T> {
    fn style(&self, data: &T) -> Style {
        Style::default()
    }
    fn render(&self, draw: &mut Draw2D, layout: Layout, data: &T) {}

    fn add_with_children<F: FnOnce(&mut NuiContext<T>)>(self, ctx: &mut NuiContext<T>, cb: F)
    where
        Self: Sized + 'static,
    {
        ctx.add_data_node_with(self, cb);
    }

    fn add(self, ctx: &mut NuiContext<T>)
    where
        Self: Sized + 'static,
    {
        ctx.add_data_node(self);
    }
}

pub enum NuiNodeType<'a, D> {
    Node(&'a dyn NuiNode),
    WithData(&'a dyn NuiNodeWithData<D>),
}

pub trait NuiWidget<T> {
    fn ui(self, ctx: &mut NuiContext<T>);

    fn add(self, ctx: &mut NuiContext<T>)
    where
        Self: Sized + 'static,
    {
        ctx.add_widget(self);
    }
}

pub struct Node<'a> {
    style: Style,
    render: Option<&'a dyn FnOnce(&mut Draw2D, Layout)>,
}

impl Default for Node<'_> {
    fn default() -> Self {
        Self {
            style: Default::default(),
            render: Default::default(),
        }
    }
}

impl<'a> NuiNode for Node<'a> {
    fn style(&self) -> Style {
        self.style
    }

    fn render(&self, draw: &mut Draw2D, layout: Layout) {
        let color: [f32; 3] = [random::gen(), random::gen(), random::gen()];
        draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
            .color(color.into());
    }
}

impl<'a> Node<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn on_render<F: FnOnce(&mut Draw2D, Layout)>(mut self, render: &'a F) -> Self {
        self.render = Some(render);
        self
    }
}
