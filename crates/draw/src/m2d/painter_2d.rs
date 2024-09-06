use crate::create_pixel_pipeline;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use core::gfx::RenderPipeline;
use internment::Intern;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub(crate) static PAINTER_2D: Lazy<AtomicRefCell<Painter2D>> =
    Lazy::new(|| AtomicRefCell::new(Painter2D::default()));

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DrawPipeline {
    Pixel,
    Shapes,
    Custom(Intern<str>),
}

impl DrawPipeline {
    pub fn id(&self) -> &str {
        match self {
            DrawPipeline::Pixel => "gk_pixel",
            DrawPipeline::Shapes => "gk_shapes",
            DrawPipeline::Custom(inner) => inner,
        }
    }
}

impl From<&str> for DrawPipeline {
    fn from(value: &str) -> Self {
        Self::Custom(value.into())
    }
}

pub(crate) struct Painter2D {
    pipelines: HashMap<Intern<str>, RenderPipeline>,
}

impl Default for Painter2D {
    fn default() -> Self {
        // Initialize pipelines
        let mut painter = Self {
            pipelines: Default::default(),
        };

        painter.add_pipeline(DrawPipeline::Pixel.id(), create_pixel_pipeline().unwrap());

        painter
    }
}

impl Painter2D {
    pub fn add_pipeline(&mut self, id: &str, pip: RenderPipeline) -> Option<RenderPipeline> {
        self.pipelines.insert(id.into(), pip)
    }

    pub fn remove_pipeline(&mut self, id: &str) -> Option<RenderPipeline> {
        self.pipelines.remove(&id.into())
    }
}

pub(crate) fn get_2d_painter() -> AtomicRef<'static, Painter2D> {
    PAINTER_2D.borrow()
}

pub(crate) fn get_mut_2d_painter() -> AtomicRefMut<'static, Painter2D> {
    PAINTER_2D.borrow_mut()
}
