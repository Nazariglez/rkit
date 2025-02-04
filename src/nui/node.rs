use taffy::Layout;

use super::{style::Style, CtxId, NuiContext, CACHE};
use crate::draw::*;

pub trait NuiWidget<T> {
    fn ui<'data>(self, ctx: &mut NuiContext<'data, '_, T>);

    fn add<'data>(self, ctx: &mut NuiContext<'data, '_, T>)
    where
        Self: Sized + 'data,
    {
        ctx.add_widget(self);
    }
}

pub struct Node<'ctx, 'data, 'arena, T>
where
    'data: 'arena,
{
    pub(super) temp_id: u64,
    pub(super) ctx: Option<&'ctx mut NuiContext<'data, 'arena, T>>,
    pub(super) style: Style,
}

impl<'ctx, 'data, 'arena, T> Node<'ctx, 'data, 'arena, T>
where
    'data: 'arena,
{
    pub fn new(ctx: &'ctx mut NuiContext<'data, 'arena, T>) -> Self {
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

    pub fn on_draw<F: for<'draw> FnMut(&'draw mut Draw2D, Layout, &T) + 'data>(
        mut self,
        cb: F,
    ) -> Self {
        if let Some(ctx) = &mut self.ctx {
            ctx.on_draw(self.temp_id, cb);
        }
        self
    }

    pub fn on_draw2<F: for<'draw> FnMut(&'draw mut Draw2D, Layout, &T) + 'data>(
        mut self,
        cb: F,
    ) -> Self {
        if let Some(ctx) = &mut self.ctx {
            ctx.on_draw(self.temp_id, cb);
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
