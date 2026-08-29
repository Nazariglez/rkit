use super::document::{
    IconOptions, SemanticDocument, SemanticResolver, SourceMapKind, SourceSpan, TextDiagnosticCode,
    TextSourceId,
};
use super::rich::TextIconAlign;
use super::{Color, TextEffects, TextIcons, TextStyles};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

const COLOR_TAG_OPEN: &str = "[color:#";
const COLOR_TAG_CLOSE: &str = "[/color]";
const STYLE_TAG_OPEN: &str = "[style:";
const STYLE_TAG_CLOSE: &str = "[/style]";
const ICON_TAG_OPEN: &str = "[icon:";
const UNDERLINE_TAG_OPEN: &str = "[u]";
const UNDERLINE_TAG_CLOSE: &str = "[/u]";
const STRIKETHROUGH_TAG_OPEN: &str = "[s]";
const STRIKETHROUGH_TAG_CLOSE: &str = "[/s]";
const EFFECT_TAG_OPEN: &str = "[effect:";
const EFFECT_TAG_CLOSE: &str = "[/effect]";
const MAX_ICON_TAG_LEN: usize = 108;
const MAX_EXTENDED_TAG_LEN: usize = 256;
const MAX_NESTING: usize = 64;

pub(crate) enum MarkupMode<'a> {
    Colors,
    Rich(&'a TextIcons),
    Extended {
        icons: Option<&'a TextIcons>,
        styles: Option<&'a TextStyles>,
        effects: Option<&'a TextEffects>,
        source_id: TextSourceId,
    },
}

pub(crate) fn plain(input: &str, color: Color, _wrap: bool) -> SemanticDocument<'_> {
    let mut resolver = SemanticResolver::new(color, None, None, None);
    resolver.text(
        input,
        SourceSpan {
            id: TextSourceId::DEFAULT,
            range: 0..input.len(),
        },
        SourceMapKind::Copied,
    );
    resolver.finish()
}

pub(crate) fn parse(
    input: &str,
    default_color: Color,
    mode: MarkupMode<'_>,
    _wrap: bool,
) -> SemanticDocument<'static> {
    match mode {
        MarkupMode::Extended {
            icons,
            styles,
            effects,
            source_id,
        } => parse_extended(input, default_color, icons, styles, effects, source_id),
        mode => parse_legacy(input, default_color, mode),
    }
}

fn parse_legacy(
    input: &str,
    default_color: Color,
    mode: MarkupMode<'_>,
) -> SemanticDocument<'static> {
    let icons = match mode {
        MarkupMode::Rich(icons) => Some(icons),
        _ => None,
    };
    let rich = icons.is_some();
    let mut resolver = SemanticResolver::new(default_color, icons, None, None);
    let mut colors = 0_usize;
    let mut cursor = 0;
    let mut text_start = 0;

    while let Some(offset) = input[cursor..].find('[') {
        let tag_start = cursor + offset;
        let remaining = &input[tag_start..];
        if remaining.starts_with(COLOR_TAG_OPEN) {
            let after_prefix = &remaining[COLOR_TAG_OPEN.len()..];
            if let Some((color, consumed)) = try_parse_open_tag(after_prefix) {
                append_markup_text(
                    &mut resolver,
                    input,
                    text_start..tag_start,
                    TextSourceId::DEFAULT,
                );
                resolver.push_color(color);
                colors += 1;
                cursor = tag_start + COLOR_TAG_OPEN.len() + consumed;
                text_start = cursor;
            } else {
                cursor = tag_start + 1;
            }
            continue;
        }
        if remaining.starts_with(COLOR_TAG_CLOSE) {
            append_markup_text(
                &mut resolver,
                input,
                text_start..tag_start,
                TextSourceId::DEFAULT,
            );
            if colors > 0 {
                resolver.pop_style().expect("tracked color scope");
                colors -= 1;
            }
            cursor = tag_start + COLOR_TAG_CLOSE.len();
            text_start = cursor;
            continue;
        }
        if !rich {
            cursor = tag_start + 1;
            continue;
        }
        if !remaining.starts_with(ICON_TAG_OPEN) {
            cursor = tag_start + 1;
            continue;
        }
        let Some(tag_end) = bounded_tag_end(input, tag_start, MAX_ICON_TAG_LEN) else {
            debug_warn("Malformed text icon tag");
            break;
        };
        let tag = &input[tag_start..tag_end];
        let Some((id, height)) = parse_legacy_icon(tag) else {
            debug_warn("Malformed text icon tag");
            cursor = tag_end;
            continue;
        };
        let source = SourceSpan {
            id: TextSourceId::DEFAULT,
            range: tag_start..tag_end,
        };
        let options = IconOptions {
            height,
            align: TextIconAlign::Middle,
        };
        if resolver.validate_icon(id, options).is_err() {
            debug_warn("Unknown or invalid text icon");
            cursor = tag_end;
            continue;
        }
        append_markup_text(
            &mut resolver,
            input,
            text_start..tag_start,
            TextSourceId::DEFAULT,
        );
        resolver
            .icon(id, options, source)
            .expect("validated legacy icon");
        cursor = tag_end;
        text_start = cursor;
    }
    append_markup_text(
        &mut resolver,
        input,
        text_start..input.len(),
        TextSourceId::DEFAULT,
    );
    resolver.finish()
}

#[derive(Clone)]
enum EventKind {
    OpenColor(Color),
    CloseColor,
    OpenStyle(String),
    CloseStyle,
    OpenUnderline,
    CloseUnderline,
    OpenStrikethrough,
    CloseStrikethrough,
    OpenEffect(String),
    CloseEffect,
    Icon(String, IconOptions),
    Escape,
}

impl EventKind {
    fn range_name(&self) -> Option<&'static str> {
        match self {
            Self::OpenColor(_) | Self::CloseColor => Some("color"),
            Self::OpenStyle(_) | Self::CloseStyle => Some("style"),
            Self::OpenUnderline | Self::CloseUnderline => Some("u"),
            Self::OpenStrikethrough | Self::CloseStrikethrough => Some("s"),
            Self::OpenEffect(_) | Self::CloseEffect => Some("effect"),
            _ => None,
        }
    }

    fn is_open(&self) -> bool {
        matches!(
            self,
            Self::OpenColor(_)
                | Self::OpenStyle(_)
                | Self::OpenUnderline
                | Self::OpenStrikethrough
                | Self::OpenEffect(_)
        )
    }

    fn is_close(&self) -> bool {
        matches!(
            self,
            Self::CloseColor
                | Self::CloseStyle
                | Self::CloseUnderline
                | Self::CloseStrikethrough
                | Self::CloseEffect
        )
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
    effects: Option<&TextEffects>,
    source_id: TextSourceId,
) -> SemanticDocument<'static> {
    let mut resolver = SemanticResolver::new(default_color, icons, styles, effects);
    resolver.set_source(source_id);
    let mut events = scan_extended(input, source_id, &mut resolver);
    match_ranges(&mut events, source_id, &mut resolver);

    let mut cursor = 0;
    let mut active_scopes = Vec::new();
    for event in events.into_iter().filter(|event| event.active) {
        let range = event.range;
        append_markup_text(&mut resolver, input, cursor..range.start, source_id);
        let source = SourceSpan {
            id: source_id,
            range: range.clone(),
        };
        match event.kind {
            EventKind::OpenColor(color) => {
                resolver.push_color(color);
                active_scopes.push(true);
            }
            EventKind::OpenStyle(id) => {
                let active = super::rich::is_markup_id(&id) && resolver.try_push_style(&id);
                if !active {
                    let code = if super::rich::is_markup_id(&id) {
                        TextDiagnosticCode::UnknownStyleId
                    } else {
                        TextDiagnosticCode::InvalidIdentifier
                    };
                    resolver.diagnostic(code, source.clone(), None);
                    resolver.text(&input[range.clone()], source, SourceMapKind::Copied);
                }
                active_scopes.push(active);
            }
            EventKind::OpenUnderline => {
                resolver.push_underline();
                active_scopes.push(true);
            }
            EventKind::OpenStrikethrough => {
                resolver.push_strikethrough();
                active_scopes.push(true);
            }
            EventKind::OpenEffect(id) => {
                let valid = super::rich::is_markup_id(&id);
                let active = valid && resolver.push_effect(&id).is_ok();
                if !active {
                    resolver.diagnostic(
                        if valid {
                            TextDiagnosticCode::UnknownEffectId
                        } else {
                            TextDiagnosticCode::InvalidIdentifier
                        },
                        source.clone(),
                        None,
                    );
                    resolver.text(&input[range.clone()], source, SourceMapKind::Copied);
                }
                active_scopes.push(active);
            }
            EventKind::CloseEffect => {
                if active_scopes.pop().unwrap_or(false) {
                    resolver.pop_effect().expect("matched effect scope");
                } else {
                    resolver.text(&input[range.clone()], source, SourceMapKind::Copied);
                }
            }
            EventKind::CloseColor
            | EventKind::CloseStyle
            | EventKind::CloseUnderline
            | EventKind::CloseStrikethrough => {
                if active_scopes.pop().unwrap_or(false) {
                    resolver.pop_style().expect("matched markup scope");
                } else {
                    resolver.text(&input[range.clone()], source, SourceMapKind::Copied);
                }
            }
            EventKind::Icon(id, options) => {
                if let Err(error) = resolver.icon(&id, options, source.clone()) {
                    let code = if !super::rich::is_markup_id(&id) {
                        TextDiagnosticCode::InvalidIdentifier
                    } else if error.contains("size") {
                        TextDiagnosticCode::MalformedTag
                    } else {
                        TextDiagnosticCode::UnknownIconId
                    };
                    resolver.diagnostic(code, source.clone(), None);
                    resolver.text(&input[range.clone()], source, SourceMapKind::Copied);
                }
            }
            EventKind::Escape => resolver.text("[", source, SourceMapKind::EscapedOpen),
        }
        cursor = range.end;
    }
    append_markup_text(&mut resolver, input, cursor..input.len(), source_id);
    resolver.finish()
}

fn scan_extended(
    input: &str,
    source_id: TextSourceId,
    resolver: &mut SemanticResolver<'_>,
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
            || remaining.starts_with("[icon")
            || remaining.starts_with(UNDERLINE_TAG_OPEN)
            || remaining.starts_with(UNDERLINE_TAG_CLOSE)
            || remaining.starts_with(STRIKETHROUGH_TAG_OPEN)
            || remaining.starts_with(STRIKETHROUGH_TAG_CLOSE)
            || remaining.starts_with(EFFECT_TAG_OPEN)
            || remaining.starts_with(EFFECT_TAG_CLOSE);
        let Some(end) = bounded_tag_end(input, start, MAX_EXTENDED_TAG_LEN) else {
            if reserved {
                let mut end = input.len().min(start + MAX_EXTENDED_TAG_LEN);
                while end > start && !input.is_char_boundary(end) {
                    end -= 1;
                }
                resolver.diagnostic(
                    TextDiagnosticCode::TagLengthLimitExceeded,
                    SourceSpan {
                        id: source_id,
                        range: start..end,
                    },
                    None,
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
            Some(EventKind::OpenStyle(id.to_owned()))
        } else if tag == STYLE_TAG_CLOSE {
            Some(EventKind::CloseStyle)
        } else if tag == UNDERLINE_TAG_OPEN {
            Some(EventKind::OpenUnderline)
        } else if tag == UNDERLINE_TAG_CLOSE {
            Some(EventKind::CloseUnderline)
        } else if tag == STRIKETHROUGH_TAG_OPEN {
            Some(EventKind::OpenStrikethrough)
        } else if tag == STRIKETHROUGH_TAG_CLOSE {
            Some(EventKind::CloseStrikethrough)
        } else if let Some(id) = tag
            .strip_prefix(EFFECT_TAG_OPEN)
            .and_then(|tag| tag.strip_suffix(']'))
        {
            Some(EventKind::OpenEffect(id.to_owned()))
        } else if tag == EFFECT_TAG_CLOSE {
            Some(EventKind::CloseEffect)
        } else if tag.starts_with(ICON_TAG_OPEN) {
            parse_extended_icon(tag).map(|(id, options)| EventKind::Icon(id.to_owned(), options))
        } else {
            None
        };

        if let Some(kind) = kind {
            if !boundaries[start] || !boundaries[end] {
                resolver.diagnostic(
                    TextDiagnosticCode::GraphemeBoundary,
                    SourceSpan {
                        id: source_id,
                        range: start..end,
                    },
                    None,
                );
            } else {
                let active = !kind.is_open() && !kind.is_close();
                events.push(Event {
                    range: start..end,
                    kind,
                    active,
                });
            }
        } else if reserved {
            resolver.diagnostic(
                TextDiagnosticCode::MalformedTag,
                SourceSpan {
                    id: source_id,
                    range: start..end,
                },
                None,
            );
        }
        cursor = end;
    }
    events
}

fn match_ranges(
    events: &mut [Event],
    source_id: TextSourceId,
    resolver: &mut SemanticResolver<'_>,
) {
    let mut stack: Vec<usize> = Vec::new();
    let mut overflow_depth = 0_usize;
    for index in 0..events.len() {
        if events[index].kind.is_open() {
            if overflow_depth > 0 || stack.len() == MAX_NESTING {
                diagnose_event(
                    resolver,
                    TextDiagnosticCode::NestingLimitExceeded,
                    source_id,
                    &events[index],
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
            diagnose_event(
                resolver,
                TextDiagnosticCode::UnmatchedClosingTag,
                source_id,
                &events[index],
            );
            continue;
        };
        if events[open_index].kind.range_name() == Some(name) {
            stack.pop();
            events[open_index].active = true;
            events[index].active = true;
            continue;
        }
        let related = events[open_index].range.clone();
        let code = if let Some(position) = stack
            .iter()
            .rposition(|open| events[*open].kind.range_name() == Some(name))
        {
            stack.truncate(position);
            TextDiagnosticCode::CrossedRange
        } else {
            TextDiagnosticCode::MismatchedClosingTag
        };
        resolver.diagnostic(
            code,
            SourceSpan {
                id: source_id,
                range: events[index].range.clone(),
            },
            Some(related),
        );
    }
    for open in stack {
        diagnose_event(
            resolver,
            TextDiagnosticCode::UnclosedRange,
            source_id,
            &events[open],
        );
    }
}

fn diagnose_event(
    resolver: &mut SemanticResolver<'_>,
    code: TextDiagnosticCode,
    source_id: TextSourceId,
    event: &Event,
) {
    resolver.diagnostic(
        code,
        SourceSpan {
            id: source_id,
            range: event.range.clone(),
        },
        None,
    );
}

fn append_markup_text(
    resolver: &mut SemanticResolver<'_>,
    input: &str,
    range: Range<usize>,
    source_id: TextSourceId,
) {
    resolver.text(
        &input[range.clone()],
        SourceSpan {
            id: source_id,
            range,
        },
        SourceMapKind::Copied,
    );
}

fn bounded_tag_end(input: &str, start: usize, limit: usize) -> Option<usize> {
    input[start..]
        .as_bytes()
        .iter()
        .take(limit)
        .position(|byte| *byte == b']')
        .map(|offset| start + offset + 1)
}

fn parse_legacy_icon(tag: &str) -> Option<(&str, Option<f32>)> {
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

fn parse_extended_icon(tag: &str) -> Option<(&str, IconOptions)> {
    let body = tag.strip_prefix(ICON_TAG_OPEN)?.strip_suffix(']')?;
    let mut parts = body.split(' ');
    let id = parts.next()?;
    if !super::rich::is_markup_id(id) {
        return None;
    }
    let mut height = None;
    let mut align = TextIconAlign::Middle;
    if let Some(part) = parts.next() {
        if let Some(size) = part.strip_prefix("size=") {
            height = Some(parse_icon_size(size)?);
            if let Some(part) = parts.next() {
                align = parse_align(part.strip_prefix("align=")?)?;
            }
        } else {
            align = parse_align(part.strip_prefix("align=")?)?;
        }
    }
    parts
        .next()
        .is_none()
        .then_some((id, IconOptions { height, align }))
}

fn parse_align(value: &str) -> Option<TextIconAlign> {
    match value {
        "middle" => Some(TextIconAlign::Middle),
        "baseline" => Some(TextIconAlign::Baseline),
        "top" => Some(TextIconAlign::Top),
        "bottom" => Some(TextIconAlign::Bottom),
        _ => None,
    }
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
        assert_eq!(markup.styles[markup.runs[0].style.0].color, Color::WHITE);
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
        assert_eq!(
            markup.styles[markup.runs[0].style.0].color,
            Color::hex(0xFF000080)
        );
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
        assert_eq!(
            markup.styles[markup.runs[0].style.0].color,
            Color::hex(0xFF0000FF)
        );
        assert_eq!(
            markup.styles[markup.runs[1].style.0].color,
            Color::hex(0x00FF00FF)
        );
        assert_eq!(
            markup.styles[markup.runs[2].style.0].color,
            Color::hex(0xFF0000FF)
        );
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
