use crate::shapes::TessMode;
use crate::{Draw2D, Drawing, Element2D, Path2D};
use core::gfx::Color;
use core::math::{vec2, Mat3, Vec2};
use std::f32::consts::PI;

pub struct Polygon2D {
    color: Color,
    pos: Vec2,
    stroke_width: f32,
    alpha: f32,
    transform: Mat3,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    sides: u8,
    radius: f32,
}

impl Polygon2D {
    pub fn new(sides: u8, radius: f32) -> Self {
        Self {
            color: Color::WHITE,
            stroke_width: 1.0,
            pos: Vec2::splat(0.0),
            alpha: 1.0,
            transform: Mat3::IDENTITY,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,
            sides,
            radius,
        }
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn fill_color(&mut self, color: Color) -> &mut Self {
        self.fill_color = Some(color);
        self
    }

    pub fn stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = Some(color);
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

    pub fn fill(&mut self) -> &mut Self {
        self.modes[self.mode_index] = Some(TessMode::Fill);
        self.mode_index = (self.mode_index + 1) % 2;
        self
    }

    pub fn stroke(&mut self, width: f32) -> &mut Self {
        self.modes[self.mode_index] = Some(TessMode::Stroke);
        self.stroke_width = width;
        self.mode_index = (self.mode_index + 1) % 2;
        self
    }
}

impl Element2D for Polygon2D {
    fn process(&self, draw: &mut Draw2D) {
        let mut path_builder = draw.path();
        draw_polygon(&mut path_builder, self.pos, self.sides as _, self.radius);
        path_builder.color(self.color).alpha(self.alpha);

        let first_mode = self.modes[0].unwrap_or(TessMode::Fill);
        match first_mode {
            TessMode::Fill => {
                if let Some(c) = self.fill_color {
                    path_builder.fill_color(c);
                }
                path_builder.fill();
            }
            TessMode::Stroke => {
                if let Some(c) = self.stroke_color {
                    path_builder.stroke_color(c);
                }
                path_builder.stroke(self.stroke_width);
            }
        }

        if let Some(mode) = self.modes[1] {
            match mode {
                TessMode::Fill => {
                    if let Some(c) = self.fill_color {
                        path_builder.fill_color(c);
                    }
                    path_builder.fill();
                }
                TessMode::Stroke => {
                    if let Some(c) = self.stroke_color {
                        path_builder.stroke_color(c);
                    }
                    path_builder.stroke(self.stroke_width);
                }
            }
        }
    }
}

fn draw_polygon(path_builder: &mut Drawing<Path2D>, center: Vec2, sides: usize, radius: f32) {
    for n in 0..sides {
        let i = n as f32;

        let pi_sides = PI / sides as f32;
        let is_even = sides % 2 == 0;
        let offset = if is_even { pi_sides } else { pi_sides * 0.5 };

        let angle = i * 2.0 * pi_sides - offset;
        let pos = center + radius * vec2(angle.cos(), angle.sin());

        if n == 0 {
            path_builder.move_to(pos);
        } else {
            path_builder.line_to(pos);
        }
    }

    path_builder.close();
}