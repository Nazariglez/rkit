use crate::shapes::TessMode;
use crate::{Draw2D, DrawPipelineId, Drawing, Element2D, Path2D, Transform2D};
use corelib::gfx::Color;
use corelib::math::{bvec2, vec2, Vec2};
use macros::Drawable2D;
use std::f32::consts::PI;

#[derive(Drawable2D)]
pub struct Star2D {
    color: Color,
    pos: Vec2,
    stroke_width: f32,
    alpha: f32,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    spikes: u8,
    outer_radius: f32,
    inner_radius: f32,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Star2D {
    pub fn new(spikes: u8, outer_radius: f32, inner_radius: f32) -> Self {
        Self {
            color: Color::WHITE,
            stroke_width: 1.0,
            pos: Vec2::splat(0.0),
            alpha: 1.0,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,
            spikes,
            outer_radius,
            inner_radius,

            pip: DrawPipelineId::Shapes,
            transform: None,
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

impl Element2D for Star2D {
    fn process(&self, draw: &mut Draw2D) {
        let mut path_builder = draw.path();
        path_builder.transform = self.transform;
        path_builder.pip = self.pip;
        draw_star(
            &mut path_builder,
            self.pos,
            self.spikes as _,
            self.outer_radius,
            self.inner_radius,
        );
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

fn draw_star(
    path_builder: &mut Drawing<Path2D>,
    center: Vec2,
    spikes: usize,
    outer_radius: f32,
    inner_radius: f32,
) {
    let step = PI / spikes as f32;

    let start_pos = center - vec2(0.0, outer_radius);
    path_builder.move_to(start_pos);

    let mut rot = PI / 2.0 * 3.0;
    for _ in 0..spikes {
        let pos = center + vec2(rot.cos(), rot.sin()) * outer_radius;
        rot += step;

        path_builder.line_to(pos);

        let pos = center + vec2(rot.cos(), rot.sin()) * inner_radius;
        rot += step;

        path_builder.line_to(pos);
    }

    path_builder.line_to(start_pos).close();
}
