use super::{RegisteredIcon, layout::AtomId, shaping::GlyphSource};
use corelib::math::Vec2;

#[derive(Default)]
pub(super) struct TextRenderPlan {
    items: Vec<RenderItem>,
}

impl TextRenderPlan {
    pub(super) fn clear(&mut self) {
        self.items.clear();
    }

    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    pub(super) fn push(&mut self, item: RenderItem) {
        self.items.push(item);
    }

    pub(super) fn get(&self, index: usize) -> Option<&RenderItem> {
        self.items.get(index)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &RenderItem> {
        self.items.iter()
    }

    pub(super) fn apply_line_offsets(&mut self, atoms: &super::layout::LayoutAtoms) {
        for item in &mut self.items {
            let Some(offset) = atoms.line_offset(item.atom()) else {
                continue;
            };
            item.offset_x(offset);
        }
    }
}

pub(super) enum RenderItem {
    Glyph(PlacedGlyph),
    Icon(PlacedIcon),
    Solid(PlacedSolid),
}

impl RenderItem {
    pub(super) fn atom(&self) -> AtomId {
        match self {
            Self::Glyph(glyph) => glyph.atom,
            Self::Icon(icon) => icon.atom,
            Self::Solid(solid) => solid.atom,
        }
    }

    fn offset_x(&mut self, offset: f32) {
        match self {
            Self::Glyph(glyph) => glyph.origin.x += offset,
            Self::Icon(icon) => icon.pos.x += offset,
            Self::Solid(solid) => solid.pos.x += offset,
        }
    }
}

pub(super) struct PlacedGlyph {
    pub(super) atom: AtomId,
    pub(super) source: GlyphSource,
    pub(super) origin: Vec2,
    pub(super) pixelated: bool,
    pub(super) strike_scale: f32,
}

pub(super) struct PlacedSolid {
    pub(super) atom: AtomId,
    pub(super) pos: Vec2,
    pub(super) size: Vec2,
    pub(super) pixelated: bool,
}

pub(super) struct PlacedIcon {
    pub(super) atom: AtomId,
    pub(super) icon: RegisteredIcon,
    pub(super) pos: Vec2,
    pub(super) size: Vec2,
}
