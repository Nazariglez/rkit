use super::Font;
use corelib::gfx::Color;
use rustc_hash::FxHashMap;

#[derive(Clone, Debug, Default)]
pub struct TextStyle {
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) color: Option<Color>,
    pub(crate) underline: Option<bool>,
    pub(crate) strikethrough: Option<bool>,
}

impl TextStyle {
    pub const fn new() -> Self {
        Self {
            font: None,
            size: None,
            line_height: None,
            color: None,
            underline: None,
            strikethrough: None,
        }
    }

    pub fn font(mut self, font: &Font) -> Self {
        self.font = Some(font.clone());
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = Some(height);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn underline(mut self, enabled: bool) -> Self {
        self.underline = Some(enabled);
        self
    }

    pub fn strikethrough(mut self, enabled: bool) -> Self {
        self.strikethrough = Some(enabled);
        self
    }
}

#[derive(Debug, Default)]
pub struct TextStyles {
    styles: FxHashMap<String, TextStyle>,
}

impl TextStyles {
    pub fn new<K, I>(styles: I) -> Result<Self, String>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, TextStyle)>,
    {
        let mut registry = Self::default();
        for (id, style) in styles {
            let id = id.into();
            if !super::rich::is_markup_id(&id) {
                return Err(format!("Invalid text style ID '{id}'"));
            }
            if registry.styles.contains_key(&id) {
                return Err(format!("Duplicate text style ID '{id}'"));
            }
            if style.size.is_some_and(|size| !is_positive(size)) {
                return Err(format!(
                    "Text style '{id}' size must be finite and greater than zero"
                ));
            }
            if style.line_height.is_some_and(|height| !is_positive(height)) {
                return Err(format!(
                    "Text style '{id}' line height must be finite and greater than zero"
                ));
            }
            if style.color.is_some_and(|color| {
                ![color.r, color.g, color.b, color.a]
                    .into_iter()
                    .all(f32::is_finite)
            }) {
                return Err(format!("Text style '{id}' color must be finite"));
            }
            if registry.styles.len() == u32::MAX as usize {
                return Err("Text style registry is too large".into());
            }
            registry.styles.insert(id, style);
        }
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.styles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    pub(crate) fn get(&self, id: &str) -> Option<&TextStyle> {
        self.styles.get(id)
    }
}

fn is_positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}
