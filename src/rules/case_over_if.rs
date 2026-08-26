//! `case_over_if` lint: prefer case over if.

use super::Rule;
use crate::{BranchContext, Context, Span};

/// Lint rule that flags every `if` expression.
pub const RULE: Rule = Rule::new(
    "case_over_if",
    include_str!("../../rules/case_over_if/rule.md"),
    check,
);

fn check(_ctx: &Context, branch: &BranchContext) -> Vec<Span> {
    let mut errors = Vec::new();
    for node in branch.tree.nodes() {
        if node.kind() != erl_parse::SyntaxKind::IfExpr {
            continue;
        }
        if let Some(span) = if_keyword_span(branch, node) {
            errors.push(span);
        }
    }
    errors
}

fn if_keyword_span(branch: &BranchContext, node: erl_parse::NodeView<'_>) -> Option<Span> {
    let index = node.range().find(|i| {
        branch.tree.tokens().get(i.get()).is_some_and(|token| {
            matches!(
                token.kind(),
                erl_tokenize::TokenKind::Keyword(erl_tokenize::Keyword::If)
            )
        })
    })?;
    branch.span_of_range(erl_parse::TokenRange::new(
        index,
        erl_parse::TokenIndex::new(index.get() + 1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    #[test]
    fn flags_if_expression() {
        let src = "\
-module(t).
foo(N) ->
    if
        N > 0 ->
            positive;
        true ->
            zero
    end.
";
        assert_eq!(findings(check, src), ["if"]);
    }

    #[test]
    fn flags_every_if_expression() {
        let src = "\
-module(t).
foo(N) ->
    if
        N > 0 ->
            positive;
        true ->
            zero
    end,
    if
        true ->
            ok
    end.
";
        assert_eq!(findings(check, src), ["if", "if"]);
    }

    #[test]
    fn ignores_case() {
        let src = "\
-module(t).
foo(N) ->
    case N of
        _ ->
            ok
    end.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ng_fixture_has_three_findings() {
        let src = include_str!("../../rules/case_over_if/ng.erl");
        assert_eq!(findings(check, src).len(), 3);
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/case_over_if/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
