use std::fmt::Alignment;
use std::ops::Rem;
use draw::{create_draw_2d, Draw2D, Transform2DBuilder};
use rkit::gfx::Color;
use rkit::math::{vec2, Vec2};
use rkit::prelude::*;
use rkit::{gfx, time};

use taffy::prelude::*;
use corelib::math::Vec3;

#[derive(Resource, Default)]
pub struct TTree {
    tree: TaffyTree<()>,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(AddMainPlugins::default())
        .add_resource(TTree::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut tree: ResMut<TTree>) {
    println!("whatever");
}

fn draw_system(mut tree: ResMut<TTree>, win: Res<Window>) {
    let size = win.size();
    let tree = &mut tree.tree;
    let header_node = tree
        .new_leaf(
            Style {
                size: Size { width: length(size.x), height: length(100.0) },
                ..Default::default()
            },
        ).unwrap();

    let body_node = tree
        .new_leaf(
            Style {
                size: Size { width: length(size.x), height: auto() },
                flex_grow: 1.0,
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                ..Default::default()
            },
        ).unwrap();

    let footer_node = tree
        .new_leaf(
            Style {
                size: Size { width: length(size.x), height: length(100.0) }, // Fixed height
                ..Default::default()
            },
        ).unwrap();

    let root_node = tree
        .new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size { width: length(size.x), height: length(size.y) },
                ..Default::default()
            },
            &[header_node, body_node, footer_node],
        )
        .unwrap();

    // new
    let content = tree.new_leaf(
        Style {
        size: Size { width: percent(0.9), height: percent(0.9) },
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SpaceBetween),
            gap: Size { width: length(20.0), height: length(0.0) },
            padding: Rect::length(30.0),
        ..Default::default()
    }
    ).unwrap();
    tree.set_children(body_node, &[content]);

    for _ in (0..5) {
        let col = tree.new_leaf(
            Style {
                flex_grow: 1.0,
                size: Size { width: auto(), height: percent(1.0) },
                ..Default::default()
            }
        ).unwrap();

        tree.add_child(content, col);
    }

    tree.compute_layout(root_node, Size::MAX_CONTENT).unwrap();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    const COLORS: [Color; 19] = [
        Color::WHITE,
        Color::BLACK,
        Color::RED,
        Color::GREEN,
        Color::BLUE,
        Color::YELLOW,
        Color::MAGENTA,
        Color::SILVER,
        Color::GRAY,
        Color::OLIVE,
        Color::PURPLE,
        Color::MAROON,
        Color::BROWN,
        Color::SADDLE_BROWN,
        Color::AQUA,
        Color::TEAL,
        Color::NAVY,
        Color::ORANGE,
        Color::PINK,
    ];

    fn draw_node(node_id: NodeId, tree: &mut TaffyTree<()>, draw: &mut Draw2D, mut color_idx: usize) {
        let c = COLORS[color_idx.rem(COLORS.len())];
        println!("c {c:?} idx {color_idx}");

        let layout = tree.layout(node_id).unwrap();
        println!("\n{node_id:?}:\n{layout:?}");
        draw.push_matrix(Transform2DBuilder::default().set_translation(vec2(layout.location.x, layout.location.y)).build().as_mat3());

        draw.rect(Vec2::ZERO, vec2(layout.size.width, layout.size.height))
            .color(c);

        for child_id in tree.children(node_id).unwrap() {
            color_idx += 1;
            draw_node(child_id, tree, draw, color_idx);
        }

        draw.pop_matrix();
    }

    println!("--------------------");
    draw_node(root_node, tree, &mut draw, 0);
    println!("++++++++++++++++++++");

    gfx::render_to_frame(&draw).unwrap();
}
