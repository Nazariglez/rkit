// This example is a port of javascript code made by Ibon Tolosona (@hyperandroid)
// https://codepen.io/hyperandroid/full/yLyRQmw

use draw::{Drawing, Path2D};
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;
use rkit::time;

const CENTER_X: f32 = 400.0;
const CENTER_Y: f32 = 300.0;
const START_RADIUS: usize = 20;
const RADIUS_INCREMENT: usize = 13;
const MAX_RADIUS: usize = 250;
const MAX_LINES: usize = (MAX_RADIUS - START_RADIUS) / RADIUS_INCREMENT;
const AMPLITUDE: f32 = 30.0;
const PERIOD: f32 = 6.0;
const PI: f32 = std::f32::consts::PI;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let t = time::elapsed_f32() * 1000.0;

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let mut count = 0.0;
    for (line_index, i) in (START_RADIUS..MAX_RADIUS)
        .step_by(RADIUS_INCREMENT as _)
        .enumerate()
    {
        let ti = ((t + line_index as f32 * 79.0) % 2000.0) / 2000.0;
        draw_flower(
            draw.path(),
            i as _,
            ((t % 38000.0) / 38000.0) * 2.0 * PI,
            count,
            (ti * PI * 2.0).cos(),
        );
        count += if line_index <= MAX_LINES / 2 {
            2.0
        } else {
            -2.0
        };
    }

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_flower(
    mut path_builder: Drawing<Path2D>,
    radius: f32,
    initial_angle: f32,
    index: f32,
    amplitude_modifier: f32,
) {
    let segments = (2.0 * PI * radius).floor();
    let mut begin = false;

    for i in 0..segments as usize {
        let n = i as f32;

        let period_segments = segments / PERIOD;
        let current_periods = n % period_segments;

        let radians_period = if current_periods < radius {
            current_periods / radius
        } else {
            0.0
        };

        let c_radius = radius
            + AMPLITUDE
                * (radians_period * (3.0 + index) * PI).sin()
                * ((radians_period * PI).sin() / 2.0 * amplitude_modifier);

        let radians = n / segments * 2.0 * PI + initial_angle;
        let pos = vec2(
            CENTER_X + c_radius * radians.cos(),
            CENTER_Y + c_radius * radians.sin(),
        );

        if !begin {
            path_builder.move_to(pos);
            begin = true;
        }

        path_builder.line_to(pos);
    }

    path_builder.close().color(Color::MAGENTA).stroke(3.0);
}
