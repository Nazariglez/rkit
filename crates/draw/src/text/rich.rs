use corelib::{
    gfx::TextureFilter,
    math::{UVec2, uvec2},
};
use rustc_hash::FxHashMap;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

static NEXT_ICON_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct TextIconId(u64);

#[derive(Clone)]
pub struct TextIcon {
    pub(crate) id: TextIconId,
    pub(crate) pixels: Arc<[u8]>,
    size: UVec2,
    pub(crate) sampling: TextureFilter,
}

impl TextIcon {
    /// Decodes and owns an image source for inline text use.
    pub fn from_image(encoded: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory(encoded)
            .map_err(|error| format!("Cannot decode text icon: {error}"))?
            .to_rgba8();
        Self::from_rgba(image.as_raw(), image.width(), image.height())
    }

    /// Copies an RGBA8 image source for inline text use.
    pub fn from_rgba(pixels: &[u8], width: u32, height: u32) -> Result<Self, String> {
        let Some(expected_len) = rgba_len(width, height) else {
            return Err("Text icon dimensions must be non-zero RGBA8 dimensions".into());
        };
        if pixels.len() != expected_len {
            return Err(format!(
                "Text icon RGBA8 data has {} bytes, expected {expected_len}",
                pixels.len()
            ));
        }

        Ok(Self {
            id: next_icon_id(),
            pixels: Arc::from(pixels),
            size: uvec2(width, height),
            sampling: TextureFilter::Linear,
        })
    }

    /// Selects how this icon is sampled when rendered in text.
    pub fn sampling(mut self, filter: TextureFilter) -> Self {
        if !same_filter(self.sampling, filter) {
            self.id = next_icon_id();
            self.sampling = filter;
        }
        self
    }

    /// Returns the decoded source dimensions.
    pub fn source_size(&self) -> UVec2 {
        self.size
    }
}

#[derive(Default)]
pub struct TextIcons {
    icons: FxHashMap<Box<str>, TextIcon>,
}

impl TextIcons {
    /// Creates an owned icon registry. Markup IDs are identifiers, not paths.
    pub fn new<K, I>(icons: I) -> Result<Self, String>
    where
        K: Into<Box<str>>,
        I: IntoIterator<Item = (K, TextIcon)>,
    {
        let mut registry = Self::default();
        for (id, icon) in icons {
            let id = id.into();
            if !is_icon_id(&id) {
                return Err(format!("Invalid text icon ID '{id}'"));
            }
            if registry.icons.contains_key(id.as_ref()) {
                return Err(format!("Duplicate text icon ID '{id}'"));
            }
            registry.icons.insert(id, icon);
        }
        Ok(registry)
    }

    pub fn get(&self, id: &str) -> Option<&TextIcon> {
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

fn rgba_len(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(4)
}

fn next_icon_id() -> TextIconId {
    TextIconId(NEXT_ICON_ID.fetch_add(1, Ordering::Relaxed))
}

fn same_filter(lhs: TextureFilter, rhs: TextureFilter) -> bool {
    matches!(
        (lhs, rhs),
        (TextureFilter::Linear, TextureFilter::Linear)
            | (TextureFilter::Nearest, TextureFilter::Nearest)
    )
}

use super::{HAlign, TextInfo, TextLayout, get_mut_text_system, markup};
use corelib::gfx::Color;
use corelib::math::Vec2;

pub fn rich_text(text: &str) -> RichTextBuilder<'_> {
    RichTextBuilder {
        text,
        icons: None,
        font: None,
        size: 14.0,
        line_height: None,
        max_width: None,
        color: Color::WHITE,
        resolution: 1.0,
        h_align: HAlign::Left,
    }
}

/// Builds an owned rich-text snapshot from borrowed markup, fonts, and icon registry data.
/// The snapshot retains used icon sources, so the registry and markup may be dropped afterward.
/// Rich text has no shadow or outline effects; changing layout properties requires rebuilding it.
pub struct RichTextBuilder<'a> {
    text: &'a str,
    icons: Option<&'a TextIcons>,
    font: Option<&'a super::Font>,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    color: Color,
    resolution: f32,
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
        self.resolution = resolution;
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
            resolution: self.resolution,
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
        system.ensure_layout(&layout)?;
        Ok(RichTextLayout { layout })
    }
}

/// An atlas-safe semantic layout snapshot with final bounds and line geometry.
/// Its retained glyph keys and icon sources are re-resolved after atlas changes; changing text,
/// icons, font, size, line height, width, color, alignment, or resolution requires relayout.
pub struct RichTextLayout {
    pub(crate) layout: TextLayout,
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
