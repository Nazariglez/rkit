use taffy::Layout;

use super::{style::Style, NuiContext};
use crate::draw::*;

pub trait NuiWidget<T> {
    fn ui<'data>(self, ctx: &mut NuiContext<'data, T>);

    fn add<'data>(self, ctx: &mut NuiContext<'data, T>)
    where
        Self: Sized + 'data,
    {
        ctx.add_widget(self);
    }
}

pub struct Node<'ctx, 'data, T> {
    pub(super) temp_id: u64,
    pub(super) ctx: Option<&'ctx mut NuiContext<'data, T>>,
    pub(super) style: Style,
}

impl<'ctx, 'data, T> Node<'ctx, 'data, T> {
    pub fn new(ctx: &'ctx mut NuiContext<'data, T>) -> Self {
        ctx.temp_id += 1;
        Self {
            temp_id: ctx.temp_id,
            ctx: Some(ctx),
            style: Style::default(),
        }
    }

    pub fn set_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    // pub fn on_render<F: FnOnce(&mut Draw2D, Layout) + 'static>(mut self, cb: F) -> Self {
    //     if let Some(ctx) = &mut self.ctx {
    //         ctx.on_render(self.temp_id, cb);
    //     }
    //     self
    // }

    pub fn on_draw<F: for<'draw> FnMut(&'draw mut Draw2D, Layout, &T) + 'data>(
        mut self,
        cb: F,
    ) -> Self {
        if let Some(ctx) = &mut self.ctx {
            ctx.on_render(self.temp_id, cb);
        }
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
    }

    pub fn add_with_children<F: FnMut(&mut NuiContext<T>)>(mut self, cb: F)
    where
        Self: Sized,
    {
        let ctx = self.ctx.take().unwrap();
        ctx.add_node_with(self, cb);
    }

    pub fn add(mut self)
    where
        Self: Sized,
    {
        let ctx = self.ctx.take().unwrap();
        ctx.add_node(self);
    }
}
