use crate::app::window_size;
use crate::gfx::{AsRenderer, RenderTexture};

pub trait Filter {}

pub struct PostProcess<'a, R>
where
    R: AsRenderer,
{
    filters: &'a [&'a dyn Filter],
    render: &'a R,
}

impl<'a, R> AsRenderer for PostProcess<'a, R>
where
    R: AsRenderer,
{
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let size = target.map(|rt| rt.size()).unwrap_or_else(|| window_size());
        todo!()
    }
}
