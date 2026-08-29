use super::document::{ResolvedStyleId, SourceSpan};
use std::ops::Range;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum AtomKind {
    Text,
    Space,
    Icon,
}

pub(crate) struct LogicalBounds {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) struct LayoutAtom {
    kind: AtomKind,
    items: Range<usize>,
    source: SourceSpan,
    semantic: Range<usize>,
    shaping: Range<usize>,
    line: u32,
    visual_order: u32,
    style: u32,
    bidi_level: u8,
    advance: f32,
    bounds: LogicalBounds,
}

pub(crate) struct NewAtom {
    pub(crate) kind: AtomKind,
    pub(crate) item: usize,
    pub(crate) source: SourceSpan,
    pub(crate) semantic: Range<usize>,
    pub(crate) shaping: Range<usize>,
    pub(crate) line: usize,
    pub(crate) style: ResolvedStyleId,
    pub(crate) bidi_level: u8,
    pub(crate) advance: f32,
    pub(crate) bounds: LogicalBounds,
}

pub(crate) struct LineGeometry {
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) baseline: f32,
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    pub(crate) visual_atoms: Range<usize>,
    pub(crate) x_offset: f32,
    pub(crate) rtl: bool,
}

#[derive(Default)]
pub(crate) struct LayoutAtoms {
    atoms: Vec<LayoutAtom>,
    logical_order: Vec<u32>,
    lines: Vec<LineGeometry>,
}

impl LayoutAtoms {
    pub(crate) fn clear(&mut self) {
        self.atoms.clear();
        self.logical_order.clear();
        self.lines.clear();
    }

    pub(crate) fn push(&mut self, new: NewAtom) -> Result<(), String> {
        let line = u32::try_from(new.line).map_err(|_| "Text line count exceeds layout limits")?;
        let style =
            u32::try_from(new.style.0).map_err(|_| "Text style count exceeds layout limits")?;
        if let Some(atom) = self.atoms.last_mut()
            && atom.kind == new.kind
            && atom.line == line
            && atom.style == style
            && atom.shaping == new.shaping
            && atom.items.end == new.item
        {
            atom.items.end += 1;
            atom.advance += new.advance;
            let right = (atom.bounds.x + atom.bounds.width).max(new.bounds.x + new.bounds.width);
            let bottom = (atom.bounds.y + atom.bounds.height).max(new.bounds.y + new.bounds.height);
            atom.bounds.x = atom.bounds.x.min(new.bounds.x);
            atom.bounds.y = atom.bounds.y.min(new.bounds.y);
            atom.bounds.width = right - atom.bounds.x;
            atom.bounds.height = bottom - atom.bounds.y;
            return Ok(());
        }
        let visual_order =
            u32::try_from(self.atoms.len()).map_err(|_| "Text atom count exceeds layout limits")?;
        self.atoms.push(LayoutAtom {
            kind: new.kind,
            items: new.item..new.item + 1,
            source: new.source,
            semantic: new.semantic,
            shaping: new.shaping,
            line,
            visual_order,
            style,
            bidi_level: new.bidi_level,
            advance: new.advance,
            bounds: new.bounds,
        });
        Ok(())
    }

    pub(crate) fn atom_count(&self) -> usize {
        self.atoms.len()
    }

    pub(crate) fn push_line(&mut self, line: LineGeometry) {
        self.lines.push(line);
    }

    pub(crate) fn set_line_offset(&mut self, line: usize, offset: f32) -> Result<(), String> {
        let geometry = self
            .lines
            .get_mut(line)
            .ok_or_else(|| "Text line geometry is out of bounds".to_string())?;
        geometry.x_offset = offset;
        for atom in &mut self.atoms[geometry.visual_atoms.clone()] {
            atom.bounds.x += offset;
        }
        Ok(())
    }

    pub(crate) fn finish(&mut self, semantic_len: usize) -> Result<(), String> {
        self.logical_order.clear();
        self.logical_order.reserve(self.atoms.len());
        for index in 0..self.atoms.len() {
            self.logical_order
                .push(u32::try_from(index).map_err(|_| "Text atom count exceeds layout limits")?);
        }
        self.logical_order.sort_by_key(|index| {
            let atom = &self.atoms[*index as usize];
            (atom.line, atom.shaping.start, atom.visual_order)
        });
        for line in &self.lines {
            if ![
                line.top,
                line.width,
                line.height,
                line.baseline,
                line.ascent,
                line.descent,
                line.x_offset,
            ]
            .into_iter()
            .all(f32::is_finite)
                || line.visual_atoms.start > line.visual_atoms.end
                || line.visual_atoms.end > self.atoms.len()
                || (line.rtl && line.visual_atoms.is_empty() && line.width > 0.0)
            {
                return Err("Text line geometry is invalid".into());
            }
        }
        for atom in &self.atoms {
            if atom.source.range.start > atom.source.range.end
                || atom.semantic.start > atom.semantic.end
                || atom.semantic.end > semantic_len
                || atom.shaping.start > atom.shaping.end
                || !atom.advance.is_finite()
                || ![
                    atom.bounds.x,
                    atom.bounds.y,
                    atom.bounds.width,
                    atom.bounds.height,
                ]
                .into_iter()
                .all(f32::is_finite)
                || atom.bidi_level > 125
            {
                return Err("Text atom geometry is invalid".into());
            }
        }
        Ok(())
    }

    pub(crate) fn visual(&self) -> impl Iterator<Item = &LayoutAtom> {
        self.atoms.iter()
    }
}

impl LayoutAtom {
    pub(crate) fn items(&self) -> Range<usize> {
        self.items.clone()
    }
}
