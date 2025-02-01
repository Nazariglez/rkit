use taffy::Layout;

use super::{style::Style, NuiContext};
use crate::draw::*;

pub trait NuiWidget<T> {
    fn ui<'layout, 'ctx>(self, ctx: &'layout mut NuiContext<'layout, 'ctx, T>);

    fn add<'layout, 'ctx>(self, ctx: &'layout mut NuiContext<'layout, 'ctx, T>)
    where
        Self: Sized + 'ctx,
    {
        ctx.add_widget(self);
    }
}

pub struct Node<'layout, 'ctx, T> {
    pub(super) temp_id: u64,
    pub(super) ctx: Option<&'ctx mut NuiContext<'layout, 'ctx, T>>,
    pub(super) style: Style,
}

impl<'layout, 'ctx, T> Node<'layout, 'ctx, T> {
    pub fn new(ctx: &'layout mut NuiContext<'layout, 'ctx, T>) -> Self {
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

    pub fn on_render<F: FnOnce(&mut Draw2D, Layout) + 'ctx>(mut self, cb: F) -> Self {
        if let Some(ctx) = &mut self.ctx {
            ctx.on_render(self.temp_id, cb);
        }
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
    }

    pub fn add_with_children<F: FnOnce(&mut NuiContext<T>)>(mut self, cb: F)
    where
        Self: Sized + 'ctx,
    {
        let ctx = self.ctx.take().unwrap();
        ctx.add_node_with(self, cb);
    }

    pub fn add(mut self)
    where
        Self: Sized + 'ctx,
    {
        let ctx = self.ctx.take().unwrap();
        ctx.add_node(self);
    }
}
