//! Rustc-style rendering of diagnostics for the CLI.

use std::io::{IsTerminal, Write};
use std::path::Path;

use unicode_width::UnicodeWidthChar;

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

    /// Returns the 1-based line number and 1-based display column of
    /// `offset`.
    pub fn line_col(&self, text: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(text.len());
        let idx = self
            .starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line = idx + 1;
        let column = display_width(&text[self.starts[idx]..offset]) + 1;
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

    /// Blue `note:` label.
    pub fn note(self, s: &str) -> String {
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

/// Exact token endpoints of a multi-line span, used to place the carets on
/// the start and end lines.
///
/// When available, the start-line caret covers the span's first token and
/// the end-line caret covers its last token, instead of approximating from
/// whitespace.
#[derive(Debug, Clone, Copy)]
pub struct Endpoints {
    /// Byte range of the span's first token.
    pub first: Span,
    /// Byte range of the span's last token.
    pub last: Span,
}

/// Renders one rustc-style diagnostic block:
///
/// ```text
/// error[code]: message
///   --> path:line:col (in foo/2)
///    |
///    | previous line, shown as context when it exists
/// 10 |     element(1, T)
///    |     ^^^^^^^^^^^^
///    |
/// note: a follow-up line when provided
/// ```
///
/// The caret spans `span` within the first line it touches. When the
/// reported line is not the first line, the immediately preceding line is
/// shown without a line number as context (an empty preceding line is
/// omitted). `enclosing`, when present, is the enclosing function name
/// appended to the location. Tabs are expanded to four columns in the
/// source and caret lines so the caret stays aligned.
///
/// When `span` crosses lines, the end line is shown after an ellipsis
/// (`...|`) with its own caret. `endpoints`, when present, anchors the
/// start caret on the first token and the end caret on the last token;
/// without it the end caret is derived by scanning back from `span.end`.
#[allow(clippy::too_many_arguments)]
pub fn render(
    w: &mut impl Write,
    color: Color,
    source: &Source<'_>,
    code: Option<&str>,
    message: &str,
    span: Span,
    endpoints: Option<Endpoints>,
    enclosing: Option<&str>,
    note: Option<&str>,
) -> std::io::Result<()> {
    let (line, column) = source.index.line_col(source.text, span.start);
    let (end_line, _) = source.index.line_col(source.text, span.end);
    let width = line.to_string().len().max(end_line.to_string().len());
    let bar = format!("{} |", " ".repeat(width));
    let gutter = format!("{} | ", " ".repeat(width));

    let header = match code {
        Some(code) => format!(
            "{}[{}]: {}",
            color.error("error"),
            color.code(code),
            message
        ),
        None => format!("{}: {}", color.error("error"), message),
    };
    let location = format!("{}:{}:{}", source.path.display(), line, column);
    let location = match enclosing {
        Some(name) => format!("{location} (in {})", color.note(name)),
        None => location,
    };
    writeln!(w, "{header}")?;
    writeln!(w, "{}{} {location}", " ".repeat(width), color.arrow("-->"))?;
    writeln!(w, "{bar}")?;

    if line > 1 {
        let (prev_start, prev_end) = source.index.line_range(source.text, line - 2);
        let prev_text = &source.text[prev_start..prev_end];
        if !prev_text.is_empty() {
            writeln!(w, "{gutter}{}", expand_tabs(prev_text))?;
        }
    }

    let (line_start, line_end) = source.index.line_range(source.text, line - 1);
    let line_text = &source.text[line_start..line_end];
    let line_number = color.line_number(&format!("{line:>width$}"));
    writeln!(w, "{line_number} | {}", expand_tabs(line_text))?;

    let span_end = span.end.min(line_end);
    let (visual, caret_len) = if end_line != line {
        match endpoints {
            Some(ep) => {
                let caret_end = ep.first.end.min(line_end);
                (
                    display_width(&source.text[line_start..ep.first.start]),
                    display_width(&source.text[ep.first.start..caret_end]).max(1),
                )
            }
            None => (
                display_width(&source.text[line_start..span.start]),
                display_width(&source.text[span.start..span_end]).max(1),
            ),
        }
    } else {
        (
            display_width(&source.text[line_start..span.start]),
            display_width(&source.text[span.start..span_end]).max(1),
        )
    };
    let caret = color.caret(&"^".repeat(caret_len));
    writeln!(w, "{gutter}{}{caret}", " ".repeat(visual))?;

    if end_line != line {
        writeln!(w, "{}|", ".".repeat(width))?;
        if end_line > 1 {
            let (prev_start, prev_end) = source.index.line_range(source.text, end_line - 2);
            let prev_text = &source.text[prev_start..prev_end];
            if !prev_text.is_empty() {
                writeln!(w, "{gutter}{}", expand_tabs(prev_text))?;
            }
        }
        let (end_start, end_end) = source.index.line_range(source.text, end_line - 1);
        let end_text = &source.text[end_start..end_end];
        let end_number = color.line_number(&format!("{end_line:>width$}"));
        writeln!(w, "{end_number} | {}", expand_tabs(end_text))?;
        let (end_visual, end_caret_len) = match endpoints {
            Some(ep) => {
                let caret_start = ep.last.start.max(end_start);
                let caret_end = ep.last.end.min(end_end);
                (
                    display_width(&source.text[end_start..caret_start]),
                    display_width(&source.text[caret_start..caret_end]).max(1),
                )
            }
            None => {
                let mut caret_end = span.end.min(end_end);
                while caret_end > end_start {
                    let c = source.text[..caret_end]
                        .chars()
                        .next_back()
                        .expect("non-empty slice");
                    if !c.is_whitespace() {
                        break;
                    }
                    caret_end -= c.len_utf8();
                }
                let mut caret_start = caret_end;
                while caret_start > end_start {
                    let c = source.text[..caret_start]
                        .chars()
                        .next_back()
                        .expect("non-empty slice");
                    if c.is_whitespace() {
                        break;
                    }
                    caret_start -= c.len_utf8();
                }
                (
                    display_width(&source.text[end_start..caret_start]),
                    display_width(&source.text[caret_start..caret_end]).max(1),
                )
            }
        };
        let end_caret = color.caret(&"^".repeat(end_caret_len));
        writeln!(w, "{gutter}{}{end_caret}", " ".repeat(end_visual))?;
    }

    writeln!(w, "{bar}")?;

    if let Some(note) = note {
        writeln!(w, "{} {note}", color.note("note:"))?;
    }

    Ok(())
}

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| {
            if c == '\t' {
                4
            } else {
                UnicodeWidthChar::width(c).unwrap_or(0)
            }
        })
        .sum()
}

fn expand_tabs(s: &str) -> String {
    s.replace('\t', "    ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(
        text: &str,
        span: Span,
        code: Option<&str>,
        message: &str,
        note: Option<&str>,
    ) -> String {
        let source = Source::new(Path::new("t.erl"), text);
        let mut out = Vec::new();
        render(
            &mut out,
            Color { enabled: false },
            &source,
            code,
            message,
            span,
            None,
            None,
            note,
        )
        .expect("write to Vec");
        String::from_utf8(out).expect("utf8")
    }

    #[test]
    fn renders_single_line_span() {
        let text = "-module(t).\nfoo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(
            text,
            Span::new(start, start + 2),
            Some("newline_after_arrow"),
            "summary",
            None,
        );
        assert_eq!(
            out,
            "\
error[newline_after_arrow]: summary
 --> t.erl:2:10
  |
  | -module(t).
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
        let out = render_to_string(text, Span::new(start, start + 2), None, "message", None);
        assert_eq!(
            out,
            "\
error: message
  --> t.erl:11:10
   |
   | j
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
        let out = render_to_string(text, Span::new(start, start + 2), None, "message", None);
        // Tab becomes four columns, so the caret sits at column 5.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:2:5
  |
  | foo() ->
2 |     ok.
  |     ^^
  |
"
        );
    }

    #[test]
    fn expands_tabs_in_preceding_line() {
        let text = "\tpre\n\tok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), None, "message", None);
        // The tab in the preceding line expands to four columns, matching
        // the error line's gutter alignment.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:2:5
  |
  |     pre
2 |     ok.
  |     ^^
  |
"
        );
    }

    #[test]
    fn omits_empty_preceding_line() {
        let text = "a\n\nfoo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), None, "message", None);
        // The blank line before the error line contributes no context.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:3:10
  |
3 | foo() -> ok.
  |          ^^
  |
"
        );
    }

    #[test]
    fn first_line_error_shows_no_preceding_line() {
        let text = "foo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(text, Span::new(start, start + 2), None, "message", None);
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:1:10
  |
1 | foo() -> ok.
  |          ^^
  |
"
        );
    }

    #[test]
    fn aligns_caret_after_wide_characters() {
        let text = "foo(\"おはよう\", X)\n";
        let start = text.find('X').expect("finding");
        let out = render_to_string(text, Span::new(start, start + 1), None, "message", None);
        // `おはよう` is four wide characters (8 columns), so the caret is
        // at display column 17.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:1:17
  |
1 | foo(\"おはよう\", X)
  |                 ^
  |
"
        );
    }

    #[test]
    fn aligns_caret_after_combining_characters() {
        let text = "caf\u{301} X\n";
        let start = text.find('X').expect("finding");
        let out = render_to_string(text, Span::new(start, start + 1), None, "message", None);
        // The combining acute accent has display width 0, so the caret is
        // at display column 5.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:1:5
  |
1 | caf\u{301} X
  |     ^
  |
"
        );
    }

    #[test]
    fn aligns_caret_with_tab_and_wide_characters() {
        let text = "\tおX\n";
        let start = text.find('X').expect("finding");
        let out = render_to_string(text, Span::new(start, start + 1), None, "message", None);
        // Tab expands to four columns and `お` is two columns wide, so the
        // caret is at display column 7.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:1:7
  |
1 |     おX
  |       ^
  |
"
        );
    }

    #[test]
    fn clamps_empty_span_to_one_caret() {
        let text = "-module(t).\n";
        let out = render_to_string(text, Span::new(0, 0), None, "message", None);
        assert!(out.contains("^\n"), "{out:?}");
    }

    #[test]
    fn renders_note_after_block() {
        let text = "foo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let out = render_to_string(
            text,
            Span::new(start, start + 1),
            Some("r"),
            "m",
            Some("see `elint explain r`"),
        );
        assert_eq!(
            out,
            "\
error[r]: m
 --> t.erl:1:10
  |
1 | foo() -> ok.
  |          ^
  |
note: see `elint explain r`
"
        );
    }

    #[test]
    fn shows_enclosing_function_name() {
        let text = "foo() -> ok.\n";
        let start = text.find("ok.").expect("finding");
        let source = Source::new(Path::new("t.erl"), text);
        let mut out = Vec::new();
        render(
            &mut out,
            Color { enabled: false },
            &source,
            None,
            "message",
            Span::new(start, start + 2),
            None,
            Some("foo/0"),
            None,
        )
        .expect("write to Vec");
        let out = String::from_utf8(out).expect("utf8");
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:1:10 (in foo/0)
  |
1 | foo() -> ok.
  |          ^^
  |
"
        );
    }

    #[test]
    fn shows_multiline_span_head_and_tail() {
        let text = "foo() ->\n    case A of\n        a ->\n            ok\n    end.\n";
        let start = text.find("case").expect("finding");
        let end = text.find("end.").expect("finding") + 3;
        let endpoints = Some(Endpoints {
            first: Span::new(start, start + 4),
            last: Span::new(end - 3, end),
        });
        let source = Source::new(Path::new("t.erl"), text);
        let mut out = Vec::new();
        render(
            &mut out,
            Color { enabled: false },
            &source,
            None,
            "message",
            Span::new(start, end),
            endpoints,
            None,
            None,
        )
        .expect("write to Vec");
        let out = String::from_utf8(out).expect("utf8");
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:2:5
  |
  | foo() ->
2 |     case A of
  |     ^^^^
.|
  |             ok
5 |     end.
  |     ^^^
  |
"
        );
    }

    #[test]
    fn shows_multiline_span_tail_without_endpoints() {
        let text = "foo() ->\n    case A of\n        a ->\n            ok\n    end.\n";
        let start = text.find("case").expect("finding");
        let end = text.find("end.").expect("finding") + 3;
        let out = render_to_string(text, Span::new(start, end), None, "message", None);
        // Without endpoints the start caret runs to the end of the line and
        // the end caret is derived by scanning back from span.end.
        assert_eq!(
            out,
            "\
error: message
 --> t.erl:2:5
  |
  | foo() ->
2 |     case A of
  |     ^^^^^^^^^
.|
  |             ok
5 |     end.
  |     ^^^
  |
"
        );
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
            None,
            None,
            Some("note"),
        )
        .expect("write");
        let out = String::from_utf8(out).expect("utf8");
        assert!(out.contains("\x1b[1;31merror\x1b[0m"), "{out:?}");
        assert!(out.contains("\x1b[36mr\x1b[0m"), "{out:?}");
        assert!(out.contains("\x1b[34mnote:\x1b[0m"), "{out:?}");
    }
}
