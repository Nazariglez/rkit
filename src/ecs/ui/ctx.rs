use crate::draw::text_metrics;
use bevy_ecs::prelude::*;
use taffy::{AvailableSpace, Size};

use super::widgets::{UIImage, UIText};

#[derive(Debug, Clone, Copy)]
pub(super) struct NodeContext {
    pub entity: Entity,
    pub typ: UINodeType,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UINodeType {
    #[default]
    Container,
    Text,
    Image,
}

pub(super) fn measure<T: Component>(
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    ctx: Option<&mut NodeContext>,
    images: &Query<&UIImage, With<T>>,
    texts: &Query<&UIText, With<T>>,
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
            typ: UINodeType::Container,
            ..
        }) => Size::ZERO,
        Some(NodeContext {
            entity,
            typ: UINodeType::Text,
        }) => match texts.get(*entity) {
            Ok(text) => measure_text(known_dimensions, available_space, text),
            Err(err) => {
                log::debug!("Cannot measure UIText: {err}");
                Size::ZERO
            }
        },
        Some(NodeContext {
            entity,
            typ: UINodeType::Image,
        }) => match images.get(*entity) {
            Ok(image) => measure_image(known_dimensions, image),
            Err(err) => {
                log::debug!("Cannot measure UIImage: {err}");
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

fn measure_text(
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    text: &UIText,
) -> Size<f32> {
    if text.text.is_empty() {
        return Size::ZERO;
    }

    let mut metrics = text_metrics(&text.text).size(text.size);
    if let Some(font) = &text.font {
        metrics = metrics.font(font);
    }
    if let Some(lh) = text.line_height {
        metrics = metrics.line_height(lh);
    }

    let max_width = known_dimensions.width.or(match available_space.width {
        AvailableSpace::Definite(w) => Some(w),
        _ => None,
    });
    if let Some(mw) = max_width {
        metrics = metrics.max_width(mw);
    }

    let size = metrics.measure().size;
    Size {
        width: known_dimensions.width.unwrap_or(size.x),
        height: known_dimensions.height.unwrap_or(size.y),
    }
}
