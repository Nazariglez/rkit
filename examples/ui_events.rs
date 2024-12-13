use corelib::input::MouseButton;
use corelib::math::{vec2, FloatExt};
use corelib::time;
use draw::Camera2D;
use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::ui::{UIElement, UIEventQueue, UIHandler, UIManager};

struct MoveLeft;
struct MoveRight;
struct Stopped;

struct State {
    cam: Camera2D,
    ui: UIManager<()>,
    container: UIHandler<Element>,
}

impl State {
    fn new() -> Self {
        let mut ui = UIManager::default();

        let mut container = Transform2D::default();
        container
            .set_pivot(Vec2::splat(0.5))
            .set_anchor(Vec2::splat(0.5))
            .set_size(Vec2::splat(300.0))
            .set_translation(window_size() * 0.5);

        let container_handler = ui.add(
            Element {
                moving: false,
                target: 0.0,
                color: Color::SILVER,
            },
            container,
        );

        // move to the left
        let _ = ui.on(container_handler, |evt: &MoveLeft, data| {
            data.node.moving = true;
            data.node.target = 200.0;
        });

        // move to the right
        let _ = ui.on(container_handler, |evt: &MoveRight, data| {
            data.node.moving = true;
            data.node.target = 600.0;
        });

        // change color when stopped
        let _ = ui.on(container_handler, |evt: &Stopped, data| {
            data.node.color = Color::PINK;
        });

        // buttons
        let mut child_transform = Transform2D::default();
        child_transform
            .set_translation(Vec2::splat(150.0))
            .set_size(Vec2::splat(50.0));

        let cam = Camera2D::new(window_size(), Default::default());

        Self {
            cam,
            ui,
            container: container_handler,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).update(update).run()
}

fn update(state: &mut State) {
    state.cam.set_size(window_size());
    state.cam.set_position(window_size() * 0.5);
    state.cam.update();

    state.ui.update(&state.cam, &mut ());

    if state.ui.clicked(state.container) {
        state.ui.push_event(MoveLeft);
    }

    if state.ui.clicked_by(state.container, MouseButton::Right) {
        state.ui.push_event(MoveRight);
    }

    let mut draw = create_draw_2d();
    draw.set_camera(&state.cam);
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    state.ui.render(&mut draw, &mut ());

    gfx::render_to_frame(&draw).unwrap();
}

struct Element {
    moving: bool,
    target: f32,
    color: Color,
}

impl<S> UIElement<S> for Element {
    fn update(&mut self, transform: &mut Transform2D, state: &mut S, events: &mut UIEventQueue<S>) {
        if !self.moving {
            return;
        }

        self.color = Color::SILVER;
        let pos = transform.position();
        let offset = pos.x.lerp(self.target, time::delta_f32() * 4.0);
        transform.set_translation(vec2(offset, pos.y));

        let distance = (transform.position().x - self.target).abs();
        if distance < 1.0 {
            self.moving = false;
            events.send(Stopped);
        }
    }

    fn render(&mut self, transform: &Transform2D, draw: &mut Draw2D, state: &S) {
        draw.rect(Vec2::ZERO, transform.size()).color(self.color);

        draw.text("Left click to move to the left.\n\nRight click to move to the right")
            .anchor(Vec2::splat(0.5))
            .translate(transform.size() * 0.5)
            .color(Color::BLACK)
            .size(22.0)
            .max_width(transform.size().x * 0.9)
            .h_align_center();
    }
}
