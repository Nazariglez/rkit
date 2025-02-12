use super::components::{UINode, UIRender};
use super::style::UIStyle;
use crate::draw::Draw2D;
use crate::gfx::Color;
use crate::math::Vec2;
use bevy_ecs::prelude::*;

// -- Container
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(UIStyle, UIRender(container_render_component))]
pub struct UIContainer {
    pub bg_color: Option<Color>,
}

fn container_render_component() -> UIRender {
    UIRender::run::<(&UIContainer, &UINode), _>(render_container)
}

fn render_container(draw: &mut Draw2D, (container, node): (&UIContainer, &UINode)) {
    if let Some(bg_color) = container.bg_color {
        draw.rect(Vec2::ZERO, node.size).color(bg_color);
    }
}
