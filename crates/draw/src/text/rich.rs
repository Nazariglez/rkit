use super::{HAlign, TextAtlas, TextInfo, TextLayout, get_mut_text_system, markup};
use crate::Sprite;
use corelib::{
    gfx::{Color, TextureFilter},
    math::{UVec2, Vec2, uvec2},
};
use rustc_hash::FxHashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct PixelRect {
    pub(crate) origin: UVec2,
    pub(crate) size: UVec2,
}

#[derive(Clone)]
pub(crate) struct RegisteredIcon {
    pub(crate) sprite: Sprite,
    pub(crate) frame: PixelRect,
    pub(crate) atlas: TextAtlas,
}

impl RegisteredIcon {
    fn new(id: &str, sprite: Sprite) -> Result<Self, String> {
        let frame = sprite.frame();
        let values = [frame.origin.x, frame.origin.y, frame.size.x, frame.size.y];
        let valid_numbers = values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0 && value.fract() == 0.0);
        if !valid_numbers || frame.size.x == 0.0 || frame.size.y == 0.0 {
            return Err(format!(
                "Text icon '{id}' frame must be a positive pixel-aligned rectangle"
            ));
        }

        let end = frame.origin + frame.size;
        let texture_size = sprite.texture().size();
        if end.x > texture_size.x || end.y > texture_size.y {
            return Err(format!("Text icon '{id}' frame exceeds its source texture"));
        }

        let atlas = match (sprite.sampler().min_filter(), sprite.sampler().mag_filter()) {
            (TextureFilter::Linear, TextureFilter::Linear) => TextAtlas::RgbaLinear,
            (TextureFilter::Nearest, TextureFilter::Nearest) => TextAtlas::RgbaNearest,
            _ => {
                return Err(format!(
                    "Text icon '{id}' requires matching minification and magnification filters"
                ));
            }
        };

        Ok(Self {
            sprite,
            frame: PixelRect {
                origin: uvec2(frame.origin.x as _, frame.origin.y as _),
                size: uvec2(frame.size.x as _, frame.size.y as _),
            },
            atlas,
        })
    }

    pub(crate) fn source_size(&self) -> UVec2 {
        self.frame.size
    }
}

#[derive(Default)]
pub struct TextIcons {
    icons: FxHashMap<String, RegisteredIcon>,
}

impl TextIcons {
    /// Creates an owned Sprite registry. Markup IDs are identifiers, not paths.
    pub fn new<K, I>(icons: I) -> Result<Self, String>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, Sprite)>,
    {
        let mut registry = Self::default();
        for (id, sprite) in icons {
            let id = id.into();
            if !is_icon_id(&id) {
                return Err(format!("Invalid text icon ID '{id}'"));
            }
            if registry.icons.contains_key(&id) {
                return Err(format!("Duplicate text icon ID '{id}'"));
            }
            let icon = RegisteredIcon::new(&id, sprite)?;
            registry.icons.insert(id, icon);
        }
        Ok(registry)
    }

    pub fn get(&self, id: &str) -> Option<&Sprite> {
        self.icons.get(id).map(|icon| &icon.sprite)
    }

    pub(crate) fn registered(&self, id: &str) -> Option<&RegisteredIcon> {
        self.icons.get(id)
    }

    pub fn len(&self) -> usize {
        self.icons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }
}

pub(crate) fn is_icon_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    let Some((&first, rest)) = bytes.split_first() else {
        return false;
    };
    if bytes.len() > 64 || !first.is_ascii_alphanumeric() {
        return false;
    }
    rest.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub fn rich_text(text: &str) -> RichTextBuilder<'_> {
    RichTextBuilder {
        text,
        icons: None,
        font: None,
        size: 14.0,
        line_height: None,
        max_width: None,
        color: Color::WHITE,
        resolution: None,
        h_align: HAlign::Left,
    }
}

/// Builds an owned rich-text snapshot from borrowed markup, fonts, and icon registry data.
/// The snapshot retains used icon sources, so the registry and markup may be dropped afterward.
/// Shadows are configured when drawing; outlines are unsupported. Changing layout properties
/// requires rebuilding the snapshot.
pub struct RichTextBuilder<'a> {
    text: &'a str,
    icons: Option<&'a TextIcons>,
    font: Option<&'a super::Font>,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    color: Color,
    resolution: Option<f32>,
    h_align: HAlign,
}

impl<'a> RichTextBuilder<'a> {
    pub fn icons(mut self, icons: &'a TextIcons) -> Self {
        self.icons = Some(icons);
        self
    }

    pub fn font(mut self, font: &'a super::Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = Some(height);
        self
    }

    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn resolution(mut self, resolution: f32) -> Self {
        self.resolution = Some(resolution);
        self
    }

    pub fn h_align_left(mut self) -> Self {
        self.h_align = HAlign::Left;
        self
    }

    pub fn h_align_center(mut self) -> Self {
        self.h_align = HAlign::Center;
        self
    }

    pub fn h_align_right(mut self) -> Self {
        self.h_align = HAlign::Right;
        self
    }

    /// Builds an owned layout snapshot. Changing text, registry, or style requires a new layout.
    pub fn layout(self) -> Result<RichTextLayout, String> {
        let info = TextInfo {
            font: self.font,
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            h_align: self.h_align,
            color_tags: true,
            default_color: self.color,
            outline_width: 0,
            strict_metrics: true,
        };
        let mode = match self.icons {
            Some(icons) => markup::MarkupMode::Rich(icons),
            None => markup::MarkupMode::Colors,
        };
        let markup = markup::parse(self.text, self.color, mode, self.max_width.is_some());
        let mut system = get_mut_text_system();
        let mut layout = TextLayout::default();
        system.layout_markup(&info, markup, &mut layout)?;
        Ok(RichTextLayout {
            layout,
            resolution: self.resolution,
        })
    }
}

/// A target-independent semantic layout snapshot with final bounds and line geometry.
pub struct RichTextLayout {
    pub(crate) layout: TextLayout,
    pub(crate) resolution: Option<f32>,
}

impl RichTextLayout {
    pub fn size(&self) -> Vec2 {
        self.layout.size
    }

    pub fn line_count(&self) -> usize {
        self.layout.lines.len()
    }

    pub fn lines(&self) -> &[RichTextLine] {
        &self.layout.lines
    }
}

/// Final immutable geometry for one rich-text line in its owning layout snapshot.
#[derive(Copy, Clone, Debug)]
pub struct RichTextLine {
    pub(crate) offset_y: f32,
    pub(crate) size: Vec2,
}

impl RichTextLine {
    pub fn offset_y(&self) -> f32 {
        self.offset_y
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }
}
