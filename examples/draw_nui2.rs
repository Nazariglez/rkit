use draw::Draw2D;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};
use rkit::nui::prelude::*;

#[derive(Default)]
struct State {
    color1: Color,
    color2: Color,
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State {
        color1: Color::WHITE,
        color2: Color::PINK,
    })
    .update(update)
    .run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.ui(&mut ()).show(|ctx| {
        // root
        let root_size = ctx.size();
        ctx.node()
            .set_style(
                Style::default()
                    .flex_row()
                    .size(root_size.x, root_size.y)
                    .align_items_center()
                    .justify_content_center(),
            )
            .on_draw(|draw, l, _| {
                draw_quad(draw, l.size.width, l.size.height, state.color2);
                // *state += 10.0;
            })
            .add_with_children(|ctx| {
                // container
                let container_size = root_size * 0.9;
                ctx.node()
                    .set_style(
                        Style::default()
                            .flex_row()
                            .justify_content_space_evenly()
                            .size(container_size.x, container_size.y),
                    )
                    .on_draw(|draw, l, _| {
                        draw_quad(draw, l.size.width, l.size.height, Color::ORANGE);
                    })
                    .add_with_children(|ctx| {
                        // column1
                        ctx.node()
                            .set_style(Style::default().size_auto().width(200.0))
                            .on_draw(|draw, l, _| {
                                draw_quad(draw, l.size.width, l.size.height, Color::RED);
                            })
                            .add();

                        // column2
                        ctx.node()
                            .set_style(Style::default().size_auto().width(200.0))
                            .on_draw(|draw, l, _| {
                                draw_quad(draw, l.size.width, l.size.height, state.color1);
                            })
                            .add();

                        // column3
                        ctx.node()
                            .set_style(Style::default().size_auto().width(200.0))
                            .on_draw(|draw, l, _| {
                                draw_quad(draw, l.size.width, l.size.height, Color::BLUE);
                            })
                            .add();
                    });
            });
    });

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_quad(draw: &mut Draw2D, x: f32, y: f32, color: Color) {
    draw.rect(Vec2::ZERO, vec2(x, y)).color(color);
}
