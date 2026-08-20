use super::rich::RegisteredIcon;
use super::{Color, TextIcons};
use corelib::math::UVec2;
use std::{borrow::Cow, ops::Range};

const COLOR_TAG_OPEN: &str = "[color:#";
const COLOR_TAG_CLOSE: &str = "[/color]";
const ICON_TAG_OPEN: &str = "[icon:";
const OBJECT_REPLACEMENT: char = '\u{FFFC}';
const ZERO_WIDTH_SPACE: char = '\u{200B}';
const MAX_ICON_TAG_LEN: usize = 108;

pub(crate) enum MarkupMode<'a> {
    Colors,
    Rich(&'a TextIcons),
}

pub(crate) struct Markup<'a> {
    pub(crate) text: Cow<'a, str>,
    pub(crate) spans: Vec<MarkupSpan>,
    pub(crate) objects: Vec<InlineObject>,
}

pub(crate) struct MarkupSpan {
    pub(crate) range: Range<usize>,
    pub(crate) color: Color,
    pub(crate) object: Option<usize>,
}

pub(crate) struct InlineObject {
    pub(crate) icon: RegisteredIcon,
    pub(crate) height: Option<f32>,
    pub(crate) color: Color,
}

enum Token<'a> {
    Text(&'a str, Color),
    Icon(RegisteredIcon, Option<f32>, Color),
}

pub(crate) fn plain(input: &str, color: Color, wrap: bool) -> Markup<'_> {
    let text = add_wrap_hints(input, wrap);
    let spans = (!text.is_empty())
        .then(|| MarkupSpan {
            range: 0..text.len(),
            color,
            object: None,
        })
        .into_iter()
        .collect();
    Markup {
        text,
        spans,
        objects: Vec::new(),
    }
}

pub(crate) fn parse(
    input: &str,
    default_color: Color,
    mode: MarkupMode<'_>,
    wrap: bool,
) -> Markup<'static> {
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
                    *colors.last().unwrap(),
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
                *colors.last().unwrap(),
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
        let Some(tag_end) = remaining
            .as_bytes()
            .iter()
            .take(MAX_ICON_TAG_LEN)
            .position(|byte| *byte == b']')
            .map(|offset| tag_start + offset + 1)
        else {
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
        push_text(
            &mut tokens,
            &input[text_start..tag_start],
            *colors.last().unwrap(),
        );
        tokens.push(Token::Icon(icon.clone(), height, *colors.last().unwrap()));
        cursor = tag_end;
        text_start = cursor;
    }
    push_text(&mut tokens, &input[text_start..], *colors.last().unwrap());
    normalize(tokens, wrap)
}

fn push_text<'a>(tokens: &mut Vec<Token<'a>>, text: &'a str, color: Color) {
    if !text.is_empty() {
        tokens.push(Token::Text(text, color));
    }
}

pub(crate) fn add_wrap_hints(text: &str, wrap: bool) -> Cow<'_, str> {
    if !wrap || !text.as_bytes().contains(&b' ') {
        return Cow::Borrowed(text);
    }
    let mut normalized = String::with_capacity(text.len());
    append_text(&mut normalized, text, true);
    Cow::Owned(normalized)
}

fn normalize(tokens: Vec<Token<'_>>, wrap: bool) -> Markup<'static> {
    let mut text = String::new();
    let mut spans: Vec<MarkupSpan> = Vec::new();
    let mut objects = Vec::new();
    for token in tokens {
        let start = text.len();
        let (color, object) = match token {
            Token::Text(span_text, color) => {
                append_text(&mut text, &span_text, wrap);
                (color, None)
            }
            Token::Icon(icon, height, color) => {
                let object = objects.len();
                objects.push(InlineObject {
                    icon,
                    height,
                    color,
                });
                text.push(OBJECT_REPLACEMENT);
                (color, Some(object))
            }
        };
        if start == text.len() {
            continue;
        }
        if let Some(span) = spans.last_mut()
            && span.object.is_none()
            && object.is_none()
            && span.color == color
            && span.range.end == start
        {
            span.range.end = text.len();
        } else {
            spans.push(MarkupSpan {
                range: start..text.len(),
                color,
                object,
            });
        }
    }
    Markup {
        text: Cow::Owned(text),
        spans,
        objects,
    }
}

fn append_text(output: &mut String, text: &str, wrap: bool) {
    for character in text.chars() {
        output.push(character);
        if wrap && character == ' ' {
            output.push(ZERO_WIDTH_SPACE);
        }
    }
}

fn parse_icon_tag(tag: &str) -> Option<(&str, Option<f32>)> {
    let body = tag.strip_prefix(ICON_TAG_OPEN)?.strip_suffix(']')?;
    let (id, height) = match body.split_once(" size=") {
        Some((id, size)) => (id, Some(parse_icon_size(size)?)),
        None => (body, None),
    };
    if !super::rich::is_icon_id(id) || body.matches(" size=").count() > 1 {
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

#[inline]
fn try_parse_open_tag(s: &str) -> Option<(Color, usize)> {
    let close_bracket = s.find(']')?;
    Some((parse_hex_color(&s[..close_bracket]), close_bracket + 1))
}

#[inline]
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
        assert_eq!(markup.spans.len(), 1);
        assert_eq!(markup.spans[0].color, Color::WHITE);
        assert_eq!(&markup.text[markup.spans[0].range.clone()], "Hello 世界");
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
        assert_eq!(markup.spans[0].color, Color::hex(0xFF000080));
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
        assert_eq!(markup.spans.len(), 3);
        assert_eq!(markup.spans[0].color, Color::hex(0xFF0000FF));
        assert_eq!(markup.spans[1].color, Color::hex(0x00FF00FF));
        assert_eq!(markup.spans[2].color, Color::hex(0xFF0000FF));
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
