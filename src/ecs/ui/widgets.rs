use super::components::{UINode, UIRender};
use super::ctx::UINodeType;
use super::style::UIStyle;
use crate::draw::{Draw2D, Font, HAlign, Sprite};
use crate::gfx::Color;
use crate::math::{Vec2, vec2};
use bevy_ecs::prelude::*;

// -- Container
#[derive(Component, Debug, Clone, Copy)]
#[require(UIStyle, UIRender = container_render_component())]
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
#[require(UIStyle, UIRender = image_render_component(), UINodeType::Image)]
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

// -- Text
#[derive(Component, Debug, Clone)]
#[require(UIStyle, UIRender = text_renderer(), UINodeType::Text)]
pub struct UIText {
    pub font: Option<Font>,
    pub text: String,
    pub color: Color,
    pub size: f32,
    pub h_align: HAlign,
    pub line_height: Option<f32>,
}

impl Default for UIText {
    fn default() -> Self {
        Self {
            font: None,
            text: String::from(""),
            color: Color::WHITE,
            size: 14.0,
            h_align: HAlign::Left,
            line_height: None,
        }
    }
}

fn text_renderer() -> UIRender {
    UIRender::run::<(&UIText, &UINode), _>(render_text_sys)
}

fn render_text_sys(draw: &mut Draw2D, (text, node): (&UIText, &UINode)) {
    let data = TextData {
        node_size: node.size(),
        max_width: node.size().x + 1.0,
        font: text.font.as_ref(),
        text: &text.text,
        color: text.color,
        size: text.size,
        h_align: text.h_align,
        line_height: text.line_height,
    };

    draw_text(draw, &data);
}

struct TextData<'a> {
    node_size: Vec2,
    max_width: f32,
    font: Option<&'a Font>,
    text: &'a str,
    color: Color,
    size: f32,
    h_align: HAlign,
    line_height: Option<f32>,
}

fn draw_text(draw: &mut Draw2D, data: &TextData) {
    let mut d_text = draw.text(data.text);

    if let Some(font) = data.font {
        d_text.font(font);
    }

    if let Some(lh) = data.line_height {
        d_text.line_height(lh);
    }

    match data.h_align {
        HAlign::Left => {
            d_text
                .h_align_left()
                .origin(vec2(0.0, 0.5))
                .translate(data.node_size * 0.5 * vec2(0.0, 0.5));
        }
        HAlign::Center => {
            d_text
                .h_align_center()
                .origin(Vec2::splat(0.5))
                .translate(data.node_size * 0.5);
        }
        HAlign::Right => {
            d_text
                .h_align_right()
                .origin(vec2(1.0, 0.5))
                .translate(data.node_size * vec2(1.0, 0.5));
        }
    };

    d_text
        .color(data.color)
        .max_width(data.max_width)
        .size(data.size);
}
