use crate::text::{get_mut_text_system, get_text_system, Font, TextInfo};
use crate::{Draw2D, Element2D};
use core::gfx::Color;
use core::math::{vec2, Mat3, Vec2};

pub struct Text2D<'a> {
    text: &'a str,
    font: Option<&'a Font>,
    position: Vec2,
    color: Color,
    alpha: f32,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    h_align: (),
    v_align: (),
    transform: Mat3,
}

impl<'a> Text2D<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            font: None,
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            size: 14.0,
            line_height: None,
            max_width: None,
            h_align: (),
            v_align: (),
            transform: Mat3::IDENTITY,
        }
    }

    pub fn font(&mut self, font: &'a Font) -> &mut Self {
        self.font = Some(font);
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.position = pos;
        self
    }

    pub fn size(&mut self, size: f32) -> &mut Self {
        self.size = size;
        self
    }

    pub fn line_height(&mut self, height: f32) -> &mut Self {
        self.line_height = Some(height);
        self
    }

    pub fn max_width(&mut self, width: f32) -> &mut Self {
        self.max_width = Some(width);
        self
    }

    pub fn h_align_left(&mut self) -> &mut Self {
        // self.h_align = HorizontalAlign::Left;
        self
    }

    pub fn h_align_center(&mut self) -> &mut Self {
        // self.h_align = HorizontalAlign::Center;
        self
    }

    pub fn h_align_right(&mut self) -> &mut Self {
        // self.h_align = HorizontalAlign::Right;
        self
    }

    pub fn v_align_top(&mut self) -> &mut Self {
        // self.v_align = VerticalAlign::Top;
        self
    }

    pub fn v_align_middle(&mut self) -> &mut Self {
        // self.v_align = VerticalAlign::Center;
        self
    }

    pub fn v_align_bottom(&mut self) -> &mut Self {
        // self.v_align = VerticalAlign::Bottom;
        self
    }
}

impl<'a> Element2D for Text2D<'a> {
    fn process(&self, draw: &mut Draw2D) {
        // TODO
        let info = TextInfo {
            pos: self.position,
            font: self.font.clone(),
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            scale: 1.0,
        };

        let c = self.color;

        let mut sys = get_mut_text_system();

        // clean temporal vectors
        sys.render_vertices.clear();
        sys.render_indices.clear();

        let block = sys.prepare_text(&info).unwrap();

        let block_size = block.size;
        let processed = block.data.iter().enumerate().map(|(i, data)| {
            let Vec2 { x: x1, y: y1 } = data.xy;
            let Vec2 { x: x2, y: y2 } = data.xy + data.size;
            let Vec2 { x: u1, y: v1 } = data.uvs_xy;
            let Vec2 { x: u2, y: v2 } = data.uvs_xy + data.size;

            #[rustfmt::skip]
            let vertices = [
                x1, y1, u1, v1, c.r, c.g, c.b, c.a,
                x2, y1, u2, v1, c.r, c.g, c.b, c.a,
                x1, y2, u1, v2, c.r, c.g, c.b, c.a,
                x2, y2, u2, v2, c.r, c.g, c.b, c.a,
            ];

            let n = (i * 4) as u32;
            #[rustfmt::skip]
            let indices = [
                n    , n + 1, n + 2,
                n + 2, n + 1, n + 3
            ];

            // sys.render_vertices.extend_from_slice(&vertices);
            // sys.render_indices.extend_from_slice(&indices);
        });
    }
}
