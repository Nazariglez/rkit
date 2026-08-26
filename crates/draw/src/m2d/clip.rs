use crate::{m2d::shapes::rounded_rect, shapes::SHAPE_TESSELLATOR};
use corelib::math::{Mat3, Mat4, Rect, Vec2, vec2, vec3, vec4};
use lyon::tessellation::FillOptions;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Coverage {
    Full,
    Rect(UnitRect),
    Empty,
}

impl Coverage {
    pub(super) fn intersect(self, other: Self) -> Self {
        match (self, other) {
            (Self::Empty, _) | (_, Self::Empty) => Self::Empty,
            (Self::Full, coverage) | (coverage, Self::Full) => coverage,
            (Self::Rect(a), Self::Rect(b)) => {
                let rect = UnitRect {
                    min: a.min.max(b.min),
                    max: a.max.min(b.max),
                };
                if rect.is_empty() {
                    Self::Empty
                } else {
                    Self::Rect(rect)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct UnitRect {
    pub(super) min: Vec2,
    pub(super) max: Vec2,
}

impl UnitRect {
    fn from_points(points: &[Vec2]) -> Self {
        let mut min = Vec2::ONE;
        let mut max = Vec2::ZERO;
        for point in points {
            min = min.min(*point);
            max = max.max(*point);
        }
        Self {
            min: min.clamp(Vec2::ZERO, Vec2::ONE),
            max: max.clamp(Vec2::ZERO, Vec2::ONE),
        }
    }

    fn is_empty(self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }
}

pub(super) struct RoundedGeometry {
    pub(super) vertices: Vec<f32>,
    pub(super) indices: Vec<u32>,
    pub(super) bounds: UnitRect,
}

#[derive(Clone, Debug)]
pub(super) struct GeometryRange {
    pub(super) vbo: Range<u64>,
    pub(super) ebo: Range<u64>,
    pub(super) count: u32,
}

#[derive(Clone, Debug)]
pub(super) struct MaskState {
    pub(super) geometry: GeometryRange,
    pub(super) coverage: UnitRect,
    pub(super) parent_depth: u8,
    pub(super) child_depth: u8,
}

#[derive(Clone, Debug)]
pub(super) enum ClipKind {
    Rect,
    Rounded(Option<MaskState>),
}

#[derive(Clone, Debug)]
pub(super) struct ClipFrame {
    pub(super) coverage: Coverage,
    pub(super) stencil_depth: u8,
    pub(super) kind: ClipKind,
}

pub(super) fn project_rect(rect: Rect, matrix: Mat3, projection: Mat4) -> Result<Coverage, String> {
    validate_inputs(rect, matrix, projection)?;
    if rect.is_empty() {
        return Ok(Coverage::Empty);
    }

    let min = rect.min();
    let max = rect.max();
    let local = [min, vec2(max.x, min.y), max, vec2(min.x, max.y)];
    let mut points = [Vec2::ZERO; 4];
    let mut w_sign = 0.0_f32;
    for (index, point) in local.into_iter().enumerate() {
        let transformed = matrix * vec3(point.x, point.y, 1.0);
        let clip = projection * vec4(transformed.x, transformed.y, 0.0, 1.0);
        if clip.w == 0.0 || (w_sign != 0.0 && clip.w.signum() != w_sign) {
            return Err("Rectangular clip crosses the projection plane".to_string());
        }
        w_sign = clip.w.signum();
        points[index] = normalized_point(clip)?;
    }

    let bounds = UnitRect::from_points(&points);
    let scale = points
        .iter()
        .fold(1.0_f32, |scale, point| scale.max(point.abs().max_element()));
    let tolerance = scale * 1e-5;
    if bounds.max.x - bounds.min.x <= tolerance || bounds.max.y - bounds.min.y <= tolerance {
        return Ok(Coverage::Empty);
    }

    let mut horizontal = [false; 4];
    for index in 0..4 {
        let edge = points[(index + 1) % 4] - points[index];
        let is_horizontal = edge.y.abs() <= tolerance && edge.x.abs() > tolerance;
        let is_vertical = edge.x.abs() <= tolerance && edge.y.abs() > tolerance;
        if !is_horizontal && !is_vertical {
            return Err("Rectangular clips must remain axis aligned after projection".to_string());
        }
        horizontal[index] = is_horizontal;
    }
    if horizontal[0] == horizontal[1]
        || horizontal[1] == horizontal[2]
        || horizontal[2] == horizontal[3]
        || horizontal[3] == horizontal[0]
    {
        return Err("Rectangular clips must remain axis aligned after projection".to_string());
    }

    if bounds.is_empty() {
        Ok(Coverage::Empty)
    } else {
        Ok(Coverage::Rect(bounds))
    }
}

pub(super) fn rounded_geometry(
    rect: Rect,
    radius: f32,
    matrix: Mat3,
    projection: Mat4,
) -> Result<Option<RoundedGeometry>, String> {
    validate_inputs(rect, matrix, projection)?;
    if !radius.is_finite() {
        return Err("Clip radius must be finite".to_string());
    }
    if rect.is_empty() {
        return Ok(None);
    }

    let radius = radius.max(0.0).min(rect.size.min_element() * 0.5);
    let path = rounded_rect(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        (radius, radius, radius, radius),
    );
    let (positions, indices) = SHAPE_TESSELLATOR.with_borrow_mut(|tessellator| {
        tessellator.fill_positions(&path, &FillOptions::default())
    })?;

    let mut vertices = Vec::with_capacity(positions.len() * 4);
    let mut points = Vec::with_capacity(positions.len());
    let mut w_sign = 0.0_f32;
    for [x, y] in positions {
        let transformed = matrix * vec3(x, y, 1.0);
        let clip = projection * vec4(transformed.x, transformed.y, 0.0, 1.0);
        if !clip.is_finite() || clip.w == 0.0 {
            return Err(
                "Rounded clip projection produced invalid homogeneous coordinates".to_string(),
            );
        }
        let sign = clip.w.signum();
        if w_sign != 0.0 && sign != w_sign {
            return Err("Rounded clip crosses the projection plane".to_string());
        }
        w_sign = sign;
        points.push(normalized_point(clip)?);
        vertices.extend_from_slice(clip.as_ref());
    }

    let bounds = UnitRect::from_points(&points);
    if bounds.is_empty() {
        Ok(None)
    } else {
        Ok(Some(RoundedGeometry {
            vertices,
            indices,
            bounds,
        }))
    }
}

fn validate_inputs(rect: Rect, matrix: Mat3, projection: Mat4) -> Result<(), String> {
    let rect_values = [rect.origin.x, rect.origin.y, rect.size.x, rect.size.y];
    if !rect_values.into_iter().all(f32::is_finite)
        || !matrix.to_cols_array().into_iter().all(f32::is_finite)
        || !projection.to_cols_array().into_iter().all(f32::is_finite)
    {
        return Err("Clip rectangle, matrix, and projection must be finite".to_string());
    }
    Ok(())
}

fn normalized_point(clip: corelib::math::Vec4) -> Result<Vec2, String> {
    if !clip.is_finite() || clip.w == 0.0 {
        return Err("Clip projection produced invalid homogeneous coordinates".to_string());
    }
    let ndc = clip.truncate() / clip.w;
    let point = vec2((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);
    if !point.is_finite() {
        return Err("Clip projection produced non-finite coordinates".to_string());
    }
    Ok(point)
}
