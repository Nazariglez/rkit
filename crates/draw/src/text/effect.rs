use super::{RichTextAtomKind, TextSourceId, layout::LayoutAtoms, rich::is_markup_id};
use corelib::{
    gfx::Color,
    math::{IntoVec2, Rect, Vec2},
};
use rustc_hash::FxHashMap;
use std::{ops::Range, sync::Arc};

pub(crate) type EffectCallback = dyn for<'a> Fn(TextEffectRun<'a>) + Send + Sync + 'static;

#[derive(Clone)]
pub struct TextEffect(pub(crate) Arc<EffectCallback>);

impl TextEffect {
    pub fn new<F>(callback: F) -> Self
    where
        F: for<'a> Fn(TextEffectRun<'a>) + Send + Sync + 'static,
    {
        Self(Arc::new(callback))
    }
}

#[derive(Default)]
pub struct TextEffects {
    effects: FxHashMap<String, TextEffect>,
}

impl TextEffects {
    pub fn new<K, I>(effects: I) -> Result<Self, String>
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, TextEffect)>,
    {
        let mut registry = Self::default();
        for (id, effect) in effects {
            let id = id.into();
            if !is_markup_id(&id) {
                return Err(format!("Invalid text effect ID '{id}'"));
            }
            if registry.effects.insert(id.clone(), effect).is_some() {
                return Err(format!("Duplicate text effect ID '{id}'"));
            }
        }
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
    pub(crate) fn get(&self, id: &str) -> Option<&TextEffect> {
        self.effects.get(id)
    }
}

#[derive(Clone)]
pub(crate) struct EffectOccurrence {
    pub(crate) callback: usize,
    pub(crate) atoms: Range<usize>,
}

#[derive(Clone, Copy)]
pub(crate) struct PreparedAtom {
    pub(crate) translation: Vec2,
    pub(crate) scale: f32,
    pub(crate) rotation: f32,
    pub(crate) color: Color,
    pub(crate) alpha: f32,
    pub(crate) hidden: bool,
    pub(crate) center: Vec2,
}

impl PreparedAtom {
    pub(crate) fn new(color: Color, hidden: bool, center: Vec2) -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: 0.0,
            color,
            alpha: 1.0,
            hidden,
            center,
        }
    }

    pub(crate) fn valid(self) -> bool {
        [
            self.translation.x,
            self.translation.y,
            self.scale,
            self.rotation,
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
            self.alpha,
        ]
        .into_iter()
        .all(f32::is_finite)
    }
}

pub struct TextEffectRun<'a> {
    pub(crate) time: f32,
    pub(crate) seed: u64,
    pub(crate) occurrence_index: usize,
    pub(crate) text: &'a str,
    pub(crate) atoms: &'a LayoutAtoms,
    pub(crate) start: usize,
    pub(crate) states: &'a mut [PreparedAtom],
}

impl TextEffectRun<'_> {
    pub fn time(&self) -> f32 {
        self.time
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn occurrence_index(&self) -> usize {
        self.occurrence_index
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn items_mut(&mut self) -> impl ExactSizeIterator<Item = TextEffectItem<'_>> + '_ {
        EffectItems {
            text: self.text,
            atoms: self.atoms,
            start: self.start,
            count: self.states.len(),
            index: 0,
            states: self.states.iter_mut(),
        }
    }
}

struct EffectItems<'a> {
    text: &'a str,
    atoms: &'a LayoutAtoms,
    start: usize,
    count: usize,
    index: usize,
    states: std::slice::IterMut<'a, PreparedAtom>,
}

impl<'a> Iterator for EffectItems<'a> {
    type Item = TextEffectItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let state = self.states.next()?;
        let index = self.index;
        self.index += 1;
        let atom = self.atoms.logical(self.start + index)?;
        Some(TextEffectItem {
            text: self.text,
            atom,
            state,
            index,
            count: self.count,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states.size_hint()
    }
}

impl ExactSizeIterator for EffectItems<'_> {
    fn len(&self) -> usize {
        self.states.len()
    }
}

pub struct TextEffectItem<'a> {
    text: &'a str,
    atom: &'a super::layout::LayoutAtom,
    state: &'a mut PreparedAtom,
    index: usize,
    count: usize,
}

impl TextEffectItem<'_> {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn cluster_text(&self) -> &str {
        &self.text[self.atom.semantic.clone()]
    }
    pub fn source_id(&self) -> TextSourceId {
        self.atom.source.id
    }
    pub fn source_range(&self) -> &Range<usize> {
        &self.atom.source.range
    }
    pub fn kind(&self) -> RichTextAtomKind {
        self.atom.kind.into()
    }
    pub fn line_index(&self) -> usize {
        self.atom.line as usize
    }
    pub fn logical_index(&self) -> usize {
        self.atom.logical_index as usize
    }
    pub fn visual_index(&self) -> usize {
        self.atom.visual_order as usize
    }
    pub fn logical_bounds(&self) -> Rect {
        self.atom.rect()
    }
    pub fn is_rtl(&self) -> bool {
        self.atom.bidi_level % 2 == 1
    }

    pub fn translate(&mut self, offset: impl IntoVec2) -> &mut Self {
        self.state.translation += offset.into_vec2();
        self
    }
    pub fn scale(&mut self, factor: f32) -> &mut Self {
        self.state.scale *= factor;
        self
    }
    pub fn rotate(&mut self, radians: f32) -> &mut Self {
        self.state.rotation += radians;
        self
    }
    pub fn tint(&mut self, tint: Color) -> &mut Self {
        self.state.color = Color::rgba(
            self.state.color.r * tint.r,
            self.state.color.g * tint.g,
            self.state.color.b * tint.b,
            self.state.color.a * tint.a,
        );
        self
    }
    pub fn set_color(&mut self, color: Color) -> &mut Self {
        self.state.color = color;
        self
    }
    pub fn multiply_alpha(&mut self, alpha: f32) -> &mut Self {
        self.state.alpha *= alpha;
        self
    }
    pub fn set_alpha(&mut self, alpha: f32) -> &mut Self {
        self.state.alpha = alpha;
        self
    }
    pub fn hide(&mut self) -> &mut Self {
        self.state.hidden = true;
        self
    }
}
