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

pub(crate) fn plain(input: &str, color: Color) -> SemanticDocument<'_> {
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

    for candidate in scan_candidates(input, ScanPolicy::Legacy) {
        let tag_start = candidate.range.start;
        if tag_start < cursor {
            continue;
        }
        match candidate.kind {
            CandidateKind::ColorOpen if let Some(tag_end) = candidate.tag_end => {
                let tag = &input[tag_start..tag_end];
                let after_prefix = &tag[COLOR_TAG_OPEN.len()..];
                let color = parse_hex_color(&after_prefix[..after_prefix.len() - 1]);
                append_markup_text(
                    &mut resolver,
                    input,
                    text_start..tag_start,
                    TextSourceId::DEFAULT,
                );
                resolver.push_color(color);
                colors += 1;
                cursor = tag_end;
                text_start = cursor;
            }
            CandidateKind::ColorClose => {
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
                cursor = candidate.range.end;
                text_start = cursor;
            }
            CandidateKind::Icon(icon) if rich => {
                let Some(tag_end) = candidate
                    .tag_end
                    .filter(|end| *end - tag_start <= MAX_ICON_TAG_LEN)
                else {
                    debug_warn("Malformed text icon tag");
                    break;
                };
                cursor = tag_end;
                let Some(icon) = icon else {
                    debug_warn("Malformed text icon tag");
                    continue;
                };
                let Some(height) = icon.legacy_height else {
                    debug_warn("Malformed text icon tag");
                    continue;
                };
                if !icon.id.valid {
                    debug_warn("Unknown or invalid text icon");
                    continue;
                }
                let source = SourceSpan {
                    id: TextSourceId::DEFAULT,
                    range: tag_start..tag_end,
                };
                let options = IconOptions {
                    height,
                    align: TextIconAlign::Middle,
                };
                if resolver.validate_icon(icon.id.value, options).is_err() {
                    debug_warn("Unknown or invalid text icon");
                    continue;
                }
                append_markup_text(
                    &mut resolver,
                    input,
                    text_start..tag_start,
                    TextSourceId::DEFAULT,
                );
                resolver
                    .icon(icon.id.value, options, source)
                    .expect("validated legacy icon");
                text_start = cursor;
            }
            _ => {}
        }
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
    OpenStyle {
        id: Range<usize>,
        valid: bool,
    },
    CloseStyle,
    OpenUnderline,
    CloseUnderline,
    OpenStrikethrough,
    CloseStrikethrough,
    OpenEffect {
        id: Range<usize>,
        valid: bool,
    },
    CloseEffect,
    Icon {
        id: Range<usize>,
        options: IconOptions,
    },
    Escape,
}

impl EventKind {
    fn range_name(&self) -> Option<&'static str> {
        match self {
            Self::OpenColor(_) | Self::CloseColor => Some("color"),
            Self::OpenStyle { .. } | Self::CloseStyle => Some("style"),
            Self::OpenUnderline | Self::CloseUnderline => Some("u"),
            Self::OpenStrikethrough | Self::CloseStrikethrough => Some("s"),
            Self::OpenEffect { .. } | Self::CloseEffect => Some("effect"),
            _ => None,
        }
    }

    fn is_open(&self) -> bool {
        matches!(
            self,
            Self::OpenColor(_)
                | Self::OpenStyle { .. }
                | Self::OpenUnderline
                | Self::OpenStrikethrough
                | Self::OpenEffect { .. }
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
    pair: Option<usize>,
}

#[derive(Clone, Copy)]
struct Identifier<'a> {
    value: &'a str,
    valid: bool,
}

#[derive(Clone, Copy)]
struct IconCandidate<'a> {
    id: Identifier<'a>,
    legacy_height: Option<Option<f32>>,
    extended: Option<IconOptions>,
}

#[derive(Clone, Copy)]
enum CandidateKind<'a> {
    Escape,
    ColorOpen,
    ColorClose,
    StyleOpen(Identifier<'a>),
    StyleClose,
    UnderlineOpen,
    UnderlineClose,
    StrikethroughOpen,
    StrikethroughClose,
    EffectOpen(Identifier<'a>),
    EffectClose,
    Icon(Option<IconCandidate<'a>>),
    Reserved,
    Other,
}

struct Candidate<'a> {
    range: Range<usize>,
    tag_end: Option<usize>,
    kind: CandidateKind<'a>,
}

#[derive(Clone, Copy)]
enum ScanPolicy {
    Legacy,
    Extended,
}

struct CandidateScanner<'a> {
    input: &'a str,
    cursor: usize,
    policy: ScanPolicy,
}

impl<'a> CandidateScanner<'a> {
    fn new(input: &'a str, policy: ScanPolicy) -> Self {
        Self {
            input,
            cursor: 0,
            policy,
        }
    }

    fn bounded_end(&self, start: usize, limit: usize) -> Option<usize> {
        self.input[start..]
            .as_bytes()
            .iter()
            .take(limit)
            .position(|byte| *byte == b']')
            .map(|offset| start + offset + 1)
    }

    fn advance(&mut self, start: usize, end: Option<usize>, kind: CandidateKind<'_>) {
        self.cursor = match (self.policy, end, kind) {
            (ScanPolicy::Extended, Some(end), _) if end - start <= MAX_EXTENDED_TAG_LEN => end,
            (ScanPolicy::Extended, None, kind) if !matches!(kind, CandidateKind::Other) => {
                bounded_tag_cursor(self.input, start)
            }
            (
                ScanPolicy::Legacy,
                Some(end),
                CandidateKind::ColorOpen | CandidateKind::ColorClose,
            ) => end,
            _ => start + 1,
        };
    }
}

impl<'a> Iterator for CandidateScanner<'a> {
    type Item = Candidate<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.input[self.cursor..]
            .find('[')
            .map(|offset| self.cursor + offset)?;
        let remaining = &self.input[start..];
        if remaining.starts_with("[[") {
            self.cursor = match self.policy {
                ScanPolicy::Legacy => start + 1,
                ScanPolicy::Extended => start + 2,
            };
            return Some(Candidate {
                range: start..start + 2,
                tag_end: Some(start + 2),
                kind: CandidateKind::Escape,
            });
        }
        let fixed = if remaining.starts_with(COLOR_TAG_CLOSE) {
            Some(CandidateKind::ColorClose)
        } else if remaining.starts_with(STYLE_TAG_CLOSE) {
            Some(CandidateKind::StyleClose)
        } else if remaining.starts_with(UNDERLINE_TAG_OPEN) {
            Some(CandidateKind::UnderlineOpen)
        } else if remaining.starts_with(UNDERLINE_TAG_CLOSE) {
            Some(CandidateKind::UnderlineClose)
        } else if remaining.starts_with(STRIKETHROUGH_TAG_OPEN) {
            Some(CandidateKind::StrikethroughOpen)
        } else if remaining.starts_with(STRIKETHROUGH_TAG_CLOSE) {
            Some(CandidateKind::StrikethroughClose)
        } else if remaining.starts_with(EFFECT_TAG_CLOSE) {
            Some(CandidateKind::EffectClose)
        } else {
            None
        };
        if let Some(kind) = fixed {
            let end = match kind {
                CandidateKind::ColorClose => start + COLOR_TAG_CLOSE.len(),
                CandidateKind::StyleClose => start + STYLE_TAG_CLOSE.len(),
                CandidateKind::UnderlineOpen => start + UNDERLINE_TAG_OPEN.len(),
                CandidateKind::UnderlineClose => start + UNDERLINE_TAG_CLOSE.len(),
                CandidateKind::StrikethroughOpen => start + STRIKETHROUGH_TAG_OPEN.len(),
                CandidateKind::StrikethroughClose => start + STRIKETHROUGH_TAG_CLOSE.len(),
                CandidateKind::EffectClose => start + EFFECT_TAG_CLOSE.len(),
                _ => unreachable!(),
            };
            self.advance(start, Some(end), kind);
            return Some(Candidate {
                range: start..end,
                tag_end: Some(end),
                kind,
            });
        }

        let mut kind = if remaining.starts_with(COLOR_TAG_OPEN) {
            CandidateKind::ColorOpen
        } else if remaining.starts_with(STYLE_TAG_OPEN) {
            CandidateKind::StyleOpen(Identifier {
                value: "",
                valid: false,
            })
        } else if remaining.starts_with(EFFECT_TAG_OPEN) {
            CandidateKind::EffectOpen(Identifier {
                value: "",
                valid: false,
            })
        } else if remaining.starts_with(ICON_TAG_OPEN) {
            CandidateKind::Icon(None)
        } else if remaining.starts_with("[color")
            || remaining.starts_with("[/color")
            || remaining.starts_with("[style")
            || remaining.starts_with("[/style")
            || remaining.starts_with("[icon")
            || malformed_effect_prefix(remaining)
        {
            CandidateKind::Reserved
        } else {
            CandidateKind::Other
        };
        let color = matches!(kind, CandidateKind::ColorOpen);
        let limit = if color && matches!(self.policy, ScanPolicy::Legacy) {
            self.input.len() - start
        } else if matches!(kind, CandidateKind::Icon(_))
            && matches!(self.policy, ScanPolicy::Legacy)
        {
            MAX_ICON_TAG_LEN
        } else {
            MAX_EXTENDED_TAG_LEN
        };
        let end = self.bounded_end(start, limit);
        let Some(end) = end else {
            self.advance(start, None, kind);
            return Some(Candidate {
                range: start..self.input.len(),
                tag_end: None,
                kind,
            });
        };
        let tag = &self.input[start..end];
        kind = match kind {
            CandidateKind::StyleOpen(_) => CandidateKind::StyleOpen(identifier(
                tag.strip_prefix(STYLE_TAG_OPEN)
                    .and_then(|id| id.strip_suffix(']'))
                    .unwrap_or_default(),
            )),
            CandidateKind::EffectOpen(_) => CandidateKind::EffectOpen(identifier(
                tag.strip_prefix(EFFECT_TAG_OPEN)
                    .and_then(|id| id.strip_suffix(']'))
                    .unwrap_or_default(),
            )),
            CandidateKind::Icon(_) => CandidateKind::Icon(parse_icon_candidate(tag)),
            kind => kind,
        };
        self.advance(start, Some(end), kind);
        Some(Candidate {
            range: start..end,
            tag_end: Some(end),
            kind,
        })
    }
}

fn scan_candidates(input: &str, policy: ScanPolicy) -> CandidateScanner<'_> {
    CandidateScanner::new(input, policy)
}

fn identifier_range(start: usize, prefix: &str, id: &str) -> Range<usize> {
    let start = start + prefix.len();
    start..start + id.len()
}

fn malformed_effect_prefix(remaining: &str) -> bool {
    let suffix = remaining
        .strip_prefix("[effect")
        .or_else(|| remaining.strip_prefix("[/effect"));
    suffix.is_some_and(|suffix| {
        !matches!(
            suffix.as_bytes().first(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
        )
    })
}

fn identifier(value: &str) -> Identifier<'_> {
    Identifier {
        value,
        valid: super::rich::is_markup_id(value),
    }
}

fn parse_icon_candidate(tag: &str) -> Option<IconCandidate<'_>> {
    let body = tag.strip_prefix(ICON_TAG_OPEN)?.strip_suffix(']')?;
    let (id, attributes) = body
        .split_once(' ')
        .map_or((body, None), |(id, attributes)| (id, Some(attributes)));

    let legacy_height = match attributes {
        None => Some(None),
        Some(attribute) => attribute
            .strip_prefix("size=")
            .and_then(parse_icon_size)
            .map(Some),
    };
    let extended = parse_extended_icon_attributes(attributes)
        .map(|(height, align)| IconOptions { height, align });
    Some(IconCandidate {
        id: identifier(id),
        legacy_height,
        extended,
    })
}

fn parse_extended_icon_attributes(
    attributes: Option<&str>,
) -> Option<(Option<f32>, TextIconAlign)> {
    let Some(attributes) = attributes else {
        return Some((None, TextIconAlign::Middle));
    };
    let mut attributes = attributes.split(' ');
    let first = attributes.next()?;
    let (height, align) = if let Some(size) = first.strip_prefix("size=") {
        let height = Some(parse_icon_size(size)?);
        let align = match attributes.next() {
            Some(attribute) => parse_align(attribute.strip_prefix("align=")?)?,
            None => TextIconAlign::Middle,
        };
        (height, align)
    } else {
        (None, parse_align(first.strip_prefix("align=")?)?)
    };
    attributes.next().is_none().then_some((height, align))
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
    validate_grapheme_boundaries(input, &mut events, source_id, &mut resolver);

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
            EventKind::OpenStyle { id, valid } => {
                let id = &input[id];
                let active = valid && resolver.try_push_style(id);
                if !active {
                    let code = if valid {
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
            EventKind::OpenEffect { id, valid } => {
                let id = &input[id];
                let active = valid && resolver.push_effect(id).is_ok();
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
            EventKind::Icon { id, options } => {
                if let Err(error) = resolver.icon(&input[id], options, source.clone()) {
                    let code = if error.contains("size") {
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
    let mut events = Vec::new();
    for candidate in scan_candidates(input, ScanPolicy::Extended) {
        if matches!(candidate.kind, CandidateKind::Escape) {
            events.push(Event {
                range: candidate.range,
                kind: EventKind::Escape,
                active: true,
                pair: None,
            });
            continue;
        }
        let start = candidate.range.start;
        let Some(end) = candidate.tag_end else {
            if !matches!(candidate.kind, CandidateKind::Other) {
                diagnose_tag_limit(resolver, source_id, input, start);
            }
            continue;
        };
        if end - start > MAX_EXTENDED_TAG_LEN {
            diagnose_tag_limit(resolver, source_id, input, start);
            continue;
        }
        let tag = &input[start..end];
        let kind = match candidate.kind {
            CandidateKind::ColorOpen => tag
                .strip_prefix(COLOR_TAG_OPEN)
                .and_then(|body| body.strip_suffix(']'))
                .map(|body| EventKind::OpenColor(parse_hex_color(body))),
            CandidateKind::ColorClose if tag == COLOR_TAG_CLOSE => Some(EventKind::CloseColor),
            CandidateKind::StyleOpen(id) => Some(EventKind::OpenStyle {
                id: identifier_range(start, STYLE_TAG_OPEN, id.value),
                valid: id.valid,
            }),
            CandidateKind::StyleClose if tag == STYLE_TAG_CLOSE => Some(EventKind::CloseStyle),
            CandidateKind::UnderlineOpen if tag == UNDERLINE_TAG_OPEN => {
                Some(EventKind::OpenUnderline)
            }
            CandidateKind::UnderlineClose if tag == UNDERLINE_TAG_CLOSE => {
                Some(EventKind::CloseUnderline)
            }
            CandidateKind::StrikethroughOpen if tag == STRIKETHROUGH_TAG_OPEN => {
                Some(EventKind::OpenStrikethrough)
            }
            CandidateKind::StrikethroughClose if tag == STRIKETHROUGH_TAG_CLOSE => {
                Some(EventKind::CloseStrikethrough)
            }
            CandidateKind::EffectOpen(id) => Some(EventKind::OpenEffect {
                id: identifier_range(start, EFFECT_TAG_OPEN, id.value),
                valid: id.valid,
            }),
            CandidateKind::EffectClose if tag == EFFECT_TAG_CLOSE => Some(EventKind::CloseEffect),
            CandidateKind::Icon(Some(icon)) if icon.id.valid => {
                icon.extended.map(|options| EventKind::Icon {
                    id: identifier_range(start, ICON_TAG_OPEN, icon.id.value),
                    options,
                })
            }
            _ => None,
        };

        if let Some(kind) = kind {
            let active = !kind.is_open() && !kind.is_close();
            events.push(Event {
                range: start..end,
                kind,
                active,
                pair: None,
            });
        } else if let CandidateKind::Icon(Some(icon)) = candidate.kind {
            resolver.diagnostic(
                if icon.id.valid {
                    TextDiagnosticCode::MalformedTag
                } else {
                    TextDiagnosticCode::InvalidIdentifier
                },
                SourceSpan {
                    id: source_id,
                    range: start..end,
                },
                None,
            );
        } else if matches!(candidate.kind, CandidateKind::Reserved) {
            resolver.diagnostic(
                TextDiagnosticCode::MalformedTag,
                SourceSpan {
                    id: source_id,
                    range: start..end,
                },
                None,
            );
        }
    }
    events
}

fn bounded_tag_cursor(input: &str, start: usize) -> usize {
    let mut end = input.len().min(start + MAX_EXTENDED_TAG_LEN);
    while end > start && !input.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn diagnose_tag_limit(
    resolver: &mut SemanticResolver<'_>,
    source_id: TextSourceId,
    input: &str,
    start: usize,
) {
    let end = bounded_tag_cursor(input, start);
    resolver.diagnostic(
        TextDiagnosticCode::TagLengthLimitExceeded,
        SourceSpan {
            id: source_id,
            range: start..end,
        },
        None,
    );
}

fn validate_grapheme_boundaries(
    input: &str,
    events: &mut [Event],
    source_id: TextSourceId,
    resolver: &mut SemanticResolver<'_>,
) {
    let mut provisional = String::with_capacity(input.len());
    let mut positions = vec![None; events.len()];
    let mut cursor = 0;
    for (index, event) in events.iter().enumerate().filter(|(_, event)| event.active) {
        provisional.push_str(&input[cursor..event.range.start]);
        let start = provisional.len();
        if matches!(event.kind, EventKind::Escape) {
            provisional.push('[');
        }
        positions[index] = Some(start..provisional.len());
        cursor = event.range.end;
    }
    provisional.push_str(&input[cursor..]);

    let mut boundaries = vec![false; provisional.len() + 1];
    boundaries[provisional.len()] = true;
    for (index, _) in provisional.grapheme_indices(true) {
        boundaries[index] = true;
    }

    for index in 0..events.len() {
        let Some(position) = &positions[index] else {
            continue;
        };
        if !events[index].active || (boundaries[position.start] && boundaries[position.end]) {
            continue;
        }
        diagnose_event(
            resolver,
            TextDiagnosticCode::GraphemeBoundary,
            source_id,
            &events[index],
        );
        events[index].active = false;
        if let Some(pair) = events[index].pair {
            events[pair].active = false;
        }
    }
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
            events[open_index].pair = Some(index);
            events[index].active = true;
            events[index].pair = Some(open_index);
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
            let markup = parse(&input, Color::WHITE, MarkupMode::Rich(&icons));
            assert_eq!(markup.text, input);
            assert!(markup.objects.is_empty());
        }
    }

    #[test]
    fn plain_text_is_one_span() {
        let markup = parse("Hello 世界", Color::WHITE, MarkupMode::Colors);
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
        let markup = parse("[icon:soul] text", Color::WHITE, MarkupMode::Colors);
        assert_eq!(markup.text, "[icon:soul] text");
        assert!(markup.objects.is_empty());
    }

    #[test]
    fn malformed_icon_is_literal() {
        let icons = TextIcons::default();
        let input = "[icon:soul  size=20]";
        let markup = parse(input, Color::WHITE, MarkupMode::Rich(&icons));
        assert_eq!(markup.text, input);
        assert!(markup.objects.is_empty());
    }

    #[test]
    fn unknown_icon_is_literal() {
        let icons = TextIcons::default();
        let input = "[icon:unknown]";
        let markup = parse(input, Color::WHITE, MarkupMode::Rich(&icons));
        assert_eq!(markup.text, input);
        assert!(markup.objects.is_empty());
    }
}
