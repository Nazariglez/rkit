use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Camera2D, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::input::MouseButton;
use rkit::math::{vec2, FloatExt, Vec2};
use rkit::time;
use rkit::ui::{UIElement, UIEvents, UIHandler, UIManager, UINodeMetadata};

// events
struct MoveTo(f32);
struct Stop;

struct State {
    cam: Camera2D,
    ui: UIManager<()>,
    container: UIHandler<Element>,
}

impl State {
    fn new() -> Self {
        let mut ui = UIManager::default();

        // Create our element
        let container = ui.add(Element {
            transform: Transform2D::builder()
                .set_anchor(Vec2::splat(0.5))
                .set_size(Vec2::splat(300.0))
                .set_translation(window_size() * 0.5)
                .into(),
            ..Default::default()
        });

        // -- Define element event listeners
        // move event
        let _listener = ui.on(
            container,
            |&MoveTo(pos), handler, graph, _state, _events| {
                if let Some(node) = graph.element_mut_as::<Element>(handler.typed()) {
                    node.moving = true;
                    node.target = pos;
                    node.color = Color::SILVER;
                }
            },
        );

        // change color when stopped
        let _listener = ui.on(container, |_evt: &Stop, handler, graph, _state, _events| {
            if let Some(node) = graph.element_mut_as::<Element>(handler.typed()) {
                node.moving = false;
                node.color = Color::PINK;
            }
        });

        let cam = Camera2D::new(window_size(), Default::default());
        Self { cam, ui, container }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).update(update).run()
}

fn update(state: &mut State) {
    // Update camera parameters
    state.cam.set_size(window_size());
    state.cam.set_position(window_size() * 0.5);
    state.cam.update();

    // update UI Manager
    state.ui.update(&state.cam, &mut ());

    // left click move to the left
    if state.ui.clicked(state.container) {
        state.ui.send_event(MoveTo(200.0));
    }

    // right click move to the right
    if state.ui.clicked_by(state.container, MouseButton::Right) {
        state.ui.send_event(MoveTo(600.0));
    }

    // just draw as usual
    let mut draw = create_draw_2d();
    draw.set_camera(&state.cam);
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    // this will draw the UIManager elements
    state.ui.render(&mut draw, &mut ());

    // draw to screen as usual
    gfx::render_to_frame(&draw).unwrap();
}

// Widget
#[derive(Default)]
struct Element {
    moving: bool,
    target: f32,
    color: Color,
    transform: Transform2D,
}

impl<S> UIElement<S> for Element {
    fn transform(&self) -> &Transform2D {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform2D {
        &mut self.transform
    }

    fn update(&mut self, _state: &mut S, events: &mut UIEvents<S>, _meta: UINodeMetadata) {
        if !self.moving {
            return;
        }

        let pos = self.transform.position();
        let offset = pos.x.lerp(self.target, time::delta_f32() * 4.0);
        self.transform.set_translation(vec2(offset, pos.y));

        let distance = (self.transform.position().x - self.target).abs();
        if distance < 1.0 {
            events.send(Stop);
        }
    }

    fn render(&mut self, draw: &mut Draw2D, _state: &S, _meta: UINodeMetadata) {
        let size = self.transform.size();
        draw.rect(Vec2::ZERO, size).color(self.color);

        draw.text("Left click to move to the left.\n\nRight click to move to the right")
            .anchor(Vec2::splat(0.5))
            .translate(size * 0.5)
            .color(Color::BLACK)
            .size(22.0)
            .max_width(size.x * 0.9)
            .h_align_center();
    }
}
