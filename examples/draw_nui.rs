use corelib::math::vec2;
use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::nui::{self, *};

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

#[inline]
#[track_caller]
fn update() {
    let mut draw = create_draw_2d();
    // draw.ui(|ctx| Container.add_to_ctx(ctx));
    NuiLayout::new(&mut draw).show(|ctx| Container.add_to_ctx(ctx));
    gfx::render_to_frame(&draw).unwrap();
}
pub struct Container;
impl NuiWidget for Container {
    fn ui(&self, ctx: &mut NuiContext) {
        // Root
        nui::Node::new("root")
            .left_right()
            .size(ctx.size())
            .color(Color::ORANGE)
            .content_horizontal_center()
            .content_gap(vec2(20.0, 0.0))
            .add_to_ctx_with(ctx, |ctx| {
                // first column
                nui::Node::new("child1")
                    .size(vec2(100.0, 100.0))
                    .color(Color::RED)
                    .content_horizontal_center()
                    .add_to_ctx_with(ctx, |ctx| {
                        // column content
                        nui::Node::new("inner_child")
                            .color(Color::GREEN)
                            .size(vec2(20.0, 40.0))
                            .add_to_ctx(ctx);
                    });

                // second column
                nui::Node::new("child2")
                    .size(vec2(100.0, 100.0))
                    .color(Color::BLUE)
                    .add_to_ctx(ctx);
            });
    }
}
