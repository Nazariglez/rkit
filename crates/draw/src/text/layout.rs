use super::{
    Color,
    document::{ResolvedStyleId, SourceSpan},
};
use corelib::math::{Rect, Vec2, vec2};
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AtomId(u32);

impl AtomId {
    fn index(self) -> usize {
        self.0 as usize
    }
}

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
    pub(crate) kind: AtomKind,
    pub(crate) source: SourceSpan,
    pub(crate) semantic: Range<usize>,
    shaping: Range<usize>,
    pub(crate) line: u32,
    pub(crate) visual_order: u32,
    style: u32,
    pub(crate) bidi_level: u8,
    advance: f32,
    bounds: LogicalBounds,
    color: Color,
    pub(crate) logical_index: u32,
}

pub(crate) struct NewAtom {
    pub(crate) kind: AtomKind,
    pub(crate) source: SourceSpan,
    pub(crate) semantic: Range<usize>,
    pub(crate) shaping: Range<usize>,
    pub(crate) line: usize,
    pub(crate) style: ResolvedStyleId,
    pub(crate) bidi_level: u8,
    pub(crate) advance: f32,
    pub(crate) bounds: LogicalBounds,
    pub(crate) color: Color,
}

pub(crate) struct AtomHit {
    pub(crate) source: SourceSpan,
    pub(crate) line: usize,
    pub(crate) kind: AtomKind,
    pub(crate) bounds: Rect,
    pub(crate) logical_index: usize,
    pub(crate) rtl: bool,
    pub(crate) leading: bool,
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

impl From<AtomKind> for super::rich::RichTextAtomKind {
    fn from(kind: AtomKind) -> Self {
        match kind {
            AtomKind::Text => Self::Text,
            AtomKind::Space => Self::Space,
            AtomKind::Icon => Self::Icon,
        }
    }
}

impl LayoutAtom {
    pub(crate) fn rect(&self) -> Rect {
        Rect::new(
            vec2(self.bounds.x, self.bounds.y),
            vec2(self.bounds.width, self.bounds.height),
        )
    }
}

impl LayoutAtoms {
    pub(crate) fn clear(&mut self) {
        self.atoms.clear();
        self.logical_order.clear();
        self.lines.clear();
    }

    pub(crate) fn push(&mut self, new: NewAtom) -> Result<AtomId, String> {
        let line = u32::try_from(new.line).map_err(|_| "Text line count exceeds layout limits")?;
        let style =
            u32::try_from(new.style.0).map_err(|_| "Text style count exceeds layout limits")?;
        if let Some(atom) = self.atoms.last_mut()
            && atom.kind == new.kind
            && atom.line == line
            && atom.style == style
            && atom.shaping == new.shaping
        {
            atom.advance += new.advance;
            let right = (atom.bounds.x + atom.bounds.width).max(new.bounds.x + new.bounds.width);
            let bottom = (atom.bounds.y + atom.bounds.height).max(new.bounds.y + new.bounds.height);
            atom.bounds.x = atom.bounds.x.min(new.bounds.x);
            atom.bounds.y = atom.bounds.y.min(new.bounds.y);
            atom.bounds.width = right - atom.bounds.x;
            atom.bounds.height = bottom - atom.bounds.y;
            return Ok(AtomId(atom.visual_order));
        }
        let visual_order =
            u32::try_from(self.atoms.len()).map_err(|_| "Text atom count exceeds layout limits")?;
        self.atoms.push(LayoutAtom {
            kind: new.kind,
            source: new.source,
            semantic: new.semantic,
            shaping: new.shaping,
            line,
            visual_order,
            style,
            bidi_level: new.bidi_level,
            advance: new.advance,
            bounds: new.bounds,
            color: new.color,
            logical_index: 0,
        });
        Ok(AtomId(visual_order))
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
        for (logical_index, visual_index) in self.logical_order.iter().copied().enumerate() {
            self.atoms[visual_index as usize].logical_index = u32::try_from(logical_index)
                .map_err(|_| "Text atom count exceeds layout limits")?;
        }
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

    pub(crate) fn line(&self, id: AtomId) -> Option<usize> {
        self.atoms.get(id.index()).map(|atom| atom.line as usize)
    }

    pub(crate) fn logical(&self, index: usize) -> Option<&LayoutAtom> {
        let visual = *self.logical_order.get(index)? as usize;
        self.atoms.get(visual)
    }

    pub(crate) fn logical_index(&self, id: AtomId) -> Option<usize> {
        self.atoms
            .get(id.index())
            .map(|atom| atom.logical_index as usize)
    }

    pub(crate) fn logical_color(&self, index: usize) -> Option<Color> {
        self.logical(index).map(|atom| atom.color)
    }

    pub(crate) fn hit(&self, point: Vec2) -> Option<AtomHit> {
        let line = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| point.y >= line.top && point.y <= line.top + line.height)
            .max_by(|(_, left), (_, right)| left.top.total_cmp(&right.top))?;
        let (line_index, line) = line;
        let mut best: Option<(&LayoutAtom, f32)> = None;
        for atom in &self.atoms[line.visual_atoms.clone()] {
            let left = atom.bounds.x;
            let right = left + atom.bounds.width;
            if atom.bounds.width <= 0.0 || point.x < left || point.x > right {
                continue;
            }
            let distance = (point.x - (left + right) * 0.5).abs();
            let replace = best.is_none_or(|(current, current_distance)| {
                distance < current_distance
                    || (distance == current_distance && atom.bounds.x > current.bounds.x)
            });
            if replace {
                best = Some((atom, distance));
            }
        }
        let (atom, _) = best?;
        let midpoint = atom.bounds.x + atom.bounds.width * 0.5;
        let rtl = atom.bidi_level % 2 == 1;
        let leading = if point.x == midpoint {
            true
        } else if rtl {
            point.x > midpoint
        } else {
            point.x < midpoint
        };
        Some(AtomHit {
            source: atom.source.clone(),
            line: line_index,
            kind: atom.kind,
            bounds: Rect::new(
                vec2(atom.bounds.x, atom.bounds.y),
                vec2(atom.bounds.width, atom.bounds.height),
            ),
            logical_index: atom.logical_index as usize,
            rtl,
            leading,
        })
    }
}
