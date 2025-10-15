use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::{is_mouse_scrolling, mouse_wheel_delta}, math::{Vec2, vec2}};

#[derive(Resource)]
struct MousePos(Vec2);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    cmds.insert_resource(MousePos(vec2(400.0, 300.0)));
}

fn update_system(mut pos: ResMut<MousePos>, window: Res<Window>) {
    if is_mouse_scrolling() {
        let delta = mouse_wheel_delta();
        pos.0 = (pos.0 + delta).min(window.size()).max(Vec2::ZERO);
    }
}

fn draw_system(pos: Res<MousePos>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Scroll with your mouse's wheel or touchpad")
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5));

    draw.circle(30.0)
        .position(pos.0)
        .anchor(Vec2::splat(0.5))
        .color(Color::RED);

    gfx::render_to_frame(&draw).unwrap();
}
