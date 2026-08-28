use super::document::{
    DiagnosticSink, InlineObject, SemanticDocument, SourceMap, SourceMapKind, SourceMapSegment,
    StyleRun, TextDiagnosticCode, TextDiagnosticSeverity, TextSourceId,
};
use super::rich::RegisteredIcon;
use super::style::{TextStyle, TextStyles};
use super::{Color, Font, TextIcons};
use corelib::math::UVec2;
use std::{borrow::Cow, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

const COLOR_TAG_OPEN: &str = "[color:#";
const COLOR_TAG_CLOSE: &str = "[/color]";
const STYLE_TAG_OPEN: &str = "[style:";
const STYLE_TAG_CLOSE: &str = "[/style]";
const ICON_TAG_OPEN: &str = "[icon:";
const MAX_ICON_TAG_LEN: usize = 108;
const MAX_EXTENDED_TAG_LEN: usize = 256;
const MAX_NESTING: usize = 64;

pub(crate) enum MarkupMode<'a> {
    Colors,
    Rich(&'a TextIcons),
    Extended {
        icons: Option<&'a TextIcons>,
        styles: Option<&'a TextStyles>,
        source_id: TextSourceId,
    },
}

#[derive(Clone)]
struct RunStyle {
    color: Color,
    font: Option<Font>,
    size: Option<f32>,
    line_height: Option<f32>,
}

impl RunStyle {
    fn patched(&self, patch: &TextStyle) -> Self {
        Self {
            color: patch.color.unwrap_or(self.color),
            font: patch.font.clone().or_else(|| self.font.clone()),
            size: patch.size.or(self.size),
            line_height: patch.line_height.or(self.line_height),
        }
    }
}

enum Token<'a> {
    Text(&'a str, RunStyle, Range<usize>),
    OwnedText(String, RunStyle, Range<usize>),
    Icon(RegisteredIcon, Option<f32>, RunStyle, Range<usize>),
}

pub(crate) fn plain(input: &str, color: Color, _wrap: bool) -> SemanticDocument<'_> {
    let runs = (!input.is_empty())
        .then(|| StyleRun {
            range: 0..input.len(),
            color,
            font: None,
            size: None,
            line_height: None,
        })
        .into_iter()
        .collect();
    SemanticDocument {
        text: Cow::Borrowed(input),
        runs,
        objects: Vec::new(),
        source_map: SourceMap {
            segments: (!input.is_empty())
                .then(|| SourceMapSegment {
                    source: 0..input.len(),
                    semantic: 0..input.len(),
                    kind: SourceMapKind::Copied,
                })
                .into_iter()
                .collect(),
        },
        diagnostics: Vec::new(),
    }
}

pub(crate) fn parse(
    input: &str,
    default_color: Color,
    mode: MarkupMode<'_>,
    wrap: bool,
) -> SemanticDocument<'static> {
    match mode {
        MarkupMode::Extended {
            icons,
            styles,
            source_id,
        } => parse_extended(input, default_color, icons, styles, source_id, wrap),
        mode => parse_legacy(input, default_color, mode, wrap),
    }
}

fn parse_legacy(
    input: &str,
    default_color: Color,
    mode: MarkupMode<'_>,
    wrap: bool,
) -> SemanticDocument<'static> {
    let mut tokens = Vec::new();
    let mut colors = vec![default_color];
    let mut cursor = 0;
    let mut text_start = 0;

    while let Some(offset) = input[cursor..].find('[') {
        let tag_start = cursor + offset;
        let remaining = &input[tag_start..];
        if remaining.starts_with(COLOR_TAG_OPEN) {
            let after_prefix = &remaining[COLOR_TAG_OPEN.len()..];
            if let Some((color, consumed)) = try_parse_open_tag(after_prefix) {
                push_text(
                    &mut tokens,
                    &input[text_start..tag_start],
                    RunStyle {
                        color: *colors.last().unwrap(),
                        font: None,
                        size: None,
                        line_height: None,
                    },
                    text_start..tag_start,
                );
                colors.push(color);
                cursor = tag_start + COLOR_TAG_OPEN.len() + consumed;
                text_start = cursor;
            } else {
                cursor = tag_start + 1;
            }
            continue;
        }
        if remaining.starts_with(COLOR_TAG_CLOSE) {
            push_text(
                &mut tokens,
                &input[text_start..tag_start],
                RunStyle {
                    color: *colors.last().unwrap(),
                    font: None,
                    size: None,
                    line_height: None,
                },
                text_start..tag_start,
            );
            if colors.len() > 1 {
                colors.pop();
            }
            cursor = tag_start + COLOR_TAG_CLOSE.len();
            text_start = cursor;
            continue;
        }
        let MarkupMode::Rich(icons) = mode else {
            cursor = tag_start + 1;
            continue;
        };
        if !remaining.starts_with(ICON_TAG_OPEN) {
            cursor = tag_start + 1;
            continue;
        }
        let Some(tag_end) = bounded_tag_end(input, tag_start, MAX_ICON_TAG_LEN) else {
            debug_warn("Malformed text icon tag");
            break;
        };
        let tag = &input[tag_start..tag_end];
        let Some((id, height)) = parse_icon_tag(tag) else {
            debug_warn("Malformed text icon tag");
            cursor = tag_end;
            continue;
        };
        let Some(icon) = icons.registered(id) else {
            debug_warn("Unknown text icon");
            cursor = tag_end;
            continue;
        };
        if height.is_some_and(|height| !icon_size_is_finite(height, icon.source_size())) {
            debug_warn("Invalid text icon size");
            cursor = tag_end;
            continue;
        }
        let style = RunStyle {
            color: *colors.last().unwrap(),
            font: None,
            size: None,
            line_height: None,
        };
        push_text(
            &mut tokens,
            &input[text_start..tag_start],
            style.clone(),
            text_start..tag_start,
        );
        tokens.push(Token::Icon(icon.clone(), height, style, tag_start..tag_end));
        cursor = tag_end;
        text_start = cursor;
    }
    push_text(
        &mut tokens,
        &input[text_start..],
        RunStyle {
            color: *colors.last().unwrap(),
            font: None,
            size: None,
            line_height: None,
        },
        text_start..input.len(),
    );
    normalize(tokens, wrap, Vec::new())
}

#[derive(Clone)]
enum EventKind {
    OpenColor(Color),
    CloseColor,
    OpenStyle(TextStyle),
    CloseStyle,
    Icon(RegisteredIcon, Option<f32>),
    Escape,
}

impl EventKind {
    fn range_name(&self) -> Option<&'static str> {
        match self {
            Self::OpenColor(_) | Self::CloseColor => Some("color"),
            Self::OpenStyle(_) | Self::CloseStyle => Some("style"),
            _ => None,
        }
    }

    fn is_open(&self) -> bool {
        matches!(self, Self::OpenColor(_) | Self::OpenStyle(_))
    }

    fn is_close(&self) -> bool {
        matches!(self, Self::CloseColor | Self::CloseStyle)
    }
}

struct Event {
    range: Range<usize>,
    kind: EventKind,
    active: bool,
}

fn parse_extended(
    input: &str,
    default_color: Color,
    icons: Option<&TextIcons>,
    styles: Option<&TextStyles>,
    source_id: TextSourceId,
    wrap: bool,
) -> SemanticDocument<'static> {
    let mut sink = DiagnosticSink::default();
    let mut events = scan_extended(input, icons, styles, source_id, &mut sink);
    match_ranges(&mut events, source_id, &mut sink);

    let mut tokens = Vec::new();
    let mut stack = Vec::new();
    let mut style = RunStyle {
        color: default_color,
        font: None,
        size: None,
        line_height: None,
    };
    let mut cursor = 0;
    for event in events.into_iter().filter(|event| event.active) {
        let range = event.range;
        push_text(
            &mut tokens,
            &input[cursor..range.start],
            style.clone(),
            cursor..range.start,
        );
        match event.kind {
            EventKind::OpenColor(color) => {
                stack.push(style.clone());
                style.color = color;
            }
            EventKind::OpenStyle(patch) => {
                stack.push(style.clone());
                style = style.patched(&patch);
            }
            EventKind::CloseColor | EventKind::CloseStyle => {
                style = stack
                    .pop()
                    .ok_or_else(|| "matched text range lost its opening state")
                    .unwrap();
            }
            EventKind::Icon(icon, height) => {
                tokens.push(Token::Icon(icon, height, style.clone(), range.clone()));
            }
            EventKind::Escape => {
                tokens.push(Token::OwnedText("[".into(), style.clone(), range.clone()))
            }
        }
        cursor = range.end;
    }
    push_text(&mut tokens, &input[cursor..], style, cursor..input.len());
    normalize(tokens, wrap, sink.finish())
}

fn scan_extended(
    input: &str,
    icons: Option<&TextIcons>,
    styles: Option<&TextStyles>,
    source_id: TextSourceId,
    sink: &mut DiagnosticSink,
) -> Vec<Event> {
    let mut boundaries = vec![false; input.len() + 1];
    boundaries[input.len()] = true;
    for (index, _) in input.grapheme_indices(true) {
        boundaries[index] = true;
    }

    let mut events = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = input[cursor..].find('[') {
        let start = cursor + offset;
        if input[start..].starts_with("[[") {
            events.push(Event {
                range: start..start + 2,
                kind: EventKind::Escape,
                active: true,
            });
            cursor = start + 2;
            continue;
        }

        let remaining = &input[start..];
        let reserved = remaining.starts_with("[color")
            || remaining.starts_with("[/color")
            || remaining.starts_with("[style")
            || remaining.starts_with("[/style")
            || remaining.starts_with("[icon");
        let Some(end) = bounded_tag_end(input, start, MAX_EXTENDED_TAG_LEN) else {
            if reserved {
                let mut end = input.len().min(start + MAX_EXTENDED_TAG_LEN);
                while end > start && !input.is_char_boundary(end) {
                    end -= 1;
                }
                diagnostic(
                    sink,
                    TextDiagnosticCode::TagLengthLimitExceeded,
                    source_id,
                    start..end,
                );
            }
            cursor = start + 1;
            continue;
        };
        let tag = &input[start..end];
        let kind = if let Some(body) = tag
            .strip_prefix(COLOR_TAG_OPEN)
            .and_then(|tag| tag.strip_suffix(']'))
        {
            Some(EventKind::OpenColor(parse_hex_color(body)))
        } else if tag == COLOR_TAG_CLOSE {
            Some(EventKind::CloseColor)
        } else if let Some(id) = tag
            .strip_prefix(STYLE_TAG_OPEN)
            .and_then(|tag| tag.strip_suffix(']'))
        {
            if !super::rich::is_markup_id(id) {
                diagnostic(
                    sink,
                    TextDiagnosticCode::InvalidIdentifier,
                    source_id,
                    start..end,
                );
                None
            } else if let Some(style) = styles.and_then(|styles| styles.get(id)) {
                Some(EventKind::OpenStyle(style.clone()))
            } else {
                diagnostic(
                    sink,
                    TextDiagnosticCode::UnknownStyleId,
                    source_id,
                    start..end,
                );
                None
            }
        } else if tag == STYLE_TAG_CLOSE {
            Some(EventKind::CloseStyle)
        } else if tag.starts_with(ICON_TAG_OPEN) {
            match parse_icon_tag(tag) {
                Some((id, height)) => match icons.and_then(|icons| icons.registered(id)) {
                    Some(icon)
                        if !height.is_some_and(|height| {
                            !icon_size_is_finite(height, icon.source_size())
                        }) =>
                    {
                        Some(EventKind::Icon(icon.clone(), height))
                    }
                    Some(_) => {
                        diagnostic(
                            sink,
                            TextDiagnosticCode::MalformedTag,
                            source_id,
                            start..end,
                        );
                        None
                    }
                    None => {
                        diagnostic(
                            sink,
                            TextDiagnosticCode::UnknownIconId,
                            source_id,
                            start..end,
                        );
                        None
                    }
                },
                None => {
                    diagnostic(
                        sink,
                        TextDiagnosticCode::MalformedTag,
                        source_id,
                        start..end,
                    );
                    None
                }
            }
        } else {
            if reserved {
                diagnostic(
                    sink,
                    TextDiagnosticCode::MalformedTag,
                    source_id,
                    start..end,
                );
            }
            None
        };

        if let Some(kind) = kind {
            if !boundaries[start] || !boundaries[end] {
                diagnostic(
                    sink,
                    TextDiagnosticCode::GraphemeBoundary,
                    source_id,
                    start..end,
                );
            } else {
                let active = !kind.is_open() && !kind.is_close();
                events.push(Event {
                    range: start..end,
                    kind,
                    active,
                });
            }
        }
        cursor = end;
    }
    events
}

fn match_ranges(events: &mut [Event], source_id: TextSourceId, sink: &mut DiagnosticSink) {
    let mut stack: Vec<usize> = Vec::new();
    let mut overflow_depth = 0_usize;
    for index in 0..events.len() {
        if events[index].kind.is_open() {
            if overflow_depth > 0 || stack.len() == MAX_NESTING {
                diagnostic(
                    sink,
                    TextDiagnosticCode::NestingLimitExceeded,
                    source_id,
                    events[index].range.clone(),
                );
                overflow_depth = overflow_depth.saturating_add(1);
            } else {
                stack.push(index);
            }
            continue;
        }
        if !events[index].kind.is_close() {
            continue;
        }
        if overflow_depth > 0 {
            overflow_depth -= 1;
            continue;
        }
        let name = events[index].kind.range_name().unwrap();
        let Some(&open_index) = stack.last() else {
            diagnostic(
                sink,
                TextDiagnosticCode::UnmatchedClosingTag,
                source_id,
                events[index].range.clone(),
            );
            continue;
        };
        if events[open_index].kind.range_name() == Some(name) {
            stack.pop();
            events[open_index].active = true;
            events[index].active = true;
            continue;
        }
        if let Some(position) = stack
            .iter()
            .rposition(|open| events[*open].kind.range_name() == Some(name))
        {
            let related = events[open_index].range.clone();
            sink.push(
                TextDiagnosticCode::CrossedRange,
                TextDiagnosticSeverity::Error,
                source_id,
                events[index].range.clone(),
                Some(related),
            );
            stack.truncate(position);
        } else {
            let related = events[open_index].range.clone();
            sink.push(
                TextDiagnosticCode::MismatchedClosingTag,
                TextDiagnosticSeverity::Error,
                source_id,
                events[index].range.clone(),
                Some(related),
            );
        }
    }
    for open in stack {
        diagnostic(
            sink,
            TextDiagnosticCode::UnclosedRange,
            source_id,
            events[open].range.clone(),
        );
    }
}

fn diagnostic(
    sink: &mut DiagnosticSink,
    code: TextDiagnosticCode,
    source_id: TextSourceId,
    range: Range<usize>,
) {
    sink.push(code, TextDiagnosticSeverity::Error, source_id, range, None);
}

fn push_text<'a>(
    tokens: &mut Vec<Token<'a>>,
    text: &'a str,
    style: RunStyle,
    source: Range<usize>,
) {
    if !text.is_empty() {
        tokens.push(Token::Text(text, style, source));
    }
}

fn normalize(
    tokens: Vec<Token<'_>>,
    _wrap: bool,
    diagnostics: Vec<super::TextDiagnostic>,
) -> SemanticDocument<'static> {
    let mut text = String::new();
    let mut runs: Vec<StyleRun> = Vec::new();
    let mut objects = Vec::new();
    let mut segments = Vec::new();
    for token in tokens {
        let start = text.len();
        let (style, source, kind) = match token {
            Token::Text(span_text, style, source) => {
                text.push_str(span_text);
                (style, source, SourceMapKind::Copied)
            }
            Token::OwnedText(span_text, style, source) => {
                text.push_str(&span_text);
                (style, source, SourceMapKind::EscapedOpen)
            }
            Token::Icon(icon, height, style, source) => {
                let object = objects.len();
                objects.push(InlineObject {
                    at: start,
                    source: source.clone(),
                    icon,
                    height,
                    color: style.color,
                    font: style.font.clone(),
                    size: style.size,
                    line_height: style.line_height,
                });
                segments.push(SourceMapSegment {
                    source,
                    semantic: start..start,
                    kind: SourceMapKind::InlineObject(object),
                });
                continue;
            }
        };
        if start == text.len() {
            continue;
        }
        segments.push(SourceMapSegment {
            source,
            semantic: start..text.len(),
            kind,
        });
        if let Some(run) = runs.last_mut()
            && run.color == style.color
            && same_font(&run.font, &style.font)
            && run.size == style.size
            && run.line_height == style.line_height
            && run.range.end == start
        {
            run.range.end = text.len();
        } else {
            runs.push(StyleRun {
                range: start..text.len(),
                color: style.color,
                font: style.font,
                size: style.size,
                line_height: style.line_height,
            });
        }
    }
    SemanticDocument {
        text: Cow::Owned(text),
        runs,
        objects,
        source_map: SourceMap { segments },
        diagnostics,
    }
}

fn same_font(left: &Option<Font>, right: &Option<Font>) -> bool {
    left.as_ref().map(Font::id) == right.as_ref().map(Font::id)
}

fn bounded_tag_end(input: &str, start: usize, limit: usize) -> Option<usize> {
    input[start..]
        .as_bytes()
        .iter()
        .take(limit)
        .position(|byte| *byte == b']')
        .map(|offset| start + offset + 1)
}

fn parse_icon_tag(tag: &str) -> Option<(&str, Option<f32>)> {
    let body = tag.strip_prefix(ICON_TAG_OPEN)?.strip_suffix(']')?;
    let (id, height) = match body.split_once(" size=") {
        Some((id, size)) => (id, Some(parse_icon_size(size)?)),
        None => (body, None),
    };
    if !super::rich::is_markup_id(id) || body.matches(" size=").count() > 1 {
        return None;
    }
    Some((id, height))
}

fn parse_icon_size(value: &str) -> Option<f32> {
    let bytes = value.as_bytes();
    if !(1..=31).contains(&bytes.len()) {
        return None;
    }
    let dot = bytes.iter().position(|byte| *byte == b'.');
    match dot {
        Some(index) if index > 0 && index < bytes.len() - 1 => {
            if index > 15 || bytes.len() - index - 1 > 15 {
                return None;
            }
        }
        Some(_) => return None,
        None if bytes.len() > 16 => return None,
        None => {}
    }
    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_digit() || *byte == b'.')
        || bytes.iter().filter(|byte| **byte == b'.').count() > 1
    {
        return None;
    }
    let size = value.parse::<f32>().ok()?;
    (size.is_finite() && size > 0.0).then_some(size)
}

fn icon_size_is_finite(height: f32, size: UVec2) -> bool {
    (height * size.x as f32 / size.y as f32).is_finite()
}

#[cfg(debug_assertions)]
fn debug_warn(message: &str) {
    log::warn!("{message}");
}

#[cfg(not(debug_assertions))]
fn debug_warn(_: &str) {}

fn try_parse_open_tag(s: &str) -> Option<(Color, usize)> {
    let close_bracket = s.find(']')?;
    Some((parse_hex_color(&s[..close_bracket]), close_bracket + 1))
}

fn parse_hex_color(hex: &str) -> Color {
    let mut value: u32 = 0;
    for &byte in hex.as_bytes().iter().take(8) {
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => 0,
        };
        value = (value << 4) | u32::from(nibble);
    }
    if hex.len() <= 6 {
        value = (value << 8) | 0xFF;
    }
    Color::hex(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_icon_sizes_preserve_surrounding_text() {
        let icons = TextIcons::default();
        for size in [
            "0",
            "-1",
            "+1",
            ".5",
            "1.",
            "1e2",
            "NaN",
            "inf",
            "1..2",
            "1 2",
            "12345678901234567",
            "1.1234567890123456",
            "12345678901234567890123456789012",
        ] {
            let input = format!("before [icon:soul size={size}] after");
            let markup = parse(&input, Color::WHITE, MarkupMode::Rich(&icons), false);
            assert_eq!(markup.text, input);
            assert!(markup.objects.is_empty());
        }
    }

    #[test]
    fn plain_text_is_one_span() {
        let markup = parse("Hello 世界", Color::WHITE, MarkupMode::Colors, false);
        assert_eq!(markup.text, "Hello 世界");
        assert_eq!(markup.runs.len(), 1);
        assert_eq!(markup.runs[0].color, Color::WHITE);
        assert_eq!(&markup.text[markup.runs[0].range.clone()], "Hello 世界");
    }

    #[test]
    fn color_tags_preserve_legacy_hex_rules() {
        let markup = parse(
            "[color:#FF000080]red[/color]",
            Color::WHITE,
            MarkupMode::Colors,
            false,
        );
        assert_eq!(markup.text, "red");
        assert_eq!(markup.runs[0].color, Color::hex(0xFF000080));
        assert_eq!(parse_hex_color("X"), Color::hex(0x000000FF));
    }

    #[test]
    fn nested_colors_restore_outer_color() {
        let markup = parse(
            "[color:#FF0000]red [color:#00FF00]green[/color] back[/color]",
            Color::WHITE,
            MarkupMode::Colors,
            false,
        );
        assert_eq!(markup.text, "red green back");
        assert_eq!(markup.runs.len(), 3);
        assert_eq!(markup.runs[0].color, Color::hex(0xFF0000FF));
        assert_eq!(markup.runs[1].color, Color::hex(0x00FF00FF));
        assert_eq!(markup.runs[2].color, Color::hex(0xFF0000FF));
    }

    #[test]
    fn colors_mode_keeps_icons_literal() {
        let markup = parse("[icon:soul] text", Color::WHITE, MarkupMode::Colors, false);
        assert_eq!(markup.text, "[icon:soul] text");
        assert!(markup.objects.is_empty());
    }

    #[test]
    fn malformed_icon_is_literal() {
        let icons = TextIcons::default();
        let input = "[icon:soul  size=20]";
        let markup = parse(input, Color::WHITE, MarkupMode::Rich(&icons), false);
        assert_eq!(markup.text, input);
        assert!(markup.objects.is_empty());
    }

    #[test]
    fn unknown_icon_is_literal() {
        let icons = TextIcons::default();
        let input = "[icon:unknown]";
        let markup = parse(input, Color::WHITE, MarkupMode::Rich(&icons), false);
        assert_eq!(markup.text, input);
        assert!(markup.objects.is_empty());
    }
}
