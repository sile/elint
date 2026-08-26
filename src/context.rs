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

/// One explored conditional-arm path through the file.
#[derive(Debug)]
pub struct BranchContext {
    /// Side table parallel to `tree.tokens()`: origin and source span of each
    /// token, in the same order as the parse tree's token slice.
    pub source_tokens: Vec<erl_pp::SourceToken>,
    /// Parse forest for the preprocessed token stream.
    pub tree: erl_parse::SyntaxTree,
    /// Preprocessor diagnostics (`-error` / `-warning`, input errors).
    pub preprocess_diagnostics: Vec<PreprocessDiagnostic>,
}

/// Analyzed Erlang file: original source and one analysis per explored
/// conditional-arm path.
#[derive(Debug)]
pub struct Context {
    /// Path used as the preprocessor display name (and for CLI output).
    pub path: PathBuf,
    /// Original file text.
    pub text: String,
    /// Tokens scanned from [`Context::text`], including comments and whitespace.
    pub original_tokens: Vec<erl_tokenize::Token>,
    /// Per-branch analyses. `branches[0]` is the mainline (`Branch::Then`
    /// taken at every conditional).
    pub branches: Vec<BranchContext>,
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
        let branches = preprocess(source)
            .into_iter()
            .map(|(source_tokens, preprocess_diagnostics)| {
                let tokens: Vec<_> = source_tokens.iter().map(|t| *t.token()).collect();
                let tree = erl_parse::parse(tokens, erl_parse::ParseMode::Module);
                BranchContext {
                    source_tokens,
                    tree,
                    preprocess_diagnostics,
                }
            })
            .collect();
        Ok(Self {
            path,
            text,
            original_tokens,
            branches,
        })
    }
}

impl BranchContext {
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
        for i in range {
            let Some(s) = self.span_of_token(i.get()) else {
                continue;
            };
            start = Some(start.map_or(s.start, |t: usize| t.min(s.start)));
            end = Some(end.map_or(s.end, |t: usize| t.max(s.end)));
        }
        Some(Span::new(start?, end?))
    }

    fn span_of_token(&self, index: usize) -> Option<Span> {
        let token = self.source_tokens.get(index)?;
        span_in_original_file(token.origin(), token.source_span())
    }

    /// Returns the `Name/Arity` of the function enclosing `node`, e.g.
    /// `foo/2`, when the node lies inside a [`erl_parse::SyntaxKind::FunctionDecl`].
    ///
    /// Walks `node`'s ancestors to the nearest `FunctionDecl`, reads the
    /// name atom and argument-list arity from its first
    /// [`erl_parse::SyntaxKind::FunctionClause`], and returns `None` when
    /// there is no enclosing function (top-level or attribute context) or
    /// the name cannot be resolved.
    pub fn enclosing_function_name(&self, node: erl_parse::NodeId) -> Option<String> {
        let view = self.tree.view(node)?;
        let decl = view
            .ancestors()
            .find(|a| a.kind() == erl_parse::SyntaxKind::FunctionDecl)?;
        let clause = decl
            .children()
            .find(|c| c.kind() == erl_parse::SyntaxKind::FunctionClause)?;
        let name = clause_name(self, clause)?;
        let arity = clause_arity(clause)?;
        Some(format!("{name}/{arity}"))
    }
}

/// Reads the name atom of a function clause: its first lexical token.
pub(crate) fn clause_name(branch: &BranchContext, node: erl_parse::NodeView<'_>) -> Option<String> {
    node.indexed_tokens().find_map(|(i, _)| {
        let token = branch.source_tokens.get(i.get())?;
        if !token.token().kind().is_lexical() {
            return None;
        }
        match token.value() {
            erl_tokenize::TokenValue::Atom(name) => Some(name.into_owned()),
            _ => None,
        }
    })
}

/// Counts the arguments of a function clause's `ArgumentList`.
pub(crate) fn clause_arity(node: erl_parse::NodeView<'_>) -> Option<u64> {
    let args = node
        .children()
        .find(|c| c.kind() == erl_parse::SyntaxKind::ArgumentList)?;
    Some(args.children().count() as u64)
}

/// One in-progress exploration fork of the preprocessor.
struct Fork {
    pp: erl_pp::Preprocessor,
    /// Number of conditional directives currently open in this fork.
    depth: usize,
    /// Depth at which a non-mainline fork stops (its own `-endif`).
    stop_depth: Option<usize>,
    source_tokens: Vec<erl_pp::SourceToken>,
    diagnostics: Vec<PreprocessDiagnostic>,
}

/// Drives the preprocessor across every conditional arm.
///
/// At each `AwaitingConditional` the fork is cloned into a mainline side
/// (`Branch::Then`) and a non-mainline side (`Branch::Else`). The mainline
/// continues past the `-endif`; non-mainline forks stop at the `-endif`
/// that closes the conditional they were forked from, tracked with
/// [`Fork::depth`] / [`Fork::stop_depth`]. Each source region is therefore
/// scanned by exactly one fork and work stays linear in the input size.
fn preprocess(
    source: erl_pp::Source,
) -> Vec<(Vec<erl_pp::SourceToken>, Vec<PreprocessDiagnostic>)> {
    let mut pending = vec![Fork {
        pp: erl_pp::Preprocessor::new([source]),
        depth: 0,
        stop_depth: None,
        source_tokens: Vec::new(),
        diagnostics: Vec::new(),
    }];
    let mut done = Vec::new();

    while let Some(mut fork) = pending.pop() {
        loop {
            let event = fork.pp.step().expect("preprocessor protocol");
            match event {
                erl_pp::Event::Token(token) => fork.source_tokens.push(token),
                erl_pp::Event::AwaitingInclude(_) => {
                    fork.pp
                        .resume_include(empty_source("<skipped-include>"))
                        .expect("preprocessor protocol");
                }
                erl_pp::Event::AwaitingConditional(conditional) => {
                    let opens_conditional = matches!(
                        conditional,
                        erl_pp::Conditional::Ifdef(_)
                            | erl_pp::Conditional::Ifndef(_)
                            | erl_pp::Conditional::If(_)
                    );
                    let (base_depth, next_depth) = if opens_conditional {
                        (fork.depth, fork.depth + 1)
                    } else {
                        (fork.depth.saturating_sub(1), fork.depth)
                    };
                    let mut main = fork.pp.clone();
                    let mut side = fork.pp.clone();
                    main.resume_conditional(erl_pp::Branch::Then)
                        .expect("preprocessor protocol");
                    side.resume_conditional(erl_pp::Branch::Else)
                        .expect("preprocessor protocol");
                    fork.pp = main;
                    fork.depth = next_depth;
                    pending.push(Fork {
                        pp: side,
                        depth: next_depth,
                        stop_depth: Some(base_depth),
                        source_tokens: Vec::new(),
                        diagnostics: Vec::new(),
                    });
                }
                erl_pp::Event::AwaitingMacroExpansion(_) => {
                    fork.pp
                        .resume_macro_expansion(dummy_macro_source())
                        .expect("preprocessor protocol");
                }
                erl_pp::Event::Diagnostic(_) => {
                    // `-error` / `-warning` directives are deliberately
                    // ignored: every conditional arm is scanned, so these
                    // directives would always be hit, and judging them is
                    // the compiler's job, not a linter's.
                }
                erl_pp::Event::PreprocessError(error) => {
                    fork.diagnostics.push(PreprocessDiagnostic {
                        span: source_span_to_span(error.span()),
                        message: preprocess_error_message(&error),
                    });
                }
                erl_pp::Event::MacroDefined(_) | erl_pp::Event::MacroUndefined(_) => {}
                erl_pp::Event::BranchBoundary(boundary) => {
                    if matches!(boundary, erl_pp::BranchBoundary::Endif { .. }) {
                        fork.depth = fork.depth.saturating_sub(1);
                        if fork.stop_depth.is_some_and(|stop| fork.depth <= stop) {
                            break;
                        }
                    }
                }
                erl_pp::Event::Complete => break,
            }
        }
        done.push((fork.source_tokens, fork.diagnostics));
    }

    done
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

/// Human-readable description of a preprocessor structural error.
fn preprocess_error_message(error: &erl_pp::PreprocessError) -> String {
    match error {
        erl_pp::PreprocessError::ParseUnexpectedToken { expected, .. } => {
            format!("unexpected token in directive; expected {expected}")
        }
        erl_pp::PreprocessError::ParseUnexpectedEof { expected, .. } => {
            format!("unexpected end of file in directive; expected {expected}")
        }
        erl_pp::PreprocessError::DuplicateParameter { name, .. } => {
            format!("duplicate macro parameter: {}", name.as_str())
        }
        erl_pp::PreprocessError::ArityMismatch { name, .. } => {
            format!("macro arity mismatch: {}", name.as_str())
        }
        erl_pp::PreprocessError::UnclosedArgument { .. } => "unclosed macro argument".into(),
        erl_pp::PreprocessError::LeadingEmptyArgument { .. } => {
            "leading empty macro argument".into()
        }
        erl_pp::PreprocessError::TrailingEmptyArgument { .. } => {
            "trailing empty macro argument".into()
        }
        erl_pp::PreprocessError::InvalidStringificationTarget { .. } => {
            "invalid stringification target".into()
        }
        erl_pp::PreprocessError::CircularExpansion { name, .. } => {
            format!("circular macro expansion: {name}")
        }
        erl_pp::PreprocessError::StrayElse { .. } => "stray -else".into(),
        erl_pp::PreprocessError::StrayEndif { .. } => "stray -endif".into(),
        erl_pp::PreprocessError::DoubleElse { .. } => "double -else".into(),
        erl_pp::PreprocessError::UnclosedConditional { .. } => "unclosed conditional".into(),
        erl_pp::PreprocessError::StrayElif { .. } => "stray -elif".into(),
        erl_pp::PreprocessError::ElifAfterElse { .. } => "-elif after -else".into(),
    }
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
    fn analyze_module_yields_mainline_branch() {
        let ctx = analyze_ok("-module(foo).\n");
        let branch = &ctx.branches[0];
        assert!(branch.tree.diagnostics().is_empty());
        assert_eq!(branch.tree.tokens().len(), branch.source_tokens.len());
        assert!(!branch.tree.tokens().is_empty());
        assert_eq!(branch.tree.roots().count(), 1);
    }

    #[test]
    fn unknown_macro_expands_to_dummy_atom_without_diagnostic() {
        let ctx = analyze_ok("-module(foo).\nfoo() -> ?BAR.\n");
        let branch = &ctx.branches[0];
        assert!(branch.preprocess_diagnostics.is_empty());
        assert!(branch.tree.diagnostics().is_empty());
        assert!(branch.source_tokens.iter().any(|token| matches!(
            token.value(),
            erl_tokenize::TokenValue::Atom(name) if name == DUMMY_MACRO_ATOM
        )));
    }

    #[test]
    fn include_is_skipped_without_leaving_the_file() {
        let ctx = analyze_ok("-module(foo).\n-include(\"missing.hrl\").\n");
        let branch = &ctx.branches[0];
        assert!(branch.tree.diagnostics().is_empty());
        // The include directive is consumed by the preprocessor, so only `-module` remains.
        assert_eq!(branch.tree.roots().count(), 1);
    }

    #[test]
    fn tokenize_error_is_returned() {
        let err = Context::analyze("t.erl", "\"unterminated\n".to_string())
            .expect_err("unclosed string must fail");
        assert!(err.position.offset() < 20);
    }

    #[test]
    fn explores_every_arm_and_keeps_mainline_first() {
        let ctx = analyze_ok(
            "\
-module(foo).
-ifdef(A).
foo() -> then_arm.
-else.
foo() -> else_arm.
-endif.
bar() -> ok.
",
        );
        // Then arm and Else arm.
        assert_eq!(ctx.branches.len(), 2);
        let mainline = &ctx.branches[0];
        let mainline_text: String = mainline
            .tree
            .tokens()
            .iter()
            .map(|t| t.text(&ctx.text))
            .collect();
        assert!(mainline_text.contains("then_arm"));
        assert!(mainline_text.contains("bar()"));
        let side_text: String = ctx.branches[1]
            .tree
            .tokens()
            .iter()
            .map(|t| t.text(&ctx.text))
            .collect();
        assert!(side_text.contains("else_arm"));
        assert!(!side_text.contains("bar()"));
        // Every branch's tokens must map back to original-file text.
        for branch in &ctx.branches {
            for (i, _token) in branch.tree.tokens().iter().enumerate() {
                let Some(span) = branch
                    .span_of_range(erl_parse::TokenRange::single(erl_parse::TokenIndex::new(i)))
                else {
                    continue;
                };
                assert!(span.start < span.end);
            }
        }
    }

    #[test]
    fn side_branch_stops_at_its_own_endif() {
        // The Else arm's `case` must be linted, but code after the outer
        // `-endif` must not appear in the side branch (mainline only).
        let ctx = analyze_ok(
            "\
-module(foo).
-ifdef(A).
ok.
-else.
foo() -> ok.
-endif.
bar() -> ok.
",
        );
        let side = &ctx.branches[1];
        let side_text: String = side
            .tree
            .tokens()
            .iter()
            .map(|t| t.text(&ctx.text))
            .collect();
        assert!(side_text.contains("foo()"));
        assert!(!side_text.contains("bar()"));
    }

    #[test]
    fn consecutive_ifdefs_do_not_form_cartesian_product() {
        // With the stop-at-`-endif` rule the number of branches stays
        // linear: 2 independent ifdefs yield 3 forks (mainline plus one
        // side per conditional), not the 4 of a full product.
        let ctx = analyze_ok(
            "\
-module(foo).
-ifdef(A).
a1.
-else.
a2.
-endif.
-ifdef(B).
b1.
-else.
b2.
-endif.
",
        );
        assert_eq!(ctx.branches.len(), 3);
    }

    #[test]
    fn nested_conditionals_explore_inner_arms() {
        // Each arm's nested conditional is explored by the fork that made
        // that arm active, so every marker appears in exactly one branch.
        let ctx = analyze_ok(
            "\
-module(foo).
-ifdef(A).
    -ifdef(B).
    n1.
    -else.
    n2.
    -endif.
-else.
    -ifdef(C).
    n3.
    -else.
    n4.
    -endif.
-endif.
",
        );
        assert_eq!(ctx.branches.len(), 4);
        let marker = |branch: &BranchContext, m: &str| {
            branch.tree.tokens().iter().any(|t| t.text(&ctx.text) == m)
        };
        assert!(marker(&ctx.branches[0], "n1"));
        let mut seen = std::collections::HashSet::new();
        for branch in &ctx.branches {
            let present: Vec<_> = ["n1", "n2", "n3", "n4"]
                .into_iter()
                .filter(|m| marker(branch, m))
                .collect();
            assert_eq!(
                present.len(),
                1,
                "branch must contain exactly one marker: {present:?}"
            );
            seen.insert(present[0]);
        }
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn if_elif_chain_explores_every_arm() {
        let ctx = analyze_ok(
            "\
-module(foo).
-if(true). c1. -elif(false). c2. -elif(true). c3. -else. c4. -endif.
",
        );
        assert_eq!(ctx.branches.len(), 4);
        let mut seen = std::collections::HashSet::new();
        for branch in &ctx.branches {
            let present: Vec<_> = ["c1", "c2", "c3", "c4"]
                .into_iter()
                .filter(|m| branch.tree.tokens().iter().any(|t| t.text(&ctx.text) == *m))
                .collect();
            assert_eq!(
                present.len(),
                1,
                "branch must contain exactly one marker: {present:?}"
            );
            seen.insert(present[0]);
        }
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn findings_in_a_side_branch_are_reachable() {
        let ctx = analyze_ok(
            "\
-module(foo).
-ifdef(A).
ok.
-else.
foo(T) -> element(1, T).
-endif.
",
        );
        let branch = &ctx.branches[1];
        let rule = crate::rules::RULES
            .iter()
            .find(|rule| rule.name == "element_bif")
            .expect("element_bif rule");
        let findings = (rule.check)(&ctx, branch);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].span.text(&ctx.text), "element(1, T)");
    }

    #[test]
    fn enclosing_function_name_resolves_function() {
        let ctx = analyze_ok("-module(t).\nfoo(A, B) ->\n    element(1, A).\n");
        let branch = &ctx.branches[0];
        let call = branch
            .tree
            .nodes()
            .find(|n| n.kind() == erl_parse::SyntaxKind::CallExpr)
            .expect("call expr");
        assert_eq!(
            branch.enclosing_function_name(call.node_id()).as_deref(),
            Some("foo/2")
        );
    }

    #[test]
    fn enclosing_function_name_resolves_for_clause_node() {
        let ctx = analyze_ok("-module(t).\nfoo() -> ok.\n");
        let branch = &ctx.branches[0];
        let clause = branch
            .tree
            .nodes()
            .find(|n| n.kind() == erl_parse::SyntaxKind::FunctionClause)
            .expect("function clause");
        assert_eq!(
            branch.enclosing_function_name(clause.node_id()).as_deref(),
            Some("foo/0")
        );
    }

    #[test]
    fn enclosing_function_name_is_none_outside_function() {
        let ctx = analyze_ok("-module(t).\nfoo() -> ok.\n");
        let branch = &ctx.branches[0];
        let attribute = branch
            .tree
            .nodes()
            .find(|n| n.kind() == erl_parse::SyntaxKind::Attribute)
            .expect("attribute");
        assert_eq!(branch.enclosing_function_name(attribute.node_id()), None);
    }
}
