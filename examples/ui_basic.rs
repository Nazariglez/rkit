use draw::Camera2D;
use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
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
        let parent_handler = ui.add(
            Container {
                fill: None,
                stroke: Some(Color::WHITE),
                clicks: 0,
            },
            Transform2D::builder()
                .set_size(Vec2::splat(300.0))
                .set_translation(window_size() * 0.5)
                .into(),
        );

        // Child container
        let child_handler = ui
            .add_to(
                parent_handler,
                Container {
                    fill: Some(Color::ORANGE),
                    stroke: None,
                    clicks: 0,
                },
                Transform2D::builder()
                    .set_translation(Vec2::splat(150.0))
                    .set_size(Vec2::splat(50.0))
                    .into(),
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
    // update camera
    state.cam.set_size(window_size());
    state.cam.set_position(window_size() * 0.5);
    state.cam.update();

    // update ui manager
    state.ui.update(&state.cam, &mut ());

    // color elements on hover
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

    // add clicks to the element (this can be done via events to, check ui_events)
    if state.ui.clicked(state.parent) {
        state.ui.element_mut(state.parent).unwrap().clicks += 1;
    }

    if state.ui.clicked(state.child) {
        state.ui.element_mut(state.child).unwrap().clicks += 1;
    }

    // draw as usual
    let mut draw = create_draw_2d();
    draw.set_camera(&state.cam);
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    // draw ui manager elements
    state.ui.render(&mut draw, &mut ());

    gfx::render_to_frame(&draw).unwrap();
}

struct Container {
    fill: Option<Color>,
    stroke: Option<Color>,
    clicks: usize,
}

impl<S> UIElement<S> for Container {
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
