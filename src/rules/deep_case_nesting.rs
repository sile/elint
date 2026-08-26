//! `deep_case_nesting` lint: flag case nested three or more levels deep.

use super::{Rule, token_span_in_node};
use crate::{BranchContext, Context, Span};

/// Lint rule that flags deeply nested `case` expressions.
pub const RULE: Rule = Rule::new(
    "deep_case_nesting",
    include_str!("../../rules/deep_case_nesting/rule.md"),
    check,
);

fn check(_ctx: &Context, branch: &BranchContext) -> Vec<Span> {
    let mut errors = Vec::new();
    for root in branch.tree.roots() {
        walk(branch, root, 0, &mut errors);
    }
    errors
}

fn walk(
    branch: &BranchContext,
    node: erl_parse::NodeView<'_>,
    case_depth: usize,
    errors: &mut Vec<Span>,
) {
    if is_depth_break(node.kind()) {
        for child in node.children() {
            walk(branch, child, 0, errors);
        }
        return;
    }
    let child_depth = match node.kind() {
        erl_parse::SyntaxKind::CaseExpr => {
            if case_depth + 1 >= 3
                && let Some(span) = token_span_in_node(branch, node, |token| {
                    matches!(
                        token.kind(),
                        erl_tokenize::TokenKind::Keyword(erl_tokenize::Keyword::Case)
                    )
                })
            {
                errors.push(span);
            }
            case_depth + 1
        }
        _ => case_depth,
    };
    for child in node.children() {
        walk(branch, child, child_depth, errors);
    }
}

/// Expressions that break a run of consecutive `case`s. The `maybe` rewrite
/// is no longer straightforward across them, so the depth starts over.
fn is_depth_break(kind: erl_parse::SyntaxKind) -> bool {
    matches!(
        kind,
        erl_parse::SyntaxKind::AnonymousFun
            | erl_parse::SyntaxKind::NamedFun
            | erl_parse::SyntaxKind::TryExpr
            | erl_parse::SyntaxKind::ReceiveExpr
            | erl_parse::SyntaxKind::BeginExpr
            | erl_parse::SyntaxKind::MaybeExpr
            | erl_parse::SyntaxKind::ListExpr
            | erl_parse::SyntaxKind::TupleExpr
            | erl_parse::SyntaxKind::MapExpr
            | erl_parse::SyntaxKind::BitstringExpr
            | erl_parse::SyntaxKind::ListComprehension
            | erl_parse::SyntaxKind::MapComprehension
            | erl_parse::SyntaxKind::BinaryComprehension
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    #[test]
    fn flags_case_three_levels_deep() {
        let src = "\
-module(t).
foo(A, B, C) ->
    case A of
        a ->
            case B of
                b ->
                    case C of
                        c ->
                            ok
                    end
            end
    end.
";
        // Depths: A=1, B=2, C=3. C is reported.
        assert_eq!(findings(check, src), ["case"]);
    }

    #[test]
    fn flags_every_case_three_or_more_levels_deep() {
        let src = "\
-module(t).
foo(A, B, C, D, E) ->
    case A of
        a ->
            case B of
                b ->
                    case C of
                        c ->
                            case D of
                                d ->
                                    case E of
                                        e ->
                                            ok
                                    end
                            end
                    end
            end
    end.
";
        // Depths: A=1, B=2, C=3, D=4, E=5. C, D and E are reported.
        assert_eq!(findings(check, src), ["case", "case", "case"]);
    }

    #[test]
    fn ignores_case_two_levels_deep() {
        let src = "\
-module(t).
foo(A, B) ->
    case A of
        a ->
            case B of
                b ->
                    ok
            end
    end.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn resets_depth_at_fun_boundary() {
        let src = "\
-module(t).
foo(A) ->
    case A of
        a ->
            fun() ->
                case 1 of
                    1 ->
                        case 2 of
                            2 ->
                                ok
                        end
                end
            end()
    end.
";
        // Without the reset the innermost case would be depth 3.
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn resets_depth_at_block_expressions() {
        let src = "\
-module(t).
foo(A, B, C) ->
    case A of
        a ->
            try
                case B of
                    b ->
                        case C of
                            c ->
                                ok
                        end
                end
            catch
                _:_ ->
                    error
            end
    end.
";
        // The `try` breaks the run: depths B=1, C=2.
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn resets_depth_at_container_literals() {
        let src = "\
-module(t).
foo(A, B, C) ->
    case A of
        a ->
            {case B of
                b ->
                    case C of
                        c ->
                            ok
                    end
            end}
    end.
";
        // The tuple breaks the run: depths B=1, C=2.
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ng_fixture_has_four_findings() {
        let src = include_str!("../../rules/deep_case_nesting/ng.erl");
        assert_eq!(findings(check, src).len(), 4);
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/deep_case_nesting/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
