use super::{
    Font, HAlign, TextInfo, TextLayout,
    document::{
        DiagnosticSink, InlineObject, ResolvedStyle, ResolvedStyleId, SemanticDocument,
        SemanticEffects, SourceMapKind, SourceSpan, TextDiagnosticCode,
    },
    effect,
    layout::{AtomKind, LineGeometry, LogicalBounds, NewAtom},
    render::{PlacedGlyph, PlacedIcon, PlacedSolid, RenderItem, TextRenderPlan},
    rich::{self, RegisteredIcon, TextIconAlign},
};
use corelib::{
    gfx::Color,
    math::{Vec2, vec2},
};
use cosmic_text::{
    Attrs, Buffer, CacheKey, DecorationSpan, Family, FontSystem, Hinting, LayoutGlyph, Metrics,
    ShapeLine, Shaping, UnderlineStyle, Wrap,
};
use std::ops::Range;
use utils::helpers::closest_multiple_of;

const OBJECT_REPLACEMENT: char = '\u{FFFC}';
const ZERO_WIDTH_SPACE: char = '\u{200B}';

#[derive(Clone)]
pub(crate) struct GlyphSource(LayoutGlyph);

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct GlyphCacheKey(CacheKey);

pub(crate) struct PhysicalGlyph {
    pub(crate) key: GlyphCacheKey,
    pub(crate) offset: Vec2,
}

impl GlyphSource {
    pub(crate) fn physical(&self, scale: f32) -> PhysicalGlyph {
        let physical = self.0.physical((0.0, 0.0), scale);
        PhysicalGlyph {
            key: GlyphCacheKey(physical.cache_key),
            offset: vec2(physical.x as f32, physical.y as f32),
        }
    }
}

pub(crate) fn new_buffer() -> Buffer {
    Buffer::new_empty(Metrics::new(1.0, 1.0))
}

pub(crate) fn glyph_image<'a>(
    swash: &'a mut cosmic_text::SwashCache,
    font_system: &'a mut FontSystem,
    key: GlyphCacheKey,
) -> Option<cosmic_text::SwashImage> {
    swash.get_image_uncached(font_system, key.0)
}

pub(crate) struct ShapingInput {
    pub(crate) semantic_text: String,
    pub(crate) text: String,
    pub(crate) styles: Vec<ResolvedStyle>,
    pub(crate) spans: Vec<ShapingSpan>,
    pub(crate) objects: Vec<InlineObject>,
    pub(crate) map: Vec<ShapingMapSegment>,
    pub(crate) effects: SemanticEffects,
}

pub(crate) struct ResolvedInlineObject {
    pub(crate) icon: RegisteredIcon,
    pub(crate) size: Vec2,
    pub(crate) align: TextIconAlign,
    pub(crate) line_height: f32,
    pub(crate) style: ResolvedStyleId,
    pub(crate) source: SourceSpan,
}

pub(crate) struct ShapingMapSegment {
    shaping: Range<usize>,
    semantic: Range<usize>,
    source: SourceSpan,
    owner: ShapingOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShapingOwner {
    Text,
    Object(usize),
    WrapHint,
    FormattingControl,
}

pub(crate) struct MappedCluster {
    pub(crate) source: SourceSpan,
    pub(crate) semantic: Range<usize>,
    pub(crate) content: Range<usize>,
    pub(crate) owner: ShapingOwner,
}

pub(crate) struct ShapingSpan {
    pub(crate) range: Range<usize>,
    pub(crate) color: Color,
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) style: ResolvedStyleId,
    pub(crate) semantic: Range<usize>,
    pub(crate) content: Range<usize>,
    pub(crate) source: SourceSpan,
    pub(crate) owner: ShapingOwner,
    pub(crate) effect_membership: Range<usize>,
}

pub(crate) fn prepare(
    document: SemanticDocument<'_>,
    wrap: bool,
    diagnostics: &mut DiagnosticSink,
) -> Result<ShapingInput, String> {
    if let Err(error) = validate_source_map(&document) {
        let source = document
            .source_map
            .segments
            .first()
            .map(|segment| segment.source.clone())
            .unwrap_or(SourceSpan {
                id: super::TextSourceId::DEFAULT,
                range: 0..0,
            });
        diagnostics.error(TextDiagnosticCode::CoordinateMappingFailure, &source, None);
        return Err(error);
    }
    let mut text = String::with_capacity(document.text.len() + document.objects.len() * 3);
    let mut spans = Vec::new();
    let mut object_index = 0;
    let mut run_index = 0;
    let mut source_index = 0;
    let mut map = Vec::new();

    for (at, character) in document.text.char_indices() {
        append_objects(
            at,
            &document.objects,
            &document.styles,
            &document.effects.memberships,
            &mut object_index,
            &mut text,
            &mut spans,
            &mut map,
        )?;
        while document
            .runs
            .get(run_index)
            .is_some_and(|run| run.range.end <= at)
        {
            run_index += 1;
        }
        let run = document
            .runs
            .get(run_index)
            .filter(|run| run.range.contains(&at))
            .ok_or_else(|| "Semantic text byte is not owned by a style run".to_string())?;
        let style = document
            .styles
            .get(run.style.0)
            .ok_or_else(|| "Semantic text has an invalid style".to_string())?;
        let semantic = at..at + character.len_utf8();
        let source = source_span(&document, semantic.clone(), &mut source_index)?;
        let start = text.len();
        text.push(character);
        push_map(
            &mut map,
            ShapingMapSegment {
                shaping: start..text.len(),
                semantic: semantic.clone(),
                source: source.clone(),
                owner: if is_bidi_formatting_control(character) {
                    ShapingOwner::FormattingControl
                } else {
                    ShapingOwner::Text
                },
            },
        );
        push_span(
            &mut spans,
            &document.effects.memberships,
            start..text.len(),
            style,
            run.style,
            run.content.start + (semantic.start - run.range.start)
                ..run.content.start + (semantic.end - run.range.start),
            run.effect_membership.clone(),
            semantic.clone(),
            source.clone(),
            if is_bidi_formatting_control(character) {
                ShapingOwner::FormattingControl
            } else {
                ShapingOwner::Text
            },
        );
        if wrap && character == ' ' {
            let start = text.len();
            text.push(ZERO_WIDTH_SPACE);
            let boundary = SourceSpan {
                id: source.id,
                range: source.range.end..source.range.end,
            };
            push_map(
                &mut map,
                ShapingMapSegment {
                    shaping: start..text.len(),
                    semantic: semantic.end..semantic.end,
                    source: boundary.clone(),
                    owner: ShapingOwner::WrapHint,
                },
            );
            push_span(
                &mut spans,
                &document.effects.memberships,
                start..text.len(),
                style,
                run.style,
                run.content.end..run.content.end,
                run.effect_membership.clone(),
                semantic.end..semantic.end,
                boundary,
                ShapingOwner::WrapHint,
            );
        }
    }
    append_objects(
        document.text.len(),
        &document.objects,
        &document.styles,
        &document.effects.memberships,
        &mut object_index,
        &mut text,
        &mut spans,
        &mut map,
    )?;
    if object_index != document.objects.len() {
        return Err("Inline object is not on a semantic byte boundary".into());
    }
    Ok(ShapingInput {
        semantic_text: document.text.into_owned(),
        text,
        styles: document.styles,
        spans,
        objects: document.objects,
        map,
        effects: document.effects,
    })
}

fn append_objects(
    at: usize,
    objects: &[InlineObject],
    styles: &[ResolvedStyle],
    memberships: &[u32],
    object_index: &mut usize,
    text: &mut String,
    spans: &mut Vec<ShapingSpan>,
    map: &mut Vec<ShapingMapSegment>,
) -> Result<(), String> {
    while let Some(object) = objects.get(*object_index).filter(|object| object.at == at) {
        let style = styles
            .get(object.style.0)
            .ok_or_else(|| "Inline object has an invalid style".to_string())?;
        let start = text.len();
        text.push(OBJECT_REPLACEMENT);
        push_map(
            map,
            ShapingMapSegment {
                shaping: start..text.len(),
                semantic: object.at..object.at,
                source: object.source.clone(),
                owner: ShapingOwner::Object(*object_index),
            },
        );
        push_span(
            spans,
            memberships,
            start..text.len(),
            style,
            object.style,
            object.content.clone(),
            object.effect_membership.clone(),
            object.at..object.at,
            object.source.clone(),
            ShapingOwner::Object(*object_index),
        );
        *object_index = object_index
            .checked_add(1)
            .ok_or_else(|| "Inline object index overflowed".to_string())?;
    }
    Ok(())
}

fn push_span(
    spans: &mut Vec<ShapingSpan>,
    memberships: &[u32],
    range: Range<usize>,
    resolved: &ResolvedStyle,
    style: ResolvedStyleId,
    content: Range<usize>,
    effects: Range<usize>,
    semantic: Range<usize>,
    source: SourceSpan,
    owner: ShapingOwner,
) {
    if let Some(span) = spans.last_mut()
        && span.owner == owner
        && matches!(owner, ShapingOwner::Text | ShapingOwner::FormattingControl)
        && span.style == style
        && memberships[span.effect_membership.clone()] == memberships[effects.clone()]
        && span.range.end == range.start
        && span.semantic.end == semantic.start
        && span.source.id == source.id
        && span.source.range.end == source.range.start
    {
        span.range.end = range.end;
        span.content.end = content.end;
        span.semantic.end = semantic.end;
        span.source.range.end = source.range.end;
    } else {
        spans.push(ShapingSpan {
            range,
            color: resolved.color,
            font: resolved.font.clone(),
            size: resolved.size,
            line_height: resolved.line_height,
            underline: resolved.underline,
            strikethrough: resolved.strikethrough,
            style,
            semantic,
            content,
            source,
            owner,
            effect_membership: effects,
        });
    }
}

fn push_map(map: &mut Vec<ShapingMapSegment>, segment: ShapingMapSegment) {
    if let Some(previous) = map.last_mut() {
        let previous_copied = previous.owner == ShapingOwner::Text
            && previous.shaping.len() == previous.semantic.len()
            && previous.semantic.len() == previous.source.range.len();
        let segment_copied = segment.owner == ShapingOwner::Text
            && segment.shaping.len() == segment.semantic.len()
            && segment.semantic.len() == segment.source.range.len();
        if previous_copied
            && segment_copied
            && previous.source.id == segment.source.id
            && previous.shaping.end == segment.shaping.start
            && previous.semantic.end == segment.semantic.start
            && previous.source.range.end == segment.source.range.start
        {
            previous.shaping.end = segment.shaping.end;
            previous.semantic.end = segment.semantic.end;
            previous.source.range.end = segment.source.range.end;
            return;
        }
    }
    map.push(segment);
}

pub(crate) fn map_range(
    map: &[ShapingMapSegment],
    spans: &[ShapingSpan],
    shaping: Range<usize>,
    diagnostics: &mut DiagnosticSink,
) -> Result<MappedCluster, String> {
    let first_index = map.partition_point(|segment| segment.shaping.end <= shaping.start);
    let end_index = first_index
        + map[first_index..].partition_point(|segment| segment.shaping.start < shaping.end);
    let matches = &map[first_index..end_index];
    let Some(first) = matches
        .iter()
        .find(|segment| {
            !matches!(
                segment.owner,
                ShapingOwner::WrapHint | ShapingOwner::FormattingControl
            )
        })
        .or_else(|| matches.first())
    else {
        let source = SourceSpan {
            id: super::TextSourceId::DEFAULT,
            range: shaping.clone(),
        };
        diagnostics.error(TextDiagnosticCode::CoordinateMappingFailure, &source, None);
        return Err("Shaping range has no semantic owner".into());
    };
    let (mut source, mut semantic) = mapped_overlap(first, &shaping);
    let mut expanded = false;
    for segment in matches.iter().filter(|segment| {
        !matches!(
            segment.owner,
            ShapingOwner::WrapHint | ShapingOwner::FormattingControl
        )
    }) {
        let (next_source, next_semantic) = mapped_overlap(segment, &shaping);
        expanded |= segment.shaping.start > shaping.start || segment.shaping.end < shaping.end;
        if source.id == next_source.id {
            source.range.start = source.range.start.min(next_source.range.start);
            source.range.end = source.range.end.max(next_source.range.end);
        } else {
            diagnostics.error(
                TextDiagnosticCode::ClusterBoundaryExpanded,
                &source,
                Some(next_source.range),
            );
        }
        semantic.start = semantic.start.min(next_semantic.start);
        semantic.end = semantic.end.max(next_semantic.end);
    }
    if expanded {
        diagnostics.error(TextDiagnosticCode::ClusterBoundaryExpanded, &source, None);
    }

    let first_span = spans
        .iter()
        .find(|span| span.range.contains(&shaping.start) || span.range.start == shaping.start)
        .or_else(|| {
            spans
                .iter()
                .find(|span| span.range.start < shaping.end && span.range.end > shaping.start)
        });
    if let Some(first_span) = first_span
        && spans.iter().any(|span| {
            span.range.start < shaping.end
                && span.range.end > shaping.start
                && span.style != first_span.style
                && !matches!(
                    span.owner,
                    ShapingOwner::WrapHint | ShapingOwner::FormattingControl
                )
        })
    {
        diagnostics.error(TextDiagnosticCode::ClusterStyleConflict, &source, None);
    }

    let content = first_span.map_or(semantic.clone(), |span| match span.owner {
        ShapingOwner::Object(_) => span.content.clone(),
        _ => {
            let offset = semantic.start.saturating_sub(span.semantic.start);
            let start = span.content.start.saturating_add(offset);
            start..start.saturating_add(semantic.len())
        }
    });
    Ok(MappedCluster {
        source,
        semantic,
        content,
        owner: first.owner,
    })
}

fn mapped_overlap(
    segment: &ShapingMapSegment,
    shaping: &Range<usize>,
) -> (SourceSpan, Range<usize>) {
    let start = segment.shaping.start.max(shaping.start);
    let end = segment.shaping.end.min(shaping.end);
    let copied = segment.shaping.len() == segment.semantic.len()
        && segment.semantic.len() == segment.source.range.len();
    if copied {
        let offset = start - segment.shaping.start;
        let len = end - start;
        (
            SourceSpan {
                id: segment.source.id,
                range: segment.source.range.start + offset
                    ..segment.source.range.start + offset + len,
            },
            segment.semantic.start + offset..segment.semantic.start + offset + len,
        )
    } else {
        (segment.source.clone(), segment.semantic.clone())
    }
}

fn source_span(
    document: &SemanticDocument<'_>,
    semantic: Range<usize>,
    cursor: &mut usize,
) -> Result<SourceSpan, String> {
    let (offset, segment) = document.source_map.segments[*cursor..]
        .iter()
        .enumerate()
        .find(|(_, segment)| {
            !matches!(segment.kind, SourceMapKind::InlineObject(_))
                && segment.semantic.start <= semantic.start
                && segment.semantic.end >= semantic.end
        })
        .ok_or_else(|| "Semantic text range has no source mapping".to_string())?;
    *cursor += offset;
    match segment.kind {
        SourceMapKind::Copied => {
            let offset = semantic.start - segment.semantic.start;
            Ok(SourceSpan {
                id: segment.source.id,
                range: segment.source.range.start + offset
                    ..segment.source.range.start + offset + semantic.len(),
            })
        }
        SourceMapKind::EscapedOpen => Ok(segment.source.clone()),
        SourceMapKind::InlineObject(_) => Err("Semantic text mapped to an inline object".into()),
    }
}

fn validate_source_map(document: &SemanticDocument<'_>) -> Result<(), String> {
    for segment in &document.source_map.segments {
        if segment.semantic.start > segment.semantic.end
            || segment.semantic.end > document.text.len()
        {
            return Err("Semantic source map is out of bounds".into());
        }
        if segment.source.range.start > segment.source.range.end {
            return Err("Semantic source range is invalid".into());
        }
        if let SourceMapKind::InlineObject(index) = segment.kind {
            let object = document
                .objects
                .get(index)
                .ok_or_else(|| "Semantic source map refers to an unknown object".to_string())?;
            if object.source != segment.source || object.at != segment.semantic.start {
                return Err("Semantic object source map is inconsistent".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn layout(
    font_system: &mut FontSystem,
    buffer: &mut Buffer,
    default_font: Option<&Font>,
    text: &TextInfo,
    markup: ShapingInput,
    layout: &mut TextLayout,
    diagnostics: &mut DiagnosticSink,
) -> Result<(), String> {
    layout.clear();
    layout.semantic_text.push_str(&markup.semantic_text);
    let font = text.font.or(default_font);
    let (pixelated, ppem, res_ppem, line_height_pem) = font
        .map(|font| {
            (
                font.is_pixelated(),
                font.px_per_em,
                font.res_ppem,
                font.line_height_pem,
            )
        })
        .unwrap_or((false, 1.0, 1.0, 1.0));
    let attrs = font_attrs(font);
    let (font_size, _strike_scale, base_line_height) =
        validate_layout_metrics(text, pixelated, ppem, res_ppem, line_height_pem)?;
    buffer.set_metrics(Metrics::new(font_size, base_line_height));
    buffer.set_size(text.wrap_width, None);

    let mut span_profiles = Vec::with_capacity(markup.spans.len());
    let mut spans = Vec::with_capacity(markup.spans.len());
    for (index, span) in markup.spans.iter().enumerate() {
        let span_font = span.font.as_ref().or(font);
        let logical_size = span.size.unwrap_or(text.font_size);
        let logical_line_height = span.line_height.or(text.line_height);
        let (span_size, span_scale, span_line_height) =
            validate_style_metrics(span_font, logical_size, logical_line_height)?;
        let span_pixelated = span_font.is_some_and(Font::is_pixelated);
        let underline = if span.underline {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        };
        let mut span_attrs = font_attrs(span_font)
            .metrics(Metrics::new(span_size, span_line_height))
            .underline(underline);
        if span.strikethrough {
            span_attrs = span_attrs.strikethrough();
        }
        let span_attrs = span_attrs.metadata(index + 1);
        span_profiles.push((span_pixelated, span_scale, span_line_height));
        spans.push((&markup.text[span.range.clone()], span_attrs));
    }
    buffer.set_rich_text(spans, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let objects: Vec<_> = markup
        .objects
        .into_iter()
        .map(|object| {
            let style = markup
                .styles
                .get(object.style.0)
                .ok_or_else(|| "Text icon has an invalid resolved style".to_string())?;
            let height = object
                .options
                .height
                .or(style.size)
                .unwrap_or(text.font_size);
            let object_font = style.font.as_ref().or(font);
            let logical_line_height = style.line_height.or(text.line_height);
            let style_size = style.size.unwrap_or(text.font_size);
            let (_, _, line_height) =
                validate_style_metrics(object_font, style_size, logical_line_height)?;
            let source_size = object.icon.source_size();
            let ratio = source_size.x as f32 / source_size.y as f32;
            let width = height * ratio;
            if !height.is_finite()
                || height <= 0.0
                || !ratio.is_finite()
                || ratio <= 0.0
                || !width.is_finite()
                || width <= 0.0
            {
                return Err("Text icon has an invalid logical size".to_string());
            }
            Ok(ResolvedInlineObject {
                icon: object.icon,
                size: vec2(width, height),
                align: object.options.align,
                line_height,
                style: object.style,
                source: object.source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = vec![0_u8; objects.len()];
    let mut line_top = 0.0;
    let mut content_width = 0.0_f32;
    let mut line_base = 0_usize;

    for buffer_line in &buffer.lines {
        let line_text = buffer_line.text();
        let current_line_base = line_base;
        line_base = line_base
            .checked_add(line_text.len())
            .and_then(|base| base.checked_add(buffer_line.ending().as_str().len()))
            .ok_or_else(|| "Text shaping coordinate overflowed".to_string())?;
        let Some(shape) = buffer_line.shape_opt() else {
            continue;
        };
        let has_bidi_controls = line_text.chars().any(is_bidi_formatting_control);
        let shape = if objects.is_empty() && !has_bidi_controls {
            std::borrow::Cow::Borrowed(shape)
        } else {
            let mut shape = shape.clone();
            patch_shape(line_text, &mut shape, &markup.spans, &objects, &mut seen)?;
            std::borrow::Cow::Owned(shape)
        };
        let layout_lines = shape.layout(
            font_size,
            text.wrap_width,
            Wrap::WordOrGlyph,
            None,
            None,
            Hinting::Disabled,
        );
        if layout_lines.is_empty() {
            let height = buffer_line
                .attrs_list()
                .defaults()
                .metrics_opt
                .map(Metrics::from)
                .map(|metrics| metrics.line_height.max(metrics.font_size))
                .unwrap_or(base_line_height);
            let visual = layout.atoms.atom_count();
            layout.atoms.push_line(LineGeometry {
                top: line_top,
                width: 0.0,
                height,
                baseline: line_top,
                ascent: 0.0,
                descent: 0.0,
                visual_atoms: visual..visual,
                x_offset: 0.0,
                rtl: false,
            });
            layout.lines.push(rich::RichTextLine {
                offset_y: line_top,
                size: vec2(0.0, height),
            });
            line_top += height;
            continue;
        }
        for layout_line in layout_lines {
            let line_index = layout.lines.len();
            let visual_start = layout.atoms.atom_count();
            let min_x = layout_line
                .glyphs
                .iter()
                .filter(|glyph| !is_bidi_control_cluster(line_text, glyph.start, glyph.end))
                .flat_map(|glyph| [glyph.x, glyph.x + glyph.w])
                .reduce(f32::min)
                .unwrap_or(0.0);
            let mut baseline_icon = 0.0_f32;
            let mut non_baseline_icon = 0.0_f32;
            for icon in layout_line
                .glyphs
                .iter()
                .filter_map(|glyph| glyph.metadata.checked_sub(1))
                .filter_map(|index| markup.spans.get(index))
                .filter_map(|span| match span.owner {
                    ShapingOwner::Object(index) => Some(index),
                    _ => None,
                })
                .filter_map(|index| objects.get(index))
            {
                match icon.align {
                    rich::TextIconAlign::Baseline => baseline_icon = baseline_icon.max(icon.size.y),
                    _ => non_baseline_icon = non_baseline_icon.max(icon.size.y),
                }
            }
            let tallest_run = layout_line
                .glyphs
                .iter()
                .filter_map(|glyph| glyph.metadata.checked_sub(1))
                .filter_map(|index| span_profiles.get(index))
                .map(|profile| profile.2)
                .fold(base_line_height, f32::max);
            let text_ascent = layout_line.max_ascent;
            let text_descent = layout_line.max_descent;
            let base_ascent = text_ascent.max(baseline_icon);
            let content_height = base_ascent + text_descent;
            let height = tallest_run.max(content_height).max(non_baseline_icon);
            let baseline = line_top + (height - content_height) * 0.5 + base_ascent;
            let mut content_top = baseline - text_ascent;
            let mut content_bottom = baseline + text_descent;
            for (glyph_index, glyph) in layout_line.glyphs.iter().enumerate() {
                if is_bidi_control_cluster(line_text, glyph.start, glyph.end) {
                    continue;
                }
                let span_index = glyph
                    .metadata
                    .checked_sub(1)
                    .ok_or_else(|| "Text glyph is missing semantic metadata".to_string())?;
                let shaping_range = current_line_base + glyph.start..current_line_base + glyph.end;
                let mapped = map_range(
                    &markup.map,
                    &markup.spans,
                    shaping_range.clone(),
                    diagnostics,
                )?;
                if matches!(
                    mapped.owner,
                    ShapingOwner::WrapHint | ShapingOwner::FormattingControl
                ) {
                    continue;
                }
                let span_index = if matches!(mapped.owner, ShapingOwner::Text) {
                    markup
                        .spans
                        .iter()
                        .position(|span| {
                            matches!(span.owner, ShapingOwner::Text)
                                && span.semantic.contains(&mapped.semantic.start)
                        })
                        .unwrap_or(span_index)
                } else {
                    span_index
                };
                let span = markup
                    .spans
                    .get(span_index)
                    .ok_or_else(|| "Text glyph metadata is out of bounds".to_string())?;
                let source_range = mapped.source;
                let semantic_range = mapped.semantic;
                let content_range = mapped.content;
                let (pixelated, strike_scale, _) = span_profiles
                    .get(span_index)
                    .copied()
                    .ok_or_else(|| "Text glyph style is out of bounds".to_string())?;
                let decoration = layout_line
                    .decorations
                    .iter()
                    .find(|decoration| decoration.glyph_range.contains(&glyph_index));
                if let ShapingOwner::Object(index) = mapped.owner {
                    let icon = objects
                        .get(index)
                        .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                    if source_range != icon.source {
                        return Err("Text icon source identity changed during shaping".into());
                    }
                    let y = match icon.align {
                        rich::TextIconAlign::Middle => line_top + (height - icon.size.y) * 0.5,
                        rich::TextIconAlign::Baseline => baseline - icon.size.y,
                        rich::TextIconAlign::Top => line_top,
                        rich::TextIconAlign::Bottom => line_top + height - icon.size.y,
                    };
                    content_top = content_top.min(y);
                    content_bottom = content_bottom.max(y + icon.size.y);
                    let atom = layout.atoms.push(NewAtom {
                        kind: AtomKind::Icon,
                        source: source_range,
                        semantic: semantic_range,
                        content: content_range,
                        shaping: shaping_range,
                        line: line_index,
                        style: icon.style,
                        bidi_level: glyph.level.number(),
                        advance: glyph.w,
                        bounds: LogicalBounds {
                            x: glyph.x - min_x,
                            y,
                            width: icon.size.x,
                            height: icon.size.y,
                        },
                        color: span.color,
                    })?;
                    layout.plan.push(RenderItem::Icon(PlacedIcon {
                        atom,
                        icon: icon.icon.clone(),
                        pos: vec2(glyph.x - min_x, y),
                        size: icon.size,
                    }));
                    push_decorations(
                        &mut layout.plan,
                        atom,
                        glyph.x - min_x,
                        icon.size.x,
                        baseline,
                        glyph.font_size,
                        span.underline,
                        span.strikethrough,
                        pixelated,
                        decoration,
                    );
                } else {
                    let kind = if line_text
                        .get(glyph.start..glyph.end)
                        .is_some_and(|cluster| cluster.chars().all(char::is_whitespace))
                    {
                        AtomKind::Space
                    } else {
                        AtomKind::Text
                    };
                    let atom = layout.atoms.push(NewAtom {
                        kind,
                        source: source_range,
                        semantic: semantic_range,
                        content: content_range,
                        shaping: shaping_range,
                        line: line_index,
                        style: span.style,
                        bidi_level: glyph.level.number(),
                        advance: glyph.w,
                        bounds: LogicalBounds {
                            x: glyph.x - min_x,
                            y: line_top,
                            width: glyph.w,
                            height,
                        },
                        color: span.color,
                    })?;
                    layout.plan.push(RenderItem::Glyph(PlacedGlyph {
                        atom,
                        source: GlyphSource(glyph.clone()),
                        origin: vec2(-min_x, baseline),
                        pixelated,
                        strike_scale,
                    }));
                    push_decorations(
                        &mut layout.plan,
                        atom,
                        glyph.x - min_x,
                        glyph.w,
                        baseline,
                        glyph.font_size,
                        span.underline,
                        span.strikethrough,
                        pixelated,
                        decoration,
                    );
                }
            }
            let size = vec2(layout_line.w, height);
            content_width = content_width.max(size.x);
            layout.atoms.push_line(LineGeometry {
                top: line_top,
                width: size.x,
                height,
                baseline,
                ascent: baseline - content_top,
                descent: content_bottom - baseline,
                visual_atoms: visual_start..layout.atoms.atom_count(),
                x_offset: 0.0,
                rtl: layout_line
                    .glyphs
                    .first()
                    .is_some_and(|glyph| glyph.level.is_rtl()),
            });
            layout.lines.push(rich::RichTextLine {
                offset_y: line_top,
                size,
            });
            line_top += height;
        }
    }
    if seen.iter().any(|count| *count != 1) {
        return Err("Text icon placeholder did not produce exactly one glyph".into());
    }
    super::validate_finite(content_width, "Text layout width")?;
    super::validate_finite(line_top, "Text layout height")?;
    for (line_index, line) in layout.lines.iter().enumerate() {
        let offset = match text.h_align {
            HAlign::Left => 0.0,
            HAlign::Center => (content_width - line.size.x) * 0.5,
            HAlign::Right => content_width - line.size.x,
        };
        layout.atoms.set_line_offset(line_index, offset)?;
    }
    layout.plan.apply_line_offsets(&layout.atoms);
    layout.atoms.finish(layout.semantic_text.len())?;
    compile_effects(layout, markup.effects)?;
    let outline_pad = f32::from(text.outline_width) * 2.0;
    let size = vec2(content_width + outline_pad, line_top + outline_pad);
    super::validate_finite(size.x, "Text layout width")?;
    super::validate_finite(size.y, "Text layout height")?;
    layout.size = size;
    layout.outline_width = text.outline_width;
    Ok(())
}

fn compile_effects(layout: &mut TextLayout, effects: SemanticEffects) -> Result<(), String> {
    for (order, occurrence) in effects.occurrences.into_iter().enumerate() {
        if occurrence.order != order as u32 || usize::from(occurrence.depth) > order {
            return Err("Text effect occurrence order is invalid".into());
        }
        let Some(atoms) = layout.atoms.effect_range(occurrence.content)? else {
            continue;
        };
        let callback = effects
            .callbacks
            .get(occurrence.effect as usize)
            .ok_or_else(|| "Text effect callback is missing".to_string())?;
        let callback = match layout
            .effect_callbacks
            .iter()
            .position(|stored| std::sync::Arc::ptr_eq(stored, callback))
        {
            Some(index) => index,
            None => {
                let index = layout.effect_callbacks.len();
                layout.effect_callbacks.push(callback.clone());
                index
            }
        };
        layout
            .effects
            .push(effect::EffectOccurrence { callback, atoms });
    }
    Ok(())
}

fn font_attrs(font: Option<&Font>) -> Attrs<'_> {
    match font {
        Some(font) => Attrs::new()
            .family(Family::Name(&font.family))
            .weight(font.weight)
            .style(font.style)
            .stretch(font.stretch),
        None => Attrs::new(),
    }
}

fn validate_style_metrics(
    font: Option<&Font>,
    logical_size: f32,
    logical_line_height: Option<f32>,
) -> Result<(f32, f32, f32), String> {
    let (pixelated, ppem, res_ppem, line_height_pem) = font
        .map(|font| {
            (
                font.is_pixelated(),
                font.px_per_em,
                font.res_ppem,
                font.line_height_pem,
            )
        })
        .unwrap_or((false, 1.0, 1.0, 1.0));
    let info = TextInfo {
        font,
        text: "",
        wrap_width: None,
        font_size: logical_size,
        line_height: logical_line_height,
        h_align: HAlign::Left,
        color_tags: false,
        default_color: Color::WHITE,
        outline_width: 0,
        strict_metrics: true,
    };
    validate_layout_metrics(&info, pixelated, ppem, res_ppem, line_height_pem)
}

fn validate_layout_metrics(
    text: &TextInfo,
    pixelated: bool,
    ppem: f32,
    res_ppem: f32,
    line_height_pem: f32,
) -> Result<(f32, f32, f32), String> {
    if text.strict_metrics {
        super::validate_positive(text.font_size, "Text size")?;
        if let Some(height) = text.line_height {
            super::validate_positive(height, "Text line height")?;
        }
        if let Some(width) = text.wrap_width {
            super::validate_positive(width, "Text maximum width")?;
        }
    }
    super::validate_finite(text.font_size, "Text size")?;
    if let Some(height) = text.line_height {
        super::validate_finite(height, "Text line height")?;
    }
    if let Some(width) = text.wrap_width {
        super::validate_finite(width, "Text maximum width")?;
    }
    super::validate_positive(ppem, "Text font pixel scale")?;
    super::validate_positive(line_height_pem, "Text font line-height scale")?;

    let font_size = text.font_size * ppem;
    super::validate_positive(font_size, "Text effective font size")?;
    let strike_scale = if pixelated {
        super::validate_positive(res_ppem, "Text pixel font resolution")?;
        if res_ppem > usize::MAX as f32 {
            return Err("Text pixel font resolution is out of range".into());
        }
        let base = res_ppem as usize;
        if base == 0 {
            return Err("Text pixel font resolution is out of range".into());
        }
        let rounded_size = font_size.round();
        if rounded_size > usize::MAX as f32 {
            return Err("Text effective font size is out of range".into());
        }
        let snapped_size = (closest_multiple_of(rounded_size as usize, base) as f32).max(res_ppem);
        super::validate_positive(snapped_size, "Text snapped pixel font size")?;
        snapped_size / font_size
    } else {
        1.0
    };
    super::validate_positive(strike_scale, "Text font strike scale")?;

    let line_height = text.line_height.unwrap_or(font_size * line_height_pem);
    if text.line_height.is_some() {
        super::validate_finite(line_height, "Text effective line height")?;
    } else {
        super::validate_positive(line_height, "Text effective line height")?;
    }
    Ok((font_size, strike_scale, line_height))
}

fn push_decorations(
    plan: &mut TextRenderPlan,
    atom: super::layout::AtomId,
    x: f32,
    width: f32,
    baseline: f32,
    font_size: f32,
    underline: bool,
    strikethrough: bool,
    pixelated: bool,
    decoration: Option<&DecorationSpan>,
) {
    if width <= 0.0 {
        return;
    }
    let metrics = decoration.map(|decoration| &decoration.data);
    if underline {
        let offset = metrics.map_or(-0.125, |data| data.underline_metrics.offset);
        let thickness = metrics.map_or(1.0 / 14.0, |data| data.underline_metrics.thickness);
        plan.push(RenderItem::Solid(PlacedSolid {
            atom,
            pos: vec2(x, baseline - offset * font_size),
            size: vec2(width, thickness * font_size),
            pixelated,
        }));
    }
    if strikethrough {
        let offset = metrics.map_or(0.3, |data| data.strikethrough_metrics.offset);
        let thickness = metrics.map_or(1.0 / 14.0, |data| data.strikethrough_metrics.thickness);
        plan.push(RenderItem::Solid(PlacedSolid {
            atom,
            pos: vec2(x, baseline - offset * font_size),
            size: vec2(width, thickness * font_size),
            pixelated,
        }));
    }
}

pub(crate) fn is_bidi_formatting_control(character: char) -> bool {
    matches!(
        character,
        '\u{202A}'
            | '\u{202B}'
            | '\u{202C}'
            | '\u{202D}'
            | '\u{202E}'
            | '\u{2066}'
            | '\u{2067}'
            | '\u{2068}'
            | '\u{2069}'
    )
}

pub(crate) fn is_bidi_control_cluster(text: &str, start: usize, end: usize) -> bool {
    text.get(start..end).is_some_and(|cluster| {
        !cluster.is_empty() && cluster.chars().all(is_bidi_formatting_control)
    })
}

pub(crate) fn patch_shape(
    text: &str,
    shape: &mut ShapeLine,
    spans: &[ShapingSpan],
    objects: &[ResolvedInlineObject],
    seen: &mut [u8],
) -> Result<(), String> {
    for span in &mut shape.spans {
        for word in &mut span.words {
            for glyph in &mut word.glyphs {
                if is_bidi_control_cluster(text, glyph.start, glyph.end) {
                    glyph.x_advance = 0.0;
                    glyph.y_advance = 0.0;
                    glyph.ascent = 0.0;
                    glyph.descent = 0.0;
                    continue;
                }
                let span_index = glyph
                    .metadata
                    .checked_sub(1)
                    .ok_or_else(|| "Text glyph is missing semantic metadata".to_string())?;
                let semantic = spans
                    .get(span_index)
                    .ok_or_else(|| "Text glyph metadata is out of bounds".to_string())?;
                let ShapingOwner::Object(index) = semantic.owner else {
                    continue;
                };
                let object = objects
                    .get(index)
                    .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                if text.get(glyph.start..glyph.end) != Some("\u{FFFC}") {
                    return Err("Text icon metadata does not refer to an object placeholder".into());
                }
                let count = seen
                    .get_mut(index)
                    .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| "Text icon placeholder was repeated too often".to_string())?;
                if *count > 1 {
                    return Err("Text icon placeholder produced multiple glyphs".into());
                }
                glyph.x_advance = object.size.x / object.size.y;
                glyph.metrics_opt = Some(Metrics::new(object.size.y, object.line_height));
                glyph.y_advance = 0.0;
                glyph.ascent = 0.0;
                glyph.descent = 0.0;
            }
        }
    }
    Ok(())
}
