use crate::math::Vec2;
use taffy::Layout;

use super::{ctx::NuiContext, style::Style};
use crate::draw::*;

pub trait NuiWidget<T> {
    fn ui(self, ctx: &mut NuiContext<'_, '_, T>);

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
    pub(super) use_inputs: bool,
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
            use_inputs: false,
        }
    }

    pub fn set_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn enable_input(mut self) -> Self {
        self.use_inputs = true;
        self
    }

    pub fn on_draw<F: for<'draw> Fn(&'draw mut Draw2D, Layout, &mut T) + 'data>(
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

    pub fn add_with_children<F: Fn(&mut NuiContext<'data, 'arena, T>)>(mut self, cb: F)
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

#[derive(Debug, Default)]
pub struct NodeState {
    pub(super) position: Vec2,
    pub(super) size: Vec2,
    pub(super) content_size: Vec2,
}

impl NodeState {
    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn content_size(&self) -> Vec2 {
        self.content_size
    }
}

#[derive(Debug, Default)]
pub struct NodeInput {
    pub(super) enabled: bool,
    pub(super) pointer_position: Vec2,
    pub(super) hover: bool,
    pub(super) just_enter: bool,
    pub(super) just_exit: bool,
}

// impl NodeInput {}
