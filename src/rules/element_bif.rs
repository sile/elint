//! `element_bif` lint: flag `element/2` and `erlang:element/2` with a literal index.

use super::Rule;
use crate::{BranchContext, Context, Finding};

/// Lint rule that flags `element/2` used as a BIF.
pub const RULE: Rule = Rule::new(
    "element_bif",
    include_str!("../../rules/element_bif/rule.md"),
    check,
);

fn check(_ctx: &Context, branch: &BranchContext) -> Vec<Finding> {
    let mut errors = Vec::new();
    for node in branch.tree.nodes() {
        check_node(branch, node, &mut errors);
    }
    errors
}

fn check_node(branch: &BranchContext, node: erl_parse::NodeView<'_>, errors: &mut Vec<Finding>) {
    if node.kind() != erl_parse::SyntaxKind::CallExpr {
        return;
    }

    let mut children = node.children();
    let Some(callee) = children.next() else {
        return;
    };
    let Some(args) = children.next() else {
        return;
    };
    if args.kind() != erl_parse::SyntaxKind::ArgumentList {
        return;
    }
    if !is_element_callee(branch, callee) {
        return;
    }

    let args: Vec<_> = args.children().collect();
    if args.len() != 2 {
        return;
    }
    if args[0].kind() != erl_parse::SyntaxKind::IntegerExpr {
        return;
    }

    if let Some(span) = branch.span_of_range(node.token_range()) {
        errors.push(Finding {
            span,
            node: node.node_id(),
        });
    }
}

fn is_element_callee(branch: &BranchContext, callee: erl_parse::NodeView<'_>) -> bool {
    match callee.kind() {
        erl_parse::SyntaxKind::AtomExpr => atom_eq(branch, callee, "element"),
        erl_parse::SyntaxKind::RemoteExpr => {
            let mut children = callee.children();
            let Some(module) = children.next() else {
                return false;
            };
            let Some(name) = children.next() else {
                return false;
            };
            atom_eq(branch, module, "erlang") && atom_eq(branch, name, "element")
        }
        _ => false,
    }
}

fn atom_eq(branch: &BranchContext, node: erl_parse::NodeView<'_>, expected: &str) -> bool {
    node.indexed_tokens().any(|(i, _)| {
        branch.source_tokens.get(i.get()).is_some_and(|token| {
            matches!(token.value(), erl_tokenize::TokenValue::Atom(name) if name == expected)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    #[test]
    fn flags_local_element_2_with_integer_index() {
        let src = "-module(t).\nfoo(T) -> element(1, T).\n";
        assert_eq!(findings(check, src), ["element(1, T)"]);
    }

    #[test]
    fn flags_erlang_element_2() {
        let src = "-module(t).\nfoo(T) -> erlang:element(1, T).\n";
        assert_eq!(findings(check, src), ["erlang:element(1, T)"]);
    }

    #[test]
    fn ignores_other_modules() {
        let src = "-module(t).\nfoo(T) -> lists:element(1, T).\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ignores_wrong_arity() {
        let src = "-module(t).\nfoo(T) -> element(1, T, extra).\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ignores_non_integer_index() {
        let src = "-module(t).\nfoo(N, T) -> element(N, T).\n";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ng_fixture_has_two_findings() {
        let src = include_str!("../../rules/element_bif/ng.erl");
        assert_eq!(findings(check, src).len(), 2);
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/element_bif/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
