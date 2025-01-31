use std::time::Duration;
use corelib::math::vec2;
use corelib::time;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::nui::{self, *};

#[derive(Default)]
struct State {
    time: f32,
    measure: Duration,
    n: u32,
}

fn main() -> Result<(), String> {
    rkit::init_with(State::default).update(update).run()
}

#[inline]
#[track_caller]
fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    let mut nodes = 0;
    let now = time::now();
    draw.ui().show(|ctx| {
       for _ in 0..800 {
           Container.add(ctx);
       }
        nodes = ctx.len();
    });

    gfx::render_to_frame(&draw).unwrap();
    state.measure += now.elapsed();
    state.time += time::delta_f32();
    state.n += 1;

    if state.time > 1.0 {
        let avg = state.measure / state.n;
        log::warn!("avg: {avg:?} -> nodes: {nodes}");

        state.measure = Duration::ZERO;
        state.time = 0.0;
        state.n = 0;
    }

}
pub struct Container;
impl<T> NuiWidget<T> for Container {
    fn ui(self, ctx: &mut NuiContext<T>) {
        // Root
        nui::Node::new("root")
            .left_right()
            .size(ctx.size())
            .color(Color::ORANGE)
            .content_horizontal_center()
            .content_gap(vec2(20.0, 0.0))
            .add_with_children(ctx, |ctx| {
                // first column
                nui::Node::new("child1")
                    .size(vec2(100.0, 100.0))
                    .color(Color::RED)
                    .content_horizontal_center()
                    .add_with_children(ctx, |ctx| {
                        // column content
                        nui::Node::new("inner_child")
                            .color(Color::GREEN)
                            .size(vec2(20.0, 40.0))
                            .add(ctx);
                    });

                // second column
                nui::Node::new("child2")
                    .size(vec2(100.0, 100.0))
                    .color(Color::BLUE)
                    .add(ctx);
            });
    }
}
