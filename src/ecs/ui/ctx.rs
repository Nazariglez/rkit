use crate::draw::Font;
use crate::math::Vec2;
use bevy_ecs::prelude::*;
use taffy::{prelude::TaffyZero, AvailableSpace, Size};

use super::prelude::UIImage;

pub(super) struct NodeContext {
    pub entity: Entity,
    pub typ: NodeTyp,
}

pub(super) struct TextContext {
    pub text: String,
    pub size: f32,
    pub font: Font,
}

pub(super) struct ImageContext {
    pub size: Vec2,
}

pub(super) enum NodeTyp {
    Container,
    Text,
    Image,
}

pub(super) fn measure(
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    ctx: Option<&mut NodeContext>,
    images: Query<&UIImage>,
) -> Size<f32> {
    if let Size {
        width: Some(width),
        height: Some(height),
    } = known_dimensions
    {
        return Size { width, height };
    }

    match ctx {
        Some(NodeContext {
            typ: NodeTyp::Container,
            ..
        }) => Size::ZERO,
        Some(NodeContext {
            entity,
            typ: NodeTyp::Text,
        }) => Size::ZERO,
        Some(NodeContext {
            entity,
            typ: NodeTyp::Image,
        }) => match images.get(*entity) {
            Ok(image) => measure_image(known_dimensions, image),
            Err(err) => {
                log::error!("Cannot measure UIImage: {err}");
                Size::ZERO
            }
        },
        None => Size::ZERO,
    }
}

fn measure_image(known_dimensions: Size<Option<f32>>, image: &UIImage) -> Size<f32> {
    let img_size = image.sprite.size();
    match (known_dimensions.width, known_dimensions.height) {
        (Some(width), Some(height)) => Size { width, height },
        (Some(width), None) => Size {
            width,
            height: (width / img_size.x) * img_size.y,
        },
        (None, Some(height)) => Size {
            width: (height / img_size.y) * img_size.x,
            height,
        },
        (None, None) => Size {
            width: img_size.x,
            height: img_size.y,
        },
    }
}
