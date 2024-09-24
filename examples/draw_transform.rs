use draw::Transform2D;
use rkit::app::window_size;
use rkit::draw::draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};
use rkit::time;
use std::ops::Rem;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = draw_2d();
    draw.clear(Color::BLACK);

    let mut transform = Transform2D::new();
    transform.set_position(window_size() * 0.5);
    transform.set_size(vec2(600.0, 400.0));
    transform.set_anchor(Vec2::splat(0.5));
    transform.set_pivot(Vec2::splat(0.0));
    transform.set_scale(Vec2::splat(0.5));
    transform.set_rotation(time::elapsed_f32().rem(360.0).to_radians() * 10.0);

    draw.push_matrix(transform.as_mat3());

    draw.rect(vec2(0.0, 0.0), vec2(600.0, 400.0));

    draw.pop_matrix();
    gfx::render_to_frame(&draw).unwrap();
}
