use draw::Draw2D;
use rkit::app::window_size;
use rkit::draw::{Transform2D, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};
use rkit::time;

// Hierarchy
//  - A
//    - B
//      - C
//    - D
//    - E

struct Container {
    transform: Transform2D,
    color: Color,
}

impl Container {
    fn new(size: Vec2, color: Color) -> Self {
        let mut transform = Transform2D::new();
        transform.set_size(size);

        Self { transform, color }
    }
}

struct State {
    count: f32,
    container_a: Container,
    container_b: Container,
    container_c: Container,
    container_d: Container,
    container_e: Container,
}

impl State {
    fn new() -> Self {
        let container_a = Container::new(vec2(500.0, 500.0), Color::WHITE);
        let container_b = Container::new(vec2(230.0, 350.0), Color::rgb(1.0, 1.0, 0.5));
        let container_c = Container::new(vec2(130.0, 130.0), Color::rgb(0.5, 1.0, 0.5));
        let container_d = Container::new(vec2(130.0, 130.0), Color::rgb(1.0, 0.5, 1.0));
        let container_e = Container::new(vec2(130.0, 130.0), Color::rgb(0.5, 1.0, 1.0));

        Self {
            count: 0.0,
            container_a,
            container_b,
            container_c,
            container_d,
            container_e,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).update(update).run()
}

fn update(state: &mut State) {
    let offset = state.count.sin();
    let center = window_size() * 0.5;

    // Container A, relative to the screen
    state.container_a.transform.set_translation(
        center - state.container_a.transform.size() * 0.5 + (offset * 120.0 * Vec2::X),
    );

    // Container B, relative to Container A
    state.container_b.transform.set_translation(vec2(
        20.0,
        state.container_a.transform.size().y * 0.5 - state.container_b.transform.size().y * 0.5
            + offset * 50.0,
    ));

    // Container C, relative to Container B
    state.container_c.transform.set_translation(
        state.container_b.transform.size() * 0.5
            - state.container_c.transform.size() * 0.5
            - offset * vec2(32.0, 90.0),
    );

    // Container D, relative to Container A
    state.container_d.transform.set_translation(
        state.container_a.transform.size() * vec2(0.75, 0.25)
            - state.container_d.transform.size() * 0.5,
    );
    state.container_d.transform.set_rotation(offset * 1.0);

    // Container E, relative to Container A
    state.container_e.transform.set_translation(
        state.container_a.transform.size() * 0.75 - state.container_e.transform.size() * 0.5,
    );
    state
        .container_e
        .transform
        .set_scale(vec2(1.0 - offset * 0.3, 1.0 + offset * 0.3));

    state.count += time::delta_f32();

    draw(state);
}

fn draw(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Draw Container A as root
    draw_container(&mut draw, &mut state.container_a, |draw, _c| {
        // B is a child of A
        draw_container(draw, &mut state.container_b, |draw, _c| {
            // C is a child of B
            draw_container(draw, &mut state.container_c, |draw, c| {
                draw_triangle(draw, c.transform.size(), state.count.cos());
            });
        });

        // D is a child of A
        draw_container(draw, &mut state.container_d, |draw, c| {
            draw_triangle(draw, c.transform.size(), state.count.cos());
        });

        // E is a child of A
        draw_container(draw, &mut state.container_e, |draw, c| {
            draw_triangle(draw, c.transform.size(), state.count.cos());
        });
    });

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_container<F: FnMut(&mut Draw2D, &mut Container)>(
    draw: &mut Draw2D,
    container: &mut Container,
    mut f: F,
) {
    // Push the matrix to the stack
    draw.push_matrix(container.transform.updated_mat3());

    draw.rect(Vec2::ZERO, container.transform.size())
        .color(container.color)
        .stroke(5.0);

    // Anything draw after the push will behave as a "children" of this container
    f(draw, container);

    // After we finish drawing this container we pop the matrix from the stack
    draw.pop_matrix();
}

// Draw a triangle with a local matrix
fn draw_triangle(draw: &mut Draw2D, size: Vec2, offset: f32) {
    let a = vec2(size.x * 0.3, size.y * 0.0);
    let b = vec2(size.x * 0.0, size.y * 0.6);
    let c = vec2(size.x * 0.6, size.y * 0.6);
    let pos = size * 0.5;
    let rotation = offset * 360.0 * 0.5;

    draw.triangle(a, b, c)
        .alpha(0.7)
        .translate(pos)
        .anchor(Vec2::splat(0.5))
        .pivot(Vec2::splat(0.5))
        .rotation(rotation.to_radians())
        .color(Color::MAGENTA);
}
