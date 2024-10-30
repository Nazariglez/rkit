mod sys;

use crate::filters::sys::SYS;
use crate::gfx::{AsRenderer, RenderTexture};

pub trait Filter {}

pub struct PostProcess<'a, R>
where
    R: AsRenderer,
{
    pub filters: &'a [&'a dyn Filter],
    pub render: &'a R,
    pub pixelated: bool,
}

impl<'a, R> AsRenderer for PostProcess<'a, R>
where
    R: AsRenderer,
{
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let mut sys = SYS.borrow_mut();
        sys.process(self, target)
    }
}
