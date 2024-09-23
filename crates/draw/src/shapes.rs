use core::gfx::Color;
use lyon::path::Path;
use lyon::tessellation::*;
use std::cell::RefCell;

thread_local! {
    pub(crate) static SHAPE_TESSELLATOR:RefCell<ShapeTessellator> = RefCell::new(ShapeTessellator::new());
}

pub(crate) struct ShapeTessellator {
    stroke: StrokeTessellator,
    fill: FillTessellator,
}

impl ShapeTessellator {
    pub fn new() -> Self {
        Self {
            stroke: StrokeTessellator::new(),
            fill: FillTessellator::new(),
        }
    }

    pub(crate) fn fill_lyon_path(
        &mut self,
        path: &Path,
        color: Color,
        options: &FillOptions,
    ) -> (Vec<f32>, Vec<u32>) {
        let mut geometry: VertexBuffers<[f32; 6], u32> = VertexBuffers::new();
        self.fill
            .tessellate_path(
                path,
                options,
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                    let [x, y] = vertex.position().to_array();
                    [x, y, color.r, color.g, color.b, color.a]
                }),
            )
            .unwrap();

        (geometry.vertices.concat(), geometry.indices)
    }

    pub(crate) fn stroke_lyon_path(
        &mut self,
        path: &Path,
        color: Color,
        options: &StrokeOptions,
    ) -> (Vec<f32>, Vec<u32>) {
        let mut geometry: VertexBuffers<[f32; 6], u32> = VertexBuffers::new();
        self.stroke
            .tessellate_path(
                path,
                options,
                &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                    let [x, y] = vertex.position().to_array();
                    [x, y, color.r, color.g, color.b, color.a]
                }),
            )
            .unwrap();

        (geometry.vertices.concat(), geometry.indices)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TessMode {
    Fill,
    Stroke,
}
