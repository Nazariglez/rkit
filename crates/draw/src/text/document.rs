use super::{Color, Font, rich::RegisteredIcon};
use std::{borrow::Cow, ops::Range};

const DIAGNOSTIC_LIMIT: usize = 64;

pub(crate) struct SemanticDocument<'a> {
    pub(crate) text: Cow<'a, str>,
    pub(crate) runs: Vec<StyleRun>,
    pub(crate) objects: Vec<InlineObject>,
    pub(crate) source_map: SourceMap,
    pub(crate) diagnostics: Vec<TextDiagnostic>,
}

pub(crate) struct StyleRun {
    pub(crate) range: Range<usize>,
    pub(crate) color: Color,
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
}

pub(crate) struct InlineObject {
    pub(crate) at: usize,
    pub(crate) source: Range<usize>,
    pub(crate) icon: RegisteredIcon,
    pub(crate) height: Option<f32>,
    pub(crate) color: Color,
    pub(crate) font: Option<Font>,
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
}

#[derive(Default)]
pub(crate) struct SourceMap {
    pub(crate) segments: Vec<SourceMapSegment>,
}

pub(crate) struct SourceMapSegment {
    pub(crate) source: Range<usize>,
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
