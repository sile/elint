//! `strict_generator` lint: require strict comprehension generators.

use super::Rule;
use crate::{BranchContext, Context, Span};

/// Lint rule that flags relaxed comprehension generators.
pub const RULE: Rule = Rule::new(
    "strict_generator",
    include_str!("../../rules/strict_generator/rule.md"),
    check,
);

fn check(_ctx: &Context, branch: &BranchContext) -> Vec<Span> {
    let mut errors = Vec::new();
    for root in branch.tree.roots() {
        walk(branch, root, &mut errors);
    }
    errors
}

fn walk(branch: &BranchContext, node: erl_parse::NodeView<'_>, errors: &mut Vec<Span>) {
    if node.kind() == erl_parse::SyntaxKind::ZipQualifier {
        return;
    }
    check_node(branch, node, errors);
    for child in node.children() {
        walk(branch, child, errors);
    }
}

fn check_node(branch: &BranchContext, node: erl_parse::NodeView<'_>, errors: &mut Vec<Span>) {
    let Some(arrow) = relaxed_arrow(branch, node) else {
        return;
    };
    let Some(span) = branch.span_of_range(erl_parse::TokenRange::new(
        arrow,
        erl_parse::TokenIndex::new(arrow.get() + 1),
    )) else {
        return;
    };
    errors.push(span);
}

fn relaxed_arrow(
    branch: &BranchContext,
    node: erl_parse::NodeView<'_>,
) -> Option<erl_parse::TokenIndex> {
    let expected = match node.kind() {
        erl_parse::SyntaxKind::Generator | erl_parse::SyntaxKind::MapGenerator => {
            erl_tokenize::Symbol::LeftArrow
        }
        erl_parse::SyntaxKind::BitstringGenerator => erl_tokenize::Symbol::DoubleLeftArrow,
        _ => return None,
    };
    node.range().find(|i| {
        branch
            .tree
            .tokens()
            .get(i.get())
            .is_some_and(|token| token.kind() == erl_tokenize::TokenKind::Symbol(expected))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    #[test]
    fn flags_list_generator() {
        let src = "-module(t).\nfoo() -> [X || X <- [1, 2, 3]].\n";
        assert_eq!(findings(check, src), ["<-"]);
    }

    #[test]
    fn flags_bitstring_generator() {
        let src = "-module(t).\nfoo() -> [X || <<X:8>> <= <<1, 2, 3>>].\n";
        assert_eq!(findings(check, src), ["<="]);
    }

    #[test]
    fn flags_map_generator() {
        let src = "-module(t).\nfoo(M) -> #{K => V || K := V <- M}.\n";
        assert_eq!(findings(check, src), ["<-"]);
    }

    #[test]
    fn ignores_strict_list_generator() {
        let src = "-module(t).\nfoo() -> [X || X <:- [1, 2, 3]].\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ignores_strict_bitstring_generator() {
        let src = "-module(t).\nfoo() -> [X || <<X:8>> <:= <<1, 2, 3>>].\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ignores_strict_map_generator() {
        let src = "-module(t).\nfoo(M) -> #{K => V || K := V <:- M}.\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ignores_zip_generator() {
        let src = "-module(t).\nfoo() -> [{X, Y} || X <- [1, 2] && Y <- [3, 4]].\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn flags_each_generator_in_a_comprehension_once() {
        let src = "-module(t).\nfoo() -> [[X || X <- L] || L <- [[1], [2]]].\n";
        assert_eq!(findings(check, src), ["<-", "<-"]);
    }

    #[test]
    fn ng_fixture_has_three_findings() {
        let src = include_str!("../../rules/strict_generator/ng.erl");
        assert_eq!(findings(check, src).len(), 3);
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/strict_generator/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
