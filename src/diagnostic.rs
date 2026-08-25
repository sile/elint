//! Rustc-style rendering of diagnostics for the CLI.

use std::io::{IsTerminal, Write};
use std::path::Path;

use crate::Span;

/// Maps byte offsets in a source text to line / column positions.
#[derive(Debug)]
pub struct LineIndex {
    /// Byte offset of the start of each line.
    starts: Vec<usize>,
}

impl LineIndex {
    /// Builds an index from `text`.
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// Returns the 1-based line number and 1-based character column of
    /// `offset`.
    pub fn line_col(&self, text: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(text.len());
        let idx = self
            .starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line = idx + 1;
        let column = text[self.starts[idx]..offset].chars().count() + 1;
        (line, column)
    }

    /// Byte range `[start, end)` of the 0-based `line`, excluding the
    /// trailing newline.
    pub fn line_range(&self, text: &str, line: usize) -> (usize, usize) {
        let start = self.starts[line];
        let end = self
            .starts
            .get(line + 1)
            .map_or(text.len(), |&next| next.saturating_sub(1));
        (start, end)
    }
}

/// A source file plus its [`LineIndex`], ready for diagnostic rendering.
#[derive(Debug)]
pub struct Source<'a> {
    path: &'a Path,
    text: &'a str,
    index: LineIndex,
}

impl<'a> Source<'a> {
    /// Builds a renderable source from a file path and its text.
    pub fn new(path: &'a Path, text: &'a str) -> Self {
        Self {
            path,
            text,
            index: LineIndex::new(text),
        }
    }
}

/// Controls whether ANSI color codes are emitted.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    enabled: bool,
}

impl Color {
    /// Detects from the stderr terminal and the `NO_COLOR` environment
    /// variable.
    pub fn detect() -> Self {
        let enabled = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        Self { enabled }
    }

    fn paint(self, code: &str, s: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    /// Bold red `error` label.
    pub fn error(self, s: &str) -> String {
        self.paint("1;31", s)
    }

    /// Cyan code (`[rule]`).
    pub fn code(self, s: &str) -> String {
        self.paint("36", s)
    }

    /// Blue `-->` arrow.
    pub fn arrow(self, s: &str) -> String {
        self.paint("34", s)
    }

    /// Bold cyan line number.
    pub fn line_number(self, s: &str) -> String {
        self.paint("1;36", s)
    }

    /// Bold red caret.
    pub fn caret(self, s: &str) -> String {
        self.paint("1;31", s)
    }
}

/// Renders one rustc-style diagnostic block:
///
/// ```text
/// error[code]: message
///   --> path:line:col
///    |
/// 10 |     element(1, T)
///    |     ^^^^^^^^^^^^
///    |
/// ```
///
/// The caret spans `span` within the first line it touches. Tabs are
/// expanded to four columns in the source and caret lines so the caret
/// stays aligned.
pub fn render(
    w: &mut impl Write,
    color: Color,
    source: &Source<'_>,
    code: Option<&str>,
    message: &str,
    span: Span,
) -> std::io::Result<()> {
    let (line, column) = source.index.line_col(source.text, span.start);
    let width = line.to_string().len();
    let bar = format!("{} |", " ".repeat(width));
    let gutter = format!("{} | ", " ".repeat(width));

    let header = match code {
        Some(code) => format!("{}[{}]: {}", color.error("error"), color.code(code), message),
        None => format!("{}: {}", color.error("error"), message),
    };
    let location = format!("{}:{}:{}", source.path.display(), line, column);
    writeln!(w, "{header}")?;
    writeln!(w, "{}{} {location}", " ".repeat(width), color.arrow("-->"))?;
    writeln!(w, "{bar}")?;

    let (line_start, line_end) = source.index.line_range(source.text, line - 1);
    let line_text = &source.text[line_start..line_end];
    let line_number = color.line_number(&format!("{line:>width$}"));
    writeln!(w, "{line_number} | {}", expand_tabs(line_text))?;

    let caret_start = source.text[line_start..span.start].chars().count();
    let span_end = span.end.min(line_end);
    let caret_len = source.text[span.start..span_end].chars().count().max(1);
    let visual = line_text
        .chars()
        .take(caret_start)
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum::<usize>();
    let caret = color.caret(&"^".repeat(caret_len));
    writeln!(w, "{gutter}{}{caret}", " ".repeat(visual))?;
    writeln!(w, "{bar}")?;

    Ok(())
}

fn expand_tabs(s: &str) -> String {
    s.replace('\t', "    ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(text: &str, span: Span, code: Option<&str>, message: &str) -> String {
        let source = Source::new(Path::new("t.erl"), text);
        let mut out = Vec::new();
        render(
            &mut out,
            Color { enabled: false },
            &source,
            code,
            message,
            span,
        )
        .expect("write to Vec");
        String::from_utf8(out).expect("utf8")
    }

    #[test]
    fn renders_single_line_span() {
        let text = "-module(t).\nfoo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), Some("newline_after_arrow"), "summary");
        assert_eq!(
            out,
            "\
error[newline_after_arrow]: summary
 --> t.erl:2:10
  |
2 | foo() -> ok.
  |          ^^
  |
"
        );
    }

    #[test]
    fn pads_line_numbers_to_width() {
        let text = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nfoo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), None, "message");
        assert_eq!(
            out,
            "\
error: message
  --> t.erl:11:10
   |
11 | foo() -> ok.
   |          ^^
   |
"
        );
    }

    #[test]
    fn expands_tabs_in_caret_alignment() {
        let text = "foo() ->\n\tok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), None, "message");
        // Tab becomes four columns, so the caret sits at column 5.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:2:2
  |
2 |     ok.
  |     ^^
  |
"
        );
    }

    #[test]
    fn clamps_empty_span_to_one_caret() {
        let text = "-module(t).\n";
        let out = render_to_string(text, Span::new(0, 0), None, "message");
        assert!(out.contains("^\n"), "{out:?}");
    }

    #[test]
    fn color_codes_are_emitted_when_enabled() {
        let text = "foo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let source = Source::new(Path::new("t.erl"), text);
        let mut out = Vec::new();
        render(
            &mut out,
            Color { enabled: true },
            &source,
            Some("r"),
            "m",
            Span::new(start, start + 1),
        )
        .expect("write");
        let out = String::from_utf8(out).expect("utf8");
        assert!(out.contains("\x1b[1;31merror\x1b[0m"), "{out:?}");
        assert!(out.contains("\x1b[36mr\x1b[0m"), "{out:?}");
    }
}
