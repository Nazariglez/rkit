use super::{
    Font, PlacedIcon,
    document::{InlineObject, SemanticDocument, SourceMapKind},
};
use corelib::gfx::Color;
use cosmic_text::{Metrics, ShapeLine};
use std::ops::Range;

const OBJECT_REPLACEMENT: char = '\u{FFFC}';
const ZERO_WIDTH_SPACE: char = '\u{200B}';

pub(crate) struct ShapingInput {
    pub(crate) semantic_text: String,
    pub(crate) text: String,
    pub(crate) spans: Vec<ShapingSpan>,
    pub(crate) objects: Vec<InlineObject>,
    pub(crate) map: Vec<ShapingMapSegment>,
}

pub(crate) struct ShapingMapSegment {
    shaping: Range<usize>,
    semantic: Range<usize>,
    source: Range<usize>,
}

pub(crate) struct ShapingSpan {
    pub(crate) range: Range<usize>,
    pub(crate) color: Color,
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) semantic: Range<usize>,
    pub(crate) source: Range<usize>,
    pub(crate) object: Option<usize>,
}

pub(crate) fn prepare(document: SemanticDocument<'_>, wrap: bool) -> Result<ShapingInput, String> {
    validate_source_map(&document)?;
    let mut text = String::with_capacity(document.text.len() + document.objects.len() * 3);
    let mut spans = Vec::new();
    let mut object_index = 0;
    let mut run_index = 0;
    let mut map = Vec::new();

    for (at, character) in document.text.char_indices() {
        append_objects(
            at,
            &document.objects,
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
        let semantic = at..at + character.len_utf8();
        let source = source_range(&document, semantic.clone())?;
        let start = text.len();
        text.push(character);
        push_map(
            &mut map,
            ShapingMapSegment {
                shaping: start..text.len(),
                semantic: semantic.clone(),
                source: source.clone(),
            },
        );
        push_span(
            &mut spans,
            start..text.len(),
            run.color,
            run.font.clone(),
            run.size,
            run.line_height,
            semantic.clone(),
            source.clone(),
            None,
        );
        if wrap && character == ' ' {
            let start = text.len();
            text.push(ZERO_WIDTH_SPACE);
            push_map(
                &mut map,
                ShapingMapSegment {
                    shaping: start..text.len(),
                    semantic: semantic.end..semantic.end,
                    source: source.end..source.end,
                },
            );
            push_span(
                &mut spans,
                start..text.len(),
                run.color,
                run.font.clone(),
                run.size,
                run.line_height,
                semantic.end..semantic.end,
                source.end..source.end,
                None,
            );
        }
    }
    append_objects(
        document.text.len(),
        &document.objects,
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
        spans,
        objects: document.objects,
        map,
    })
}

fn append_objects(
    at: usize,
    objects: &[InlineObject],
    object_index: &mut usize,
    text: &mut String,
    spans: &mut Vec<ShapingSpan>,
    map: &mut Vec<ShapingMapSegment>,
) -> Result<(), String> {
    while let Some(object) = objects.get(*object_index).filter(|object| object.at == at) {
        let start = text.len();
        text.push(OBJECT_REPLACEMENT);
        push_map(
            map,
            ShapingMapSegment {
                shaping: start..text.len(),
                semantic: object.at..object.at,
                source: object.source.clone(),
            },
        );
        push_span(
            spans,
            start..text.len(),
            object.color,
            object.font.clone(),
            object.size,
            object.line_height,
            object.at..object.at,
            object.source.clone(),
            Some(*object_index),
        );
        *object_index = object_index
            .checked_add(1)
            .ok_or_else(|| "Inline object index overflowed".to_string())?;
    }
    Ok(())
}

fn push_span(
    spans: &mut Vec<ShapingSpan>,
    range: Range<usize>,
    color: Color,
    font: Option<Font>,
    size: Option<f32>,
    line_height: Option<f32>,
    semantic: Range<usize>,
    source: Range<usize>,
    object: Option<usize>,
) {
    if let Some(span) = spans.last_mut()
        && span.object.is_none()
        && object.is_none()
        && span.color == color
        && span.font.as_ref().map(Font::id) == font.as_ref().map(Font::id)
        && span.size == size
        && span.line_height == line_height
        && span.range.end == range.start
        && span.semantic.end == semantic.start
        && span.source.end == source.start
    {
        span.range.end = range.end;
        span.semantic.end = semantic.end;
        span.source.end = source.end;
    } else {
        spans.push(ShapingSpan {
            range,
            color,
            font,
            size,
            line_height,
            semantic,
            source,
            object,
        });
    }
}

fn push_map(map: &mut Vec<ShapingMapSegment>, segment: ShapingMapSegment) {
    if let Some(previous) = map.last_mut() {
        let previous_copied = previous.shaping.len() == previous.semantic.len()
            && previous.semantic.len() == previous.source.len();
        let segment_copied = segment.shaping.len() == segment.semantic.len()
            && segment.semantic.len() == segment.source.len();
        if previous_copied
            && segment_copied
            && previous.shaping.end == segment.shaping.start
            && previous.semantic.end == segment.semantic.start
            && previous.source.end == segment.source.start
        {
            previous.shaping.end = segment.shaping.end;
            previous.semantic.end = segment.semantic.end;
            previous.source.end = segment.source.end;
            return;
        }
    }
    map.push(segment);
}

pub(crate) fn map_range(
    map: &[ShapingMapSegment],
    shaping: Range<usize>,
) -> Result<(Range<usize>, Range<usize>), String> {
    let first = map.partition_point(|segment| segment.shaping.end <= shaping.start);
    let mut matches = map[first..]
        .iter()
        .take_while(|segment| segment.shaping.start < shaping.end);
    let first = matches
        .next()
        .ok_or_else(|| "Shaping range has no semantic owner".to_string())?;
    let (mut source, mut semantic) = mapped_overlap(first, &shaping);
    for segment in matches {
        let (next_source, next_semantic) = mapped_overlap(segment, &shaping);
        source.start = source.start.min(next_source.start);
        source.end = source.end.max(next_source.end);
        semantic.start = semantic.start.min(next_semantic.start);
        semantic.end = semantic.end.max(next_semantic.end);
    }
    Ok((source, semantic))
}

fn mapped_overlap(
    segment: &ShapingMapSegment,
    shaping: &Range<usize>,
) -> (Range<usize>, Range<usize>) {
    let start = segment.shaping.start.max(shaping.start);
    let end = segment.shaping.end.min(shaping.end);
    let copied = segment.shaping.len() == segment.semantic.len()
        && segment.semantic.len() == segment.source.len();
    if copied {
        let offset = start - segment.shaping.start;
        let len = end - start;
        (
            segment.source.start + offset..segment.source.start + offset + len,
            segment.semantic.start + offset..segment.semantic.start + offset + len,
        )
    } else {
        (segment.source.clone(), segment.semantic.clone())
    }
}

fn source_range(
    document: &SemanticDocument<'_>,
    semantic: Range<usize>,
) -> Result<Range<usize>, String> {
    let segment = document
        .source_map
        .segments
        .iter()
        .find(|segment| {
            segment.semantic.start <= semantic.start && segment.semantic.end >= semantic.end
        })
        .ok_or_else(|| "Semantic text range has no source mapping".to_string())?;
    match segment.kind {
        SourceMapKind::Copied => {
            let offset = semantic.start - segment.semantic.start;
            Ok(segment.source.start + offset..segment.source.start + offset + semantic.len())
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
        if segment.source.start > segment.source.end {
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
    objects: &[PlacedIcon],
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
                let Some(index) = semantic.object else {
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
                glyph.metrics_opt = Some(Metrics::new(object.size.y, object.size.y));
                glyph.y_advance = 0.0;
                glyph.ascent = 0.0;
                glyph.descent = 0.0;
            }
        }
    }
    Ok(())
}
