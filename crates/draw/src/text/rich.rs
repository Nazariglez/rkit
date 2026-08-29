use super::{
    HAlign, TextAtlas, TextInfo, TextLayout, TextMarkupPolicy, TextSourceId, TextStyles, document,
    get_mut_text_system, markup,
};
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
    pub fn new<K, I>(icons: I) -> Result<Self, String>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, Sprite)>,
    {
        let mut registry = Self::default();
        for (id, sprite) in icons {
            let id = id.into();
            if !is_markup_id(&id) {
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

pub(crate) fn is_markup_id(id: &str) -> bool {
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

/// Vertical placement of a Sprite icon within its final line box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextIconAlign {
    #[default]
    Middle,
    Baseline,
    Top,
    Bottom,
}

/// An unresolved programmatic Sprite icon occurrence.
#[derive(Clone, Debug)]
pub struct RichTextIcon {
    pub(crate) id: String,
    pub(crate) height: Option<f32>,
    pub(crate) align: TextIconAlign,
}

impl RichTextIcon {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            height: None,
            align: TextIconAlign::Middle,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.height = Some(size);
        self
    }

    pub fn align(mut self, align: TextIconAlign) -> Self {
        self.align = align;
        self
    }
}

impl From<&str> for RichTextIcon {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for RichTextIcon {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

struct RichConfig<'a> {
    icons: Option<&'a TextIcons>,
    styles: Option<&'a TextStyles>,
    font: Option<&'a super::Font>,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    color: Color,
    resolution: Option<f32>,
    h_align: HAlign,
}

impl Default for RichConfig<'_> {
    fn default() -> Self {
        Self {
            icons: None,
            styles: None,
            font: None,
            size: 14.0,
            line_height: None,
            max_width: None,
            color: Color::WHITE,
            resolution: None,
            h_align: HAlign::Left,
        }
    }
}

pub fn rich_text(text: &str) -> RichTextBuilder<'_> {
    RichTextBuilder {
        text,
        source_id: TextSourceId::DEFAULT,
        markup_policy: None,
        config: RichConfig::default(),
    }
}

pub struct RichTextBuilder<'a> {
    text: &'a str,
    source_id: TextSourceId,
    markup_policy: Option<TextMarkupPolicy>,
    config: RichConfig<'a>,
}

macro_rules! config_setters {
    () => {
        pub fn icons(mut self, icons: &'a TextIcons) -> Self {
            self.config.icons = Some(icons);
            self
        }
        pub fn styles(mut self, styles: &'a TextStyles) -> Self {
            self.config.styles = Some(styles);
            self
        }
        pub fn font(mut self, font: &'a super::Font) -> Self {
            self.config.font = Some(font);
            self
        }
        pub fn size(mut self, size: f32) -> Self {
            self.config.size = size;
            self
        }
        pub fn line_height(mut self, height: f32) -> Self {
            self.config.line_height = Some(height);
            self
        }
        pub fn max_width(mut self, width: f32) -> Self {
            self.config.max_width = Some(width);
            self
        }
        pub fn color(mut self, color: Color) -> Self {
            self.config.color = color;
            self
        }
        pub fn resolution(mut self, resolution: f32) -> Self {
            self.config.resolution = Some(resolution);
            self
        }
        pub fn h_align_left(mut self) -> Self {
            self.config.h_align = HAlign::Left;
            self
        }
        pub fn h_align_center(mut self) -> Self {
            self.config.h_align = HAlign::Center;
            self
        }
        pub fn h_align_right(mut self) -> Self {
            self.config.h_align = HAlign::Right;
            self
        }
    };
}

impl<'a> RichTextBuilder<'a> {
    config_setters!();

    pub fn source_id(mut self, source_id: TextSourceId) -> Self {
        self.source_id = source_id;
        self
    }

    pub fn markup_policy(mut self, policy: TextMarkupPolicy) -> Self {
        self.markup_policy = Some(policy);
        self
    }

    pub fn layout(self) -> Result<RichTextLayout, String> {
        let extended = self.config.styles.is_some() || self.markup_policy.is_some();
        let mode = if extended {
            markup::MarkupMode::Extended {
                icons: self.config.icons,
                styles: self.config.styles,
                source_id: self.source_id,
            }
        } else {
            match self.config.icons {
                Some(icons) => markup::MarkupMode::Rich(icons),
                None => markup::MarkupMode::Colors,
            }
        };
        let document = markup::parse(
            self.text,
            self.config.color,
            mode,
            self.config.max_width.is_some(),
        );
        compile_document(
            document,
            &self.config,
            self.markup_policy == Some(TextMarkupPolicy::Strict),
        )
    }
}

/// Builds an owned layout snapshot from a literal programmatic document.
pub fn rich_document(document: &document::RichTextDocument) -> RichDocumentBuilder<'_> {
    RichDocumentBuilder {
        document,
        config: RichConfig::default(),
    }
}

/// Configures layout of an owned literal rich-text document.
pub struct RichDocumentBuilder<'a> {
    document: &'a document::RichTextDocument,
    config: RichConfig<'a>,
}

impl<'a> RichDocumentBuilder<'a> {
    config_setters!();

    pub fn layout(self) -> Result<RichTextLayout, String> {
        let document = document::resolve_document(
            self.document,
            self.config.color,
            self.config.icons,
            self.config.styles,
        )?;
        compile_document(document, &self.config, false)
    }
}

fn compile_document(
    mut document: document::SemanticDocument<'_>,
    config: &RichConfig<'_>,
    strict: bool,
) -> Result<RichTextLayout, String> {
    let diagnostics = std::mem::take(&mut document.diagnostics);
    let shaping = super::shaping::prepare(document, config.max_width.is_some())?;
    let info = TextInfo {
        font: config.font,
        text: "",
        wrap_width: config.max_width,
        font_size: config.size,
        line_height: config.line_height,
        h_align: config.h_align,
        color_tags: true,
        default_color: config.color,
        outline_width: 0,
        strict_metrics: true,
    };
    let mut system = get_mut_text_system();
    let mut layout = TextLayout::default();
    system.layout_document(&info, shaping, &mut layout)?;
    if strict && let Some(error) = document::strict_error(&diagnostics) {
        return Err(error);
    }
    Ok(RichTextLayout {
        layout,
        resolution: config.resolution,
        diagnostics,
    })
}

pub struct RichTextLayout {
    pub(crate) layout: TextLayout,
    pub(crate) resolution: Option<f32>,
    diagnostics: Vec<super::TextDiagnostic>,
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

    pub fn diagnostics(&self) -> &[super::TextDiagnostic] {
        &self.diagnostics
    }
}

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
