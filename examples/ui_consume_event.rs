use corelib::input::MouseButton;
use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Camera2D, Draw2D, Transform2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::ui::{
    UIControl, UIElement, UIEvents, UIGraph, UIHandler, UIInput, UIManager, UINodeMetadata,
    UIRawHandler,
};

pub struct Click;

#[derive(Default)]
struct State {
    cam: Camera2D,
    ui: UIManager<()>,
}

impl State {
    fn new() -> Self {
        let mut ui = UIManager::default();

        fn click_cb<S>(
            _: &Click,
            handler: &UIRawHandler,
            graph: &mut UIGraph<S>,
            state: &mut S,
            events: &mut UIEvents<S>,
        ) -> UIControl {
            let container = graph.element_mut_as::<Container>(handler.typed()).unwrap();
            container.clicks += 1;
            UIControl::Consume
        };

        // Parent container
        let parent_handler = ui.add(Container {
            fill: None,
            stroke: Some(Color::WHITE),
            clicks: 0,
            transform: Transform2D::builder()
                .set_anchor(Vec2::splat(0.5))
                .set_size(Vec2::splat(300.0))
                .set_translation(window_size() * 0.5)
                .into(),
        });

        ui.on(parent_handler, click_cb);

        // Child container
        let child_handler = ui
            .add_to(
                parent_handler,
                Container {
                    fill: Some(Color::ORANGE),
                    stroke: None,
                    clicks: 0,
                    transform: Transform2D::builder()
                        .set_translation(Vec2::splat(150.0))
                        .set_size(Vec2::splat(50.0))
                        .into(),
                },
            )
            .unwrap();

        ui.on(child_handler, click_cb);

        let cam = Camera2D::new(window_size(), Default::default());

        Self { cam, ui }
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
    transform: Transform2D,
}

impl<S> UIElement<S> for Container {
    fn transform(&self) -> &Transform2D {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform2D {
        &mut self.transform
    }

    fn input(
        &mut self,
        input: UIInput,
        state: &mut S,
        events: &mut UIEvents<S>,
        metadata: UINodeMetadata,
    ) {
        match input {
            UIInput::ButtonClick(MouseButton::Left) => {
                events.send(Click);
            }
            _ => {}
        }
    }

    fn render(&mut self, draw: &mut Draw2D, _state: &S, _meta: UINodeMetadata) {
        {
            let mut rect = draw.rect(Vec2::ZERO, self.transform.size());
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
