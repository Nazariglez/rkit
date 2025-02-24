use super::components::{UINode, UIRender};
use super::style::UIStyle;
use crate::draw::{Draw2D, Sprite};
use crate::gfx::Color;
use crate::math::Vec2;
use bevy_ecs::prelude::*;

// -- Container
#[derive(Component, Debug, Clone, Copy)]
#[require(UIStyle, UIRender(container_render_component))]
pub struct UIContainer {
    pub bg_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_size: f32,
}

impl Default for UIContainer {
    fn default() -> Self {
        Self {
            bg_color: Default::default(),
            border_color: Default::default(),
            border_size: 2.0,
        }
    }
}

fn container_render_component() -> UIRender {
    UIRender::run::<(&UIContainer, &UINode), _>(render_container)
}

fn render_container(draw: &mut Draw2D, (container, node): (&UIContainer, &UINode)) {
    if let Some(bg_color) = container.bg_color {
        draw.rect(Vec2::ZERO, node.size).color(bg_color);
    }

    if let Some(border_color) = container.border_color {
        let offset = (container.border_size * 0.5).max(1.0);
        let size = node.size - offset * 2.0;
        draw.rect(Vec2::splat(offset), size)
            .stroke_color(border_color)
            .stroke(container.border_size);
    }
}

// -- Image
#[derive(Component, Debug, Clone)]
#[require(UIStyle, UIRender(image_render_component))]
pub struct UIImage {
    pub sprite: Sprite,
}

fn image_render_component() -> UIRender {
    UIRender::run::<(&UIImage, &UINode), _>(render_image)
}

fn render_image(draw: &mut Draw2D, (image, node): (&UIImage, &UINode)) {
    let scale = node.size() / image.sprite.size();

    draw.image(&image.sprite)
        .scale(scale)
        .origin(Vec2::splat(0.5))
        .translate(node.size() * 0.5);
}
