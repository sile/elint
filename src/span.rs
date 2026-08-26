//! Byte range in the original source file.

/// Half-open byte range `[start, end)` in the original source text.
///
/// Tree positions use `erl_parse` token ranges. Findings, `-elint_expect`,
/// and CLI line/column reporting map those ranges down to this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl Span {
    /// A range that does not point at any source location.
    pub const ZERO: Self = Self::new(0, 0);

    /// Creates a half-open range.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the slice of `full_text` covered by this range.
    pub fn text(self, full_text: &str) -> &str {
        &full_text[self.start..self.end]
    }

    /// Returns whether `other` lies entirely inside this range.
    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

/// A lint finding: a byte range plus the syntax node it originates from.
///
/// The owned [`erl_parse::NodeId`] (no lifetime) lets callers resolve
/// context around the finding, such as the enclosing function name,
/// after the rule's `check` has returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Finding {
    /// Byte range in the original source file.
    pub span: Span,
    /// The syntax node the finding originates from.
    pub node: erl_parse::NodeId,
}
