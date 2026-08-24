//! `newline_after_arrow` lint: require a newline after clause `->`.

use super::Rule;
use crate::{BranchContext, Context, Span};

/// Lint rule that flags a clause `->` with no newline before the body.
pub const RULE: Rule = Rule::new(
    "newline_after_arrow",
    include_str!("../../rules/newline_after_arrow/rule.md"),
    check,
);

fn check(ctx: &Context, branch: &BranchContext) -> Vec<Span> {
    let mut errors = Vec::new();
    for root in branch.tree.roots() {
        walk(ctx, branch, root, &mut errors);
    }
    errors
}

fn walk(
    ctx: &Context,
    branch: &BranchContext,
    node: erl_parse::NodeView<'_>,
    errors: &mut Vec<Span>,
) {
    if is_one_line_fun(ctx, branch, node) {
        return;
    }
    check_node(ctx, branch, node, errors);
    for child in node.children() {
        walk(ctx, branch, child, errors);
    }
}

fn is_one_line_fun(ctx: &Context, branch: &BranchContext, node: erl_parse::NodeView<'_>) -> bool {
    if !matches!(
        node.kind(),
        erl_parse::SyntaxKind::AnonymousFun | erl_parse::SyntaxKind::NamedFun
    ) {
        return false;
    }
    let Some(span) = branch.span_of_range(node.range()) else {
        return false;
    };
    !ctx.text[span.start..span.end].contains('\n')
}

fn check_node(
    ctx: &Context,
    branch: &BranchContext,
    node: erl_parse::NodeView<'_>,
    errors: &mut Vec<Span>,
) {
    if !is_clause_kind(node.kind()) {
        return;
    }
    let Some(body) = node
        .children()
        .find(|c| c.kind() == erl_parse::SyntaxKind::Body)
    else {
        return;
    };
    let Some(body_first) = first_lexical(branch, body.range()) else {
        return;
    };
    let Some(arrow) = prev_lexical(branch, body_first) else {
        return;
    };
    if !is_right_arrow(branch, arrow) {
        return;
    }
    if !is_source_origin(branch, arrow) {
        return;
    }
    let Some(arrow_span) = token_span(branch, arrow) else {
        return;
    };
    let Some(body_span) = token_span(branch, body_first) else {
        return;
    };
    if body_span.start < arrow_span.end {
        return;
    }
    let between = &ctx.text[arrow_span.end..body_span.start];
    if !between.contains('\n') {
        errors.push(arrow_span);
    }
}

fn is_clause_kind(kind: erl_parse::SyntaxKind) -> bool {
    matches!(
        kind,
        erl_parse::SyntaxKind::FunctionClause
            | erl_parse::SyntaxKind::Clause
            | erl_parse::SyntaxKind::IfClause
            | erl_parse::SyntaxKind::CatchClause
            | erl_parse::SyntaxKind::ReceiveAfterSection
    )
}

fn first_lexical(branch: &BranchContext, range: erl_parse::TokenRange) -> Option<erl_parse::TokenIndex> {
    range.as_range().find_map(|i| {
        branch
            .tokens
            .get(i)
            .filter(|t| t.kind().is_lexical())
            .map(|_| erl_parse::TokenIndex::new(i))
    })
}

fn prev_lexical(branch: &BranchContext, before: erl_parse::TokenIndex) -> Option<erl_parse::TokenIndex> {
    (0..before.get()).rev().find_map(|i| {
        branch
            .tokens
            .get(i)
            .filter(|t| t.kind().is_lexical())
            .map(|_| erl_parse::TokenIndex::new(i))
    })
}

fn is_right_arrow(branch: &BranchContext, index: erl_parse::TokenIndex) -> bool {
    branch.tokens.get(index.get()).is_some_and(|token| {
        token.kind() == erl_tokenize::TokenKind::Symbol(erl_tokenize::Symbol::RightArrow)
    })
}

fn is_source_origin(branch: &BranchContext, index: erl_parse::TokenIndex) -> bool {
    branch
        .source_tokens
        .get(index.get())
        .is_some_and(|token| matches!(token.origin(), erl_pp::Origin::Source))
}

fn token_span(
    branch: &BranchContext,
    index: erl_parse::TokenIndex,
) -> Option<Span> {
    branch.span_of_range(erl_parse::TokenRange::new(
        index,
        erl_parse::TokenIndex::new(index.get() + 1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src: &str) -> Vec<String> {
        let ctx = Context::analyze("t.erl", src.to_string()).expect("test source must scan");
        assert!(
            ctx.branches[0].tree.diagnostics().is_empty(),
            "parse diagnostics: {:?}",
            ctx.branches[0].tree.diagnostics()
        );
        check(&ctx, &ctx.branches[0])
            .into_iter()
            .map(|s| s.text(&ctx.text).to_string())
            .collect()
    }

    fn finding_count(src: &str) -> usize {
        findings(src).len()
    }

    #[test]
    fn flags_inline_function_clause() {
        let src = "-module(t).\nfoo() -> ok.\n";
        assert_eq!(findings(src), ["->"]);
    }

    #[test]
    fn flags_inline_case_clause() {
        let src = "-module(t).\nfoo() ->\n    case 1 of 1 -> ok end.\n";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn flags_inline_if_clause() {
        let src = "-module(t).\nfoo() ->\n    if true -> ok end.\n";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn flags_inline_receive_after() {
        let src = "-module(t).\nfoo() ->\n    receive after 0 -> ok end.\n";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn flags_inline_catch_clause() {
        let src = "-module(t).\nfoo() ->\n    try 1 catch _:_ -> ok end.\n";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn ignores_one_line_anon_fun_clause() {
        let src = "-module(t).\nfoo() ->\n    fun() -> ok end.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn ignores_one_line_named_fun_clause() {
        let src = "-module(t).\nfoo() ->\n    fun F() -> ok end.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn flags_multi_line_fun_clause_without_newline() {
        let src = "-module(t).\nfoo() ->\n    fun(X) -> X + 1\n    end.\n";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn flags_nested_inner_clause_once() {
        let src = "\
-module(t).
foo() ->
    case 1 of
        1 ->
            case 2 of
                2 -> ok
            end
    end.
";
        assert_eq!(finding_count(src), 1);
    }

    #[test]
    fn ignores_newline_after_arrow() {
        let src = "-module(t).\nfoo() ->\n    ok.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn ignores_same_line_comment_then_newline() {
        let src = "-module(t).\nfoo() -> % note\n    ok.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn ignores_spec_arrow() {
        let src = "-module(t).\n-spec foo() -> ok.\nfoo() ->\n    ok.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn ignores_map_arrow_and_maybe_match() {
        let src = "\
-module(t).
foo() ->
    _ = #{a => b},
    maybe
        X ?= {ok, 1},
        X
    else
        _ ->
            error
    end.
";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn ignores_macro_expanded_arrow() {
        let src = "-module(t).\n-define(F, f() -> ok).\n?F.\n";
        assert!(findings(src).is_empty());
    }

    #[test]
    fn elint_expect_suppresses_finding() {
        let src = "\
-module(t).
%% ELINT_EXPECT: newline_after_arrow
foo() -> ok.
";
        let ctx = Context::analyze("t.erl", src.to_string()).expect("scan");
        let mut expect = crate::expect::ExpectRules::new(&ctx).expect("expect");
        let spans = check(&ctx, &ctx.branches[0]);
        assert_eq!(spans.len(), 1);
        assert!(expect.handle_error("newline_after_arrow", spans[0]));
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn ng_fixture_has_nine_findings() {
        let src = include_str!("../../rules/newline_after_arrow/ng.erl");
        assert_eq!(finding_count(src), 9);
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/newline_after_arrow/ok.erl");
        assert!(findings(src).is_empty());
    }
}
