use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Camera2D, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::ui::{UIControl, UIElement, UIEvents, UIInput, UIManager, UINodeMetadata};

struct State {
    cam: Camera2D,
    ui: UIManager<()>,
}

impl State {
    fn new() -> Self {
        let mut ui = UIManager::default();

        // Create our element
        let _ = ui.add(DraggableNode {
            transform: Transform2D::builder()
                .set_anchor(Vec2::splat(0.5))
                .set_size(Vec2::splat(300.0))
                .set_translation(window_size() * 0.5)
                .into(),
            ..Default::default()
        });

        let cam = Camera2D::new(window_size(), Default::default());
        Self { cam, ui }
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
struct DraggableNode {
    dragging: bool,
    transform: Transform2D,
}

impl<S> UIElement<S> for DraggableNode {
    fn input_enabled(&self) -> bool {
        true
    }
    fn transform(&self) -> &Transform2D {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform2D {
        &mut self.transform
    }

    fn input(
        &mut self,
        input: UIInput,
        _state: &mut S,
        _events: &mut UIEvents<S>,
        _meta: UINodeMetadata,
    ) -> UIControl {
        match input {
            UIInput::DragStart { .. } => {
                self.dragging = true;
            }
            UIInput::Dragging { frame_delta, .. } => {
                let node_pos = self.transform.position();
                self.transform.set_translation(node_pos + frame_delta);
            }
            UIInput::DragEnd { .. } => {
                self.dragging = false;
            }
            _ => {}
        }

        UIControl::Consume
    }

    fn render(&mut self, draw: &mut Draw2D, _state: &S, _meta: UINodeMetadata) {
        let size = self.transform.size();
        let (txt, color) = if self.dragging {
            ("Dragging...", Color::GREEN)
        } else {
            ("Drag Me!", Color::SILVER)
        };

        draw.rect(Vec2::ZERO, size).color(color);
        draw.text(txt)
            .anchor(Vec2::splat(0.5))
            .translate(size * 0.5)
            .color(Color::BLACK)
            .size(24.0)
            .max_width(size.x * 0.9)
            .h_align_center();
    }
}
