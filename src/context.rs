//! Per-file tokenize / preprocess / parse pipeline.

use std::path::{Path, PathBuf};

use crate::Span;

/// Diagnostic recorded while driving the preprocessor.
#[derive(Debug, Clone)]
pub struct PreprocessDiagnostic {
    /// Location in the original file when it can be mapped; otherwise [`Span::ZERO`].
    pub span: Span,
    /// Human-readable description.
    pub message: String,
}

/// Analyzed Erlang file: original source, preprocessed tokens, tree, and diagnostics.
#[derive(Debug)]
pub struct Context {
    /// Path used as the preprocessor display name (and for CLI output).
    pub path: PathBuf,
    /// Original file text.
    pub text: String,
    /// Tokens scanned from [`Context::text`], including comments and whitespace.
    pub original_tokens: Vec<erl_tokenize::Token>,
    /// Lexical tokens after preprocessing, in the order passed to `erl_parse`.
    pub tokens: Vec<erl_tokenize::Token>,
    /// Side table parallel to [`Context::tokens`]: origin and source span of each token.
    pub source_tokens: Vec<erl_pp::SourceToken>,
    /// Parse forest for the preprocessed token stream.
    pub tree: erl_parse::SyntaxTree,
    /// Preprocessor diagnostics (unknown macros, `-error` / `-warning`, input errors).
    pub preprocess_diagnostics: Vec<PreprocessDiagnostic>,
}

impl Context {
    /// Tokenizes, preprocesses, and parses `text` as a module.
    ///
    /// Returns [`erl_tokenize::Error`] when the original file cannot be scanned.
    /// Preprocessor protocol mistakes panic: they are driver bugs.
    pub fn analyze<P: AsRef<Path>>(path: P, text: String) -> Result<Self, erl_tokenize::Error> {
        let path = path.as_ref().to_path_buf();
        let original_tokens = erl_tokenize::scan_tokens(&text)?;
        let source = erl_pp::Source::new(
            path.to_string_lossy().as_ref(),
            text.clone(),
            original_tokens.clone(),
        );
        let (source_tokens, preprocess_diagnostics) = preprocess(source);
        let tokens: Vec<_> = source_tokens.iter().map(|t| *t.token()).collect();
        let tree = erl_parse::parse(&tokens, erl_parse::ParseMode::Module);
        Ok(Self {
            path,
            text,
            original_tokens,
            tokens,
            source_tokens,
            tree,
            preprocess_diagnostics,
        })
    }

    /// Maps a token-buffer range to a byte range in the original file.
    ///
    /// Empty ranges (missing tokens / EOF) map to the end of the preceding
    /// original-file token, or to offset 0 when there is no preceding token.
    pub fn span_of_range(&self, range: erl_parse::TokenRange) -> Option<Span> {
        if range.is_empty() {
            let i = range.start().get();
            if i == 0 {
                return Some(Span::new(0, 0));
            }
            return self.span_of_token(i - 1).map(|s| Span::new(s.end, s.end));
        }

        let mut start = None;
        let mut end = None;
        for i in range.as_range() {
            let Some(s) = self.span_of_token(i) else {
                continue;
            };
            start = Some(start.map_or(s.start, |t: usize| t.min(s.start)));
            end = Some(end.map_or(s.end, |t: usize| t.max(s.end)));
        }
        Some(Span::new(start?, end?))
    }

    /// Original-file byte range of every syntax node in the forest, including nested calls.
    pub fn syntax_spans(&self) -> Vec<Span> {
        let mut spans = Vec::new();
        for root in self.tree.roots() {
            push_node_spans(self, root, &mut spans);
        }
        spans
    }

    fn span_of_token(&self, index: usize) -> Option<Span> {
        let token = self.source_tokens.get(index)?;
        span_in_original_file(token.origin(), token.source_span())
    }
}

fn push_node_spans(ctx: &Context, node: erl_parse::NodeView<'_>, spans: &mut Vec<Span>) {
    if let Some(span) = ctx.span_of_range(node.range()) {
        spans.push(span);
    }
    for child in node.children() {
        push_node_spans(ctx, child, spans);
    }
}

fn preprocess(source: erl_pp::Source) -> (Vec<erl_pp::SourceToken>, Vec<PreprocessDiagnostic>) {
    let mut preprocessor = erl_pp::Preprocessor::new([source]);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    loop {
        match preprocessor.step().expect("preprocessor protocol") {
            erl_pp::Event::Token(token) => tokens.push(token),
            erl_pp::Event::AwaitingInclude(_) => {
                preprocessor
                    .resume_include(empty_source("<skipped-include>"))
                    .expect("preprocessor protocol");
            }
            erl_pp::Event::AwaitingConditional(conditional) => {
                let branch = match conditional {
                    erl_pp::Conditional::Ifdef(defined) | erl_pp::Conditional::Ifndef(defined) => {
                        defined.recommended
                    }
                    erl_pp::Conditional::If(_) | erl_pp::Conditional::Elif(_) => {
                        erl_pp::Branch::Then
                    }
                };
                preprocessor
                    .resume_conditional(branch)
                    .expect("preprocessor protocol");
            }
            erl_pp::Event::AwaitingMacroExpansion(_call) => {
                preprocessor
                    .resume_macro_expansion(dummy_macro_source())
                    .expect("preprocessor protocol");
            }
            erl_pp::Event::Diagnostic(diagnostic) => {
                diagnostics.push(PreprocessDiagnostic {
                    span: source_span_to_span(diagnostic.directive_span),
                    message: format!("{:?}", diagnostic.severity),
                });
            }
            erl_pp::Event::PreprocessError(error) => {
                diagnostics.push(PreprocessDiagnostic {
                    span: source_span_to_span(error.span()),
                    message: format!("{error:?}"),
                });
            }
            erl_pp::Event::MacroDefined(_)
            | erl_pp::Event::MacroUndefined(_)
            | erl_pp::Event::BranchBoundary(_) => {}
            erl_pp::Event::Complete => break,
        }
    }

    (tokens, diagnostics)
}

/// Atom spliced in for an unknown macro so the file still parses.
/// Lowercase so it scans as an atom (valid wherever a macro's value
/// could appear), unlike efmt's uppercase `EFMT_DUMMY` which is a variable.
const DUMMY_MACRO_ATOM: &str = "elint_dummy";

fn empty_source(name: &str) -> erl_pp::Source {
    erl_pp::Source::new(name, "", Vec::new())
}

fn dummy_macro_source() -> erl_pp::Source {
    erl_pp::Source::new(
        "<elint-dummy-macro>",
        DUMMY_MACRO_ATOM,
        erl_tokenize::scan_tokens(DUMMY_MACRO_ATOM).expect("dummy atom always tokenizes"),
    )
}

fn source_span_to_span(span: erl_pp::SourceSpan) -> Span {
    Span::new(span.start.offset(), span.end.offset())
}

fn span_in_original_file(origin: &erl_pp::Origin, span: erl_pp::SourceSpan) -> Option<Span> {
    match origin {
        erl_pp::Origin::Source => Some(source_span_to_span(span)),
        erl_pp::Origin::Include {
            parent,
            include_site,
            ..
        } => span_in_original_file(parent, *include_site),
        erl_pp::Origin::MacroBody {
            parent, call_site, ..
        }
        | erl_pp::Origin::MacroArgument {
            parent, call_site, ..
        }
        | erl_pp::Origin::Stringification {
            parent, call_site, ..
        }
        | erl_pp::Origin::SourceInfo {
            parent, call_site, ..
        }
        | erl_pp::Origin::CallerExpansion {
            parent, call_site, ..
        } => span_in_original_file(parent, *call_site),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_ok(src: &str) -> Context {
        Context::analyze("t.erl", src.to_string()).expect("test source must scan")
    }

    #[test]
    fn analyze_module_yields_tree_and_side_table() {
        let ctx = analyze_ok("-module(foo).\n");
        assert!(ctx.tree.diagnostics().is_empty());
        assert_eq!(ctx.tokens.len(), ctx.source_tokens.len());
        assert!(!ctx.tokens.is_empty());
        assert_eq!(ctx.tree.roots().count(), 1);
    }

    #[test]
    fn unknown_macro_expands_to_dummy_atom_without_diagnostic() {
        let ctx = analyze_ok("-module(foo).\nfoo() -> ?BAR.\n");
        assert!(ctx.preprocess_diagnostics.is_empty());
        assert!(ctx.tree.diagnostics().is_empty());
        assert!(ctx.source_tokens.iter().any(|token| matches!(
            token.value(),
            erl_tokenize::TokenValue::Atom(name) if name == DUMMY_MACRO_ATOM
        )));
    }

    #[test]
    fn include_is_skipped_without_leaving_the_file() {
        let ctx = analyze_ok("-module(foo).\n-include(\"missing.hrl\").\n");
        assert!(ctx.tree.diagnostics().is_empty());
        // The include directive is consumed by the preprocessor, so only `-module` remains.
        assert_eq!(ctx.tree.roots().count(), 1);
    }

    #[test]
    fn tokenize_error_is_returned() {
        let err = Context::analyze("t.erl", "\"unterminated\n".to_string())
            .expect_err("unclosed string must fail");
        assert!(err.position.offset() < 20);
    }
}
