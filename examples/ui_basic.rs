use corelib::math::Mat4;
use draw::Camera2D;
use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::time;
use rkit::ui::{UIElement, UIEventQueue, UIHandler, UIManager};

#[derive(Default)]
struct State {
    cam: Camera2D,
    ui: UIManager<()>,
    parent: UIHandler<Container>,
    child: UIHandler<Container>,
}

impl State {
    fn new() -> Self {
        let mut ui = UIManager::default();

        // Parent container
        let mut parent_transform = Transform2D::default();
        parent_transform
            .set_pivot(Vec2::splat(0.5))
            .set_anchor(Vec2::splat(0.5))
            .set_size(Vec2::splat(300.0))
            .set_translation(window_size() * 0.5);

        let parent_handler = ui.add(
            Container {
                fill: None,
                stroke: Some(Color::WHITE),
                clicks: 0,
            },
            parent_transform,
        );

        // Child container
        let mut child_transform = Transform2D::default();
        child_transform
            .set_translation(Vec2::splat(150.0))
            .set_size(Vec2::splat(50.0));

        let child_handler = ui
            .add_to(
                parent_handler,
                Container {
                    fill: Some(Color::ORANGE),
                    stroke: None,
                    clicks: 0,
                },
                child_transform,
            )
            .unwrap();

        let cam = Camera2D::new(window_size(), Default::default());

        Self {
            cam,
            ui,
            parent: parent_handler,
            child: child_handler,
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

    let child_color = state
        .ui
        .cursor_hover(state.child)
        .then_some(Color::GREEN)
        .unwrap_or(Color::ORANGE);
    state.ui.element_mut(state.child).unwrap().fill = Some(child_color);

    let parent_color = state
        .ui
        .cursor_hover(state.parent)
        .then_some(Color::GREEN)
        .unwrap_or(Color::WHITE);
    state.ui.element_mut(state.parent).unwrap().stroke = Some(parent_color);

    if state.ui.clicked(state.parent) {
        state.ui.element_mut(state.parent).unwrap().clicks += 1;
    }

    if state.ui.clicked(state.child) {
        state.ui.element_mut(state.child).unwrap().clicks += 1;
    }

    let mut draw = create_draw_2d();
    draw.set_camera(&state.cam);

    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    state.ui.render(&mut draw, &mut ());

    gfx::render_to_frame(&draw).unwrap();
}

struct Container {
    fill: Option<Color>,
    stroke: Option<Color>,
    clicks: usize,
}

impl<S> UIElement<S> for Container {
    fn update(
        &mut self,
        transform: &mut Transform2D,
        _state: &mut S,
        _events: &mut UIEventQueue<S>,
    ) {
        // transform.set_rotation(time::elapsed_f32().sin() * 2.0);
    }

    fn render(&mut self, transform: &Transform2D, draw: &mut Draw2D, state: &S) {
        {
            let mut rect = draw.rect(Vec2::ZERO, transform.size());
            if let Some(fill) = self.fill {
                rect.fill_color(fill).fill();
            }
            if let Some(stroke) = self.stroke {
                rect.stroke_color(stroke).stroke(2.0);
            }
        }

        draw.text(&self.clicks.to_string())
            .position(Vec2::splat(3.0))
            .size(20.0);
    }
}
