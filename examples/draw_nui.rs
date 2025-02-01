use corelib::math::{vec2, Vec2};
use corelib::time;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::nui::prelude::*;
use rkit::nui::{self, *};
use std::time::Duration;

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
    draw.ui()
        // .origin(Vec2::splat(0.5))
        // .flip_x(true)
        // .scale(Vec2::splat(0.5))
        .show(|ctx| {
            // for _ in 0..200 {
            Container.add(ctx);
            // }
            nodes = ctx.len();
        });

    gfx::render_to_frame(&draw).unwrap();
    state.measure += now.elapsed();
    state.time += time::delta_f32();
    state.n += 1;

    if state.time > 5.0 {
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
        Node::new()
            .on_render(&|d, l| println!("whatever"))
            .set_style(
                Style::default()
                    .flex_row()
                    .size(px(800.0), px(600.0))
                    .align_items_center()
                    .justify_content_center()
                    .gap_x(px(20.0)),
            )
            .add_with_children(ctx, |ctx| {
                // first column
                Node::new()
                    .set_style(
                        Style::default()
                            .size(px(100.0), px(100.0))
                            .align_items_center()
                            .justify_content_center(),
                    )
                    .add_with_children(ctx, |ctx| {
                        // column content
                        Node::new()
                            .set_style(Style::default().size(px(40.0), px(20.0)))
                            .add(ctx);
                    });

                // second column
                Node::new()
                    .set_style(Style::default().size(px(100.0), px(100.0)))
                    .add(ctx);
            });
    }
}
