use taffy::Layout;

use super::{style::Style, NuiContext};
use crate::draw::*;

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
