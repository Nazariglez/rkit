use super::{
    UIScene, ui,
    widgets::{UIContainer, UIImage, UIRichText, UIText},
};
use crate::{
    draw::{Font, HAlign, RichTextLayout, Sprite},
    gfx::Color,
    macros::ui_widget,
    math::Vec2,
};

#[doc(hidden)]
pub trait ChildContent {
    fn append_to(self, children: &mut Vec<UIScene>);
}

impl ChildContent for UIScene {
    fn append_to(self, children: &mut Vec<UIScene>) {
        children.push(self);
    }
}

impl<I> ChildContent for I
where
    I: IntoIterator<Item = UIScene>,
{
    fn append_to(self, children: &mut Vec<UIScene>) {
        children.extend(self);
    }
}

#[ui_widget(Node)]
pub fn node(children: impl IntoIterator<Item = UIScene>) -> UIScene {
    ui::node().children(children)
}

#[ui_widget(Row)]
pub fn row(children: impl IntoIterator<Item = UIScene>) -> UIScene {
    ui::row().children(children)
}

#[ui_widget(Column)]
pub fn column(children: impl IntoIterator<Item = UIScene>) -> UIScene {
    ui::column().children(children)
}

#[ui_widget(Container)]
pub fn container(
    bg_color: Option<Color>,
    border_color: Option<Color>,
    border_size: Option<f32>,
    corner_radius: Option<f32>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    let mut container = UIContainer::default();
    container.bg_color = bg_color.or(container.bg_color);
    container.border_color = border_color.or(container.border_color);
    container.border_size = border_size.unwrap_or(container.border_size);
    container.corner_radius = corner_radius.or(container.corner_radius);
    ui::container(container).children(children)
}

#[ui_widget(Text)]
pub fn text(
    text: String,
    font: Option<Font>,
    color: Option<Color>,
    size: Option<f32>,
    h_align: Option<HAlign>,
    line_height: Option<f32>,
    shadow_color: Option<Color>,
    shadow_offset: Option<Vec2>,
    color_tags: Option<bool>,
    outline_color: Option<Color>,
    outline_width: Option<u16>,
) -> UIScene {
    let mut component = UIText {
        text,
        ..Default::default()
    };
    component.font = font.or(component.font);
    component.color = color.unwrap_or(component.color);
    component.size = size.unwrap_or(component.size);
    component.h_align = h_align.unwrap_or(component.h_align);
    component.line_height = line_height.or(component.line_height);
    component.shadow_color = shadow_color.unwrap_or(component.shadow_color);
    component.shadow_offset = shadow_offset.or(component.shadow_offset);
    component.color_tags = color_tags.unwrap_or(component.color_tags);
    component.outline_color = outline_color.unwrap_or(component.outline_color);
    component.outline_width = outline_width.unwrap_or(component.outline_width);
    ui::node().insert(component)
}

#[ui_widget(Image)]
pub fn image(
    sprite: Sprite,
    tint: Option<Color>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    let mut component = UIImage::new(sprite);
    component.tint = tint.or(component.tint);
    ui::image(component).children(children)
}

#[ui_widget(RichText)]
pub fn rich_text(
    layout: RichTextLayout,
    shadow_color: Option<Color>,
    shadow_offset: Option<Vec2>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    let mut component = UIRichText::new(layout);
    component.shadow_color = shadow_color.unwrap_or(component.shadow_color);
    component.shadow_offset = shadow_offset.or(component.shadow_offset);
    ui::rich_text(component).children(children)
}
