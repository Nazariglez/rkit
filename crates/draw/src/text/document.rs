use super::{
    Color, Font, TextEffects, TextIcons, TextStyles,
    effect::EffectCallback,
    rich::{RegisteredIcon, RichTextIcon, TextIconAlign, is_markup_id},
    style::TextStyle,
};
use std::{borrow::Cow, ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

const DIAGNOSTIC_LIMIT: usize = 64;

pub(crate) struct SemanticDocument<'a> {
    pub(crate) text: Cow<'a, str>,
    pub(crate) styles: Vec<ResolvedStyle>,
    pub(crate) runs: Vec<StyleRun>,
    pub(crate) objects: Vec<InlineObject>,
    pub(crate) source_map: SourceMap,
    pub(crate) diagnostics: DiagnosticSink,
    pub(crate) effects: Vec<Arc<EffectCallback>>,
}

#[derive(Clone)]
pub(crate) struct ResolvedStyle {
    pub(crate) color: Color,
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedStyleId(pub(crate) usize);

pub(crate) struct StyleRun {
    pub(crate) range: Range<usize>,
    pub(crate) style: ResolvedStyleId,
    pub(crate) effects: Vec<u32>,
}

#[derive(Clone, Copy)]
pub(crate) struct IconOptions {
    pub(crate) height: Option<f32>,
    pub(crate) align: TextIconAlign,
}

pub(crate) struct InlineObject {
    pub(crate) at: usize,
    pub(crate) source: SourceSpan,
    pub(crate) icon: RegisteredIcon,
    pub(crate) options: IconOptions,
    pub(crate) style: ResolvedStyleId,
    pub(crate) effects: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceSpan {
    pub(crate) id: TextSourceId,
    pub(crate) range: Range<usize>,
}

#[derive(Default)]
pub(crate) struct SourceMap {
    pub(crate) segments: Vec<SourceMapSegment>,
}

pub(crate) struct SourceMapSegment {
    pub(crate) source: SourceSpan,
    pub(crate) semantic: Range<usize>,
    pub(crate) kind: SourceMapKind,
}

#[derive(Clone, Copy)]
pub(crate) enum SourceMapKind {
    Copied,
    EscapedOpen,
    InlineObject(usize),
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextSourceId(u64);

impl TextSourceId {
    pub const DEFAULT: Self = Self(0);

    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextMarkupPolicy {
    Lenient,
    Strict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextDiagnosticCode {
    UnknownIconId,
    UnknownStyleId,
    UnknownEffectId,
    InvalidIdentifier,
    MalformedTag,
    UnmatchedClosingTag,
    MismatchedClosingTag,
    CrossedRange,
    UnclosedRange,
    NestingLimitExceeded,
    TagLengthLimitExceeded,
    DiagnosticLimitExceeded,
    GraphemeBoundary,
    ClusterBoundaryExpanded,
    ClusterStyleConflict,
    CoordinateMappingFailure,
}

#[derive(Clone, Debug)]
pub struct TextDiagnostic {
    code: TextDiagnosticCode,
    severity: TextDiagnosticSeverity,
    source_id: TextSourceId,
    range: Range<usize>,
    related_range: Option<Range<usize>>,
}

impl TextDiagnostic {
    pub fn code(&self) -> TextDiagnosticCode {
        self.code
    }

    pub fn severity(&self) -> TextDiagnosticSeverity {
        self.severity
    }

    pub fn source_id(&self) -> TextSourceId {
        self.source_id
    }

    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    pub fn related_range(&self) -> Option<&Range<usize>> {
        self.related_range.as_ref()
    }
}

#[derive(Default)]
pub(crate) struct DiagnosticSink {
    diagnostics: Vec<TextDiagnostic>,
    truncated: bool,
}

impl DiagnosticSink {
    pub(crate) fn push(
        &mut self,
        code: TextDiagnosticCode,
        severity: TextDiagnosticSeverity,
        source_id: TextSourceId,
        range: Range<usize>,
        related_range: Option<Range<usize>>,
    ) {
        if self.diagnostics.len() < DIAGNOSTIC_LIMIT - 1 {
            self.diagnostics.push(TextDiagnostic {
                code,
                severity,
                source_id,
                range,
                related_range,
            });
        } else if !self.truncated {
            self.truncated = true;
            self.diagnostics.push(TextDiagnostic {
                code: TextDiagnosticCode::DiagnosticLimitExceeded,
                severity: TextDiagnosticSeverity::Error,
                source_id,
                range,
                related_range: None,
            });
        }
    }

    pub(crate) fn finish(self) -> Vec<TextDiagnostic> {
        self.diagnostics
    }

    pub(crate) fn error(
        &mut self,
        code: TextDiagnosticCode,
        source: &SourceSpan,
        related_range: Option<Range<usize>>,
    ) {
        self.push(
            code,
            TextDiagnosticSeverity::Error,
            source.id,
            source.range.clone(),
            related_range,
        );
    }
}

/// Owned rich content whose text is always literal.
///
/// Named styles and icons are resolved only when the document is laid out. Source and style
/// scopes are balanced by their closures, and completed layouts do not borrow the document.
#[derive(Default)]
pub struct RichTextDocument {
    ops: Vec<DocumentOp>,
    content: usize,
}

#[derive(Clone)]
enum DocumentOp {
    Text(String),
    EnterSource(TextSourceId),
    LeaveSource,
    EnterStyle(String),
    LeaveStyle,
    EnterEffect(String),
    LeaveEffect,
    Icon(RichTextIcon),
}

impl RichTextDocument {
    pub const fn new() -> Self {
        Self {
            ops: Vec::new(),
            content: 0,
        }
    }

    pub fn clear(&mut self) {
        self.ops.clear();
        self.content = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.content == 0
    }

    /// Appends copied literal text without parsing markup or escape syntax.
    pub fn text(&mut self, text: &str) -> &mut Self {
        if text.is_empty() {
            return self;
        }
        if let Some(DocumentOp::Text(current)) = self.ops.last_mut() {
            current.push_str(text);
        } else {
            self.ops.push(DocumentOp::Text(text.to_owned()));
        }
        self.content += text.len();
        self
    }

    /// Appends content associated with a source ID.
    pub fn source<F>(&mut self, source_id: TextSourceId, write: F) -> &mut Self
    where
        F: FnOnce(&mut RichTextDocument),
    {
        let mut body = Self::new();
        write(&mut body);
        self.ops.push(DocumentOp::EnterSource(source_id));
        self.ops.append(&mut body.ops);
        self.ops.push(DocumentOp::LeaveSource);
        self.content += body.content;
        self
    }

    /// Appends content using a named style resolved during layout.
    pub fn style<I, F>(&mut self, id: I, write: F) -> &mut Self
    where
        I: Into<String>,
        F: FnOnce(&mut RichTextDocument),
    {
        let mut body = Self::new();
        write(&mut body);
        self.ops.push(DocumentOp::EnterStyle(id.into()));
        self.ops.append(&mut body.ops);
        self.ops.push(DocumentOp::LeaveStyle);
        self.content += body.content;
        self
    }

    pub fn effect<I, F>(&mut self, id: I, write: F) -> &mut Self
    where
        I: Into<String>,
        F: FnOnce(&mut RichTextDocument),
    {
        let mut body = Self::new();
        write(&mut body);
        self.ops.push(DocumentOp::EnterEffect(id.into()));
        self.ops.append(&mut body.ops);
        self.ops.push(DocumentOp::LeaveEffect);
        self.content += body.content;
        self
    }

    /// Appends one unresolved Sprite icon occurrence.
    pub fn icon<I>(&mut self, icon: I) -> &mut Self
    where
        I: Into<RichTextIcon>,
    {
        self.ops.push(DocumentOp::Icon(icon.into()));
        self.content += 1;
        self
    }
}

#[derive(Clone)]
struct ComputedStyle {
    color: Color,
    font: Option<Font>,
    size: Option<f32>,
    line_height: Option<f32>,
    underline: bool,
    strikethrough: bool,
}

impl ComputedStyle {
    fn patched(&self, patch: &TextStyle) -> Self {
        Self {
            color: patch.color.unwrap_or(self.color),
            font: patch.font.clone().or_else(|| self.font.clone()),
            size: patch.size.or(self.size),
            line_height: patch.line_height.or(self.line_height),
            underline: patch.underline.unwrap_or(self.underline),
            strikethrough: patch.strikethrough.unwrap_or(self.strikethrough),
        }
    }
}

pub(crate) struct SemanticResolver<'a> {
    icons: Option<&'a TextIcons>,
    styles: Option<&'a TextStyles>,
    text: String,
    resolved_styles: Vec<ResolvedStyle>,
    runs: Vec<StyleRun>,
    objects: Vec<InlineObject>,
    source_map: SourceMap,
    style: ComputedStyle,
    style_stack: Vec<ComputedStyle>,
    source: TextSourceId,
    source_stack: Vec<TextSourceId>,
    diagnostics: DiagnosticSink,
    effects: Option<&'a TextEffects>,
    effect_callbacks: Vec<Arc<EffectCallback>>,
    effect_stack: Vec<u32>,
}

impl<'a> SemanticResolver<'a> {
    pub(crate) fn new(
        color: Color,
        icons: Option<&'a TextIcons>,
        styles: Option<&'a TextStyles>,
        effects: Option<&'a TextEffects>,
    ) -> Self {
        Self {
            icons,
            styles,
            text: String::new(),
            resolved_styles: Vec::new(),
            runs: Vec::new(),
            objects: Vec::new(),
            source_map: SourceMap::default(),
            style: ComputedStyle {
                color,
                font: None,
                size: None,
                line_height: None,
                underline: false,
                strikethrough: false,
            },
            style_stack: Vec::new(),
            source: TextSourceId::DEFAULT,
            source_stack: Vec::new(),
            diagnostics: DiagnosticSink::default(),
            effects,
            effect_callbacks: Vec::new(),
            effect_stack: Vec::new(),
        }
    }

    pub(crate) fn set_source(&mut self, source: TextSourceId) {
        self.source = source;
    }

    pub(crate) fn push_source(&mut self, source: TextSourceId) {
        self.source_stack.push(self.source);
        self.source = source;
    }

    pub(crate) fn pop_source(&mut self) -> Result<(), String> {
        self.source = self
            .source_stack
            .pop()
            .ok_or_else(|| "Text source scope is unbalanced".to_string())?;
        Ok(())
    }

    pub(crate) fn push_color(&mut self, color: Color) {
        self.style_stack.push(self.style.clone());
        self.style.color = color;
    }

    pub(crate) fn push_underline(&mut self) {
        self.style_stack.push(self.style.clone());
        self.style.underline = true;
    }

    pub(crate) fn push_strikethrough(&mut self) {
        self.style_stack.push(self.style.clone());
        self.style.strikethrough = true;
    }

    pub(crate) fn push_style(&mut self, id: &str) -> Result<(), String> {
        if !is_markup_id(id) {
            return Err(format!("Invalid text style ID '{id}'"));
        }
        let style = self
            .styles
            .ok_or_else(|| {
                "Rich text document references a style without a style registry".to_string()
            })?
            .get(id)
            .ok_or_else(|| format!("Unknown text style ID '{id}'"))?;
        self.style_stack.push(self.style.clone());
        self.style = self.style.patched(style);
        Ok(())
    }

    pub(crate) fn try_push_style(&mut self, id: &str) -> bool {
        let Some(style) = self.styles.and_then(|styles| styles.get(id)) else {
            return false;
        };
        self.style_stack.push(self.style.clone());
        self.style = self.style.patched(style);
        true
    }

    pub(crate) fn push_effect(&mut self, id: &str) -> Result<(), String> {
        if !is_markup_id(id) {
            return Err(format!("Invalid text effect ID '{id}'"));
        }
        let effect = self
            .effects
            .and_then(|effects| effects.get(id))
            .ok_or_else(|| format!("Unknown text effect ID '{id}'"))?;
        let index = u32::try_from(self.effect_callbacks.len())
            .map_err(|_| "Text effect occurrence count exceeds limits")?;
        self.effect_callbacks.push(effect.0.clone());
        self.effect_stack.push(index);
        Ok(())
    }

    pub(crate) fn pop_effect(&mut self) -> Result<(), String> {
        self.effect_stack
            .pop()
            .ok_or_else(|| "Text effect scope is unbalanced".to_string())?;
        Ok(())
    }

    pub(crate) fn pop_style(&mut self) -> Result<(), String> {
        self.style = self
            .style_stack
            .pop()
            .ok_or_else(|| "Text style scope is unbalanced".to_string())?;
        Ok(())
    }

    fn style_id(&mut self) -> ResolvedStyleId {
        let resolved = ResolvedStyle {
            color: self.style.color,
            font: self.style.font.clone(),
            size: self.style.size,
            line_height: self.style.line_height,
            underline: self.style.underline,
            strikethrough: self.style.strikethrough,
        };
        if let Some(index) = self
            .resolved_styles
            .iter()
            .position(|style| same_style(style, &resolved))
        {
            return ResolvedStyleId(index);
        }
        let id = ResolvedStyleId(self.resolved_styles.len());
        self.resolved_styles.push(resolved);
        id
    }

    pub(crate) fn text(&mut self, text: &str, source: SourceSpan, kind: SourceMapKind) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        let style = self.style_id();
        self.runs.push(StyleRun {
            range: start..self.text.len(),
            style,
            effects: self.effect_stack.clone(),
        });
        self.source_map.segments.push(SourceMapSegment {
            source,
            semantic: start..self.text.len(),
            kind,
        });
    }

    fn resolve_icon(&self, id: &str, options: IconOptions) -> Result<RegisteredIcon, String> {
        if !is_markup_id(id) {
            return Err(format!("Invalid text icon ID '{id}'"));
        }
        if options
            .height
            .is_some_and(|height| !height.is_finite() || height <= 0.0)
        {
            return Err(format!(
                "Text icon '{id}' size must be finite and greater than zero"
            ));
        }
        let icon = self
            .icons
            .ok_or_else(|| {
                "Rich text document references an icon without an icon registry".to_string()
            })?
            .registered(id)
            .ok_or_else(|| format!("Unknown text icon ID '{id}'"))?;
        if let Some(height) = options.height {
            let size = icon.source_size();
            let ratio = size.x as f32 / size.y as f32;
            let width = height * ratio;
            if !ratio.is_finite() || ratio <= 0.0 || !width.is_finite() || width <= 0.0 {
                return Err(format!("Text icon '{id}' size produces an invalid width"));
            }
        }
        Ok(icon.clone())
    }

    pub(crate) fn validate_icon(&self, id: &str, options: IconOptions) -> Result<(), String> {
        self.resolve_icon(id, options).map(|_| ())
    }

    pub(crate) fn icon(
        &mut self,
        id: &str,
        options: IconOptions,
        source: SourceSpan,
    ) -> Result<(), String> {
        let icon = self.resolve_icon(id, options)?;
        let style = self.style_id();
        let object = self.objects.len();
        let at = self.text.len();
        self.objects.push(InlineObject {
            at,
            source: source.clone(),
            icon,
            options,
            style,
            effects: self.effect_stack.clone(),
        });
        self.source_map.segments.push(SourceMapSegment {
            source,
            semantic: at..at,
            kind: SourceMapKind::InlineObject(object),
        });
        Ok(())
    }

    pub(crate) fn diagnostic(
        &mut self,
        code: TextDiagnosticCode,
        source: SourceSpan,
        related_range: Option<Range<usize>>,
    ) {
        self.diagnostics.push(
            code,
            TextDiagnosticSeverity::Error,
            source.id,
            source.range,
            related_range,
        );
    }

    pub(crate) fn finish(mut self) -> SemanticDocument<'static> {
        merge_runs(&mut self.runs, &self.resolved_styles);
        SemanticDocument {
            text: Cow::Owned(self.text),
            styles: self.resolved_styles,
            runs: self.runs,
            objects: self.objects,
            source_map: self.source_map,
            diagnostics: self.diagnostics,
            effects: self.effect_callbacks,
        }
    }
}

fn merge_runs(runs: &mut Vec<StyleRun>, styles: &[ResolvedStyle]) {
    let mut merged: Vec<StyleRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.range.end == run.range.start
            && previous.effects == run.effects
            && same_style(&styles[previous.style.0], &styles[run.style.0])
        {
            previous.range.end = run.range.end;
        } else {
            merged.push(run);
        }
    }
    *runs = merged;
}

fn same_style(left: &ResolvedStyle, right: &ResolvedStyle) -> bool {
    left.color == right.color
        && left.font.as_ref().map(Font::id) == right.font.as_ref().map(Font::id)
        && left.size == right.size
        && left.line_height == right.line_height
        && left.underline == right.underline
        && left.strikethrough == right.strikethrough
}

pub(crate) fn resolve_document(
    document: &RichTextDocument,
    color: Color,
    icons: Option<&TextIcons>,
    styles: Option<&TextStyles>,
    effects: Option<&TextEffects>,
) -> Result<SemanticDocument<'static>, String> {
    let mut resolver = SemanticResolver::new(color, icons, styles, effects);
    let mut boundaries = Vec::new();
    for op in &document.ops {
        match op {
            DocumentOp::Text(text) => {
                let start = resolver.text.len();
                resolver.text(
                    text,
                    SourceSpan {
                        id: resolver.source,
                        range: start..start + text.len(),
                    },
                    SourceMapKind::Copied,
                );
            }
            DocumentOp::EnterSource(source) => {
                boundaries.push(resolver.text.len());
                resolver.push_source(*source);
            }
            DocumentOp::LeaveSource => {
                boundaries.push(resolver.text.len());
                resolver.pop_source()?;
            }
            DocumentOp::EnterStyle(id) => {
                boundaries.push(resolver.text.len());
                resolver.push_style(id)?;
            }
            DocumentOp::LeaveStyle => {
                boundaries.push(resolver.text.len());
                resolver.pop_style()?;
            }
            DocumentOp::EnterEffect(id) => {
                boundaries.push(resolver.text.len());
                resolver.push_effect(id)?;
            }
            DocumentOp::LeaveEffect => {
                boundaries.push(resolver.text.len());
                resolver.pop_effect()?;
            }
            DocumentOp::Icon(icon) => {
                let at = resolver.text.len();
                resolver.icon(
                    &icon.id,
                    IconOptions {
                        height: icon.height,
                        align: icon.align,
                    },
                    SourceSpan {
                        id: resolver.source,
                        range: at..at,
                    },
                )?;
            }
        }
    }
    let grapheme_boundaries: Vec<_> = resolver
        .text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(resolver.text.len()))
        .collect();
    if boundaries
        .into_iter()
        .any(|boundary| grapheme_boundaries.binary_search(&boundary).is_err())
    {
        return Err("Rich text document scope splits a grapheme cluster".into());
    }
    Ok(resolver.finish())
}

pub(crate) fn strict_error(diagnostics: &[TextDiagnostic]) -> Option<String> {
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == TextDiagnosticSeverity::Error)
        .map(|diagnostic| {
            let related = diagnostic
                .related_range
                .as_ref()
                .map(|range| format!(", related {}..{}", range.start, range.end))
                .unwrap_or_default();
            format!(
                "{:?} at source {:?} bytes {}..{}{}",
                diagnostic.code,
                diagnostic.source_id,
                diagnostic.range.start,
                diagnostic.range.end,
                related
            )
        })
        .collect();
    (!errors.is_empty()).then(|| errors.join("\n"))
}
