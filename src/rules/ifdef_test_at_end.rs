//! `ifdef_test_at_end` lint: `-ifdef(TEST)` is a single trailing block.

use super::Rule;
use crate::{BranchContext, Context, Finding, Span};

/// Lint rule that requires `-ifdef(TEST)` to be one trailing block.
pub const RULE: Rule = Rule::new(
    "ifdef_test_at_end",
    include_str!("../../rules/ifdef_test_at_end/rule.md"),
    check,
);

fn check(ctx: &Context, branch: &BranchContext) -> Vec<Finding> {
    // The directive is not a parse node, so the check reads the original
    // token stream. That stream is shared by every branch; emit on the
    // mainline only, or the same directive is reported once per branch.
    if !is_mainline(ctx, branch) {
        return Vec::new();
    }
    let Some(node) = branch.tree.roots().next().map(|root| root.node_id()) else {
        return Vec::new();
    };

    let lexical: Vec<_> = ctx
        .original_tokens
        .iter()
        .copied()
        .filter(|token| token.kind().is_lexical())
        .collect();
    let mut findings: Vec<_> = test_blocks(&ctx.text, &lexical)
        .into_iter()
        .filter(|block| !block.is_allowed_tail(&lexical))
        .map(|block| Finding {
            span: block.span,
            node,
        })
        .collect();
    findings.sort_by_key(|finding| finding.span.start);
    findings
}

fn is_mainline(ctx: &Context, branch: &BranchContext) -> bool {
    ctx.branches
        .first()
        .is_some_and(|mainline| std::ptr::eq(mainline, branch))
}

/// One `-ifdef(TEST)` and, when it is closed, the end of its `-endif.`.
struct TestBlock {
    span: Span,
    endif_end: Option<usize>,
}

impl TestBlock {
    /// A trailing block has no lexical token after its `-endif.`.
    fn is_allowed_tail(&self, lexical: &[erl_tokenize::Token]) -> bool {
        let Some(endif_end) = self.endif_end else {
            return false;
        };
        lexical
            .iter()
            .all(|token| token.start().offset() < endif_end)
    }
}

/// `-ifdef(TEST)` blocks in source order, including unclosed ones.
///
/// Macro names in `-ifdef` are uppercase variables in the token stream
/// (`TEST`), not atoms. A malformed directive yields fewer blocks rather
/// than aborting the rule.
fn test_blocks(text: &str, lexical: &[erl_tokenize::Token]) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut stack: Vec<CondFrame> = Vec::new();
    let mut i = 0;

    while i < lexical.len() {
        if !is_hyphen(lexical[i]) {
            i += 1;
            continue;
        }
        let Some((kind, end_idx)) = parse_directive_kind(text, lexical, i) else {
            i += 1;
            continue;
        };
        let directive_end = lexical[end_idx].end().offset();

        match kind {
            DirectiveKind::IfdefTest { span } => {
                stack.push(CondFrame::Test { span });
            }
            DirectiveKind::OpenOther => stack.push(CondFrame::Other),
            DirectiveKind::OtherArm => {}
            DirectiveKind::Endif => match stack.pop() {
                Some(CondFrame::Test { span }) => blocks.push(TestBlock {
                    span,
                    endif_end: Some(directive_end),
                }),
                Some(CondFrame::Other) | None => {}
            },
        }
        i = end_idx + 1;
    }

    while let Some(frame) = stack.pop() {
        if let CondFrame::Test { span } = frame {
            blocks.push(TestBlock {
                span,
                endif_end: None,
            });
        }
    }

    blocks
}

#[derive(Debug)]
enum CondFrame {
    Test { span: Span },
    Other,
}

#[derive(Debug)]
enum DirectiveKind {
    IfdefTest { span: Span },
    OpenOther,
    OtherArm,
    Endif,
}

fn parse_directive_kind(
    text: &str,
    tokens: &[erl_tokenize::Token],
    hyphen_idx: usize,
) -> Option<(DirectiveKind, usize)> {
    let name_idx = hyphen_idx + 1;
    let name_tok = *tokens.get(name_idx)?;
    // `if` and `else` are keywords. `ifdef` / `ifndef` / `elif` / `endif` are atoms.
    let name = match name_tok.kind() {
        erl_tokenize::TokenKind::Atom | erl_tokenize::TokenKind::Keyword(_) => name_tok.text(text),
        _ => return None,
    };

    match name {
        "else" => bare_directive(tokens, name_idx, DirectiveKind::OtherArm),
        "endif" => bare_directive(tokens, name_idx, DirectiveKind::Endif),
        "ifdef" | "ifndef" => parse_named_conditional(text, tokens, hyphen_idx, name_idx, name),
        "if" => {
            let dot_idx = consume_through_dot(tokens, name_idx + 1)?;
            Some((DirectiveKind::OpenOther, dot_idx))
        }
        "elif" => {
            let dot_idx = consume_through_dot(tokens, name_idx + 1)?;
            Some((DirectiveKind::OtherArm, dot_idx))
        }
        _ => None,
    }
}

fn bare_directive(
    tokens: &[erl_tokenize::Token],
    name_idx: usize,
    kind: DirectiveKind,
) -> Option<(DirectiveKind, usize)> {
    let dot_idx = name_idx + 1;
    let dot = *tokens.get(dot_idx)?;
    if !is_dot(dot) {
        return None;
    }
    Some((kind, dot_idx))
}

fn parse_named_conditional(
    text: &str,
    tokens: &[erl_tokenize::Token],
    hyphen_idx: usize,
    name_idx: usize,
    name: &str,
) -> Option<(DirectiveKind, usize)> {
    if let Some(parsed) = simple_named_conditional(text, tokens, hyphen_idx, name_idx, name) {
        return Some(parsed);
    }
    let dot_idx = consume_through_dot(tokens, name_idx + 1)?;
    Some((DirectiveKind::OpenOther, dot_idx))
}

fn simple_named_conditional(
    text: &str,
    tokens: &[erl_tokenize::Token],
    hyphen_idx: usize,
    name_idx: usize,
    name: &str,
) -> Option<(DirectiveKind, usize)> {
    let open = *tokens.get(name_idx + 1)?;
    let macro_tok = *tokens.get(name_idx + 2)?;
    let close = *tokens.get(name_idx + 3)?;
    let dot = *tokens.get(name_idx + 4)?;
    if !is_open_paren(open) || !is_close_paren(close) || !is_dot(dot) {
        return None;
    }
    let dot_idx = name_idx + 4;
    if name == "ifdef" && is_test_macro(text, macro_tok) {
        let span = ifdef_span(text, tokens[hyphen_idx], macro_tok, close);
        return Some((DirectiveKind::IfdefTest { span }, dot_idx));
    }
    Some((DirectiveKind::OpenOther, dot_idx))
}

/// `-ifdef(TEST)` when that text is one line; otherwise the `TEST` token.
///
/// A single-line span keeps the caret on the directive. The finding's node
/// is only a fallback root, so a multi-line span would borrow that root's
/// endpoints.
fn ifdef_span(
    text: &str,
    hyphen: erl_tokenize::Token,
    test_tok: erl_tokenize::Token,
    close: erl_tokenize::Token,
) -> Span {
    let start = hyphen.start().offset();
    let end = close.end().offset();
    if text
        .get(start..end)
        .is_some_and(|slice| !slice.contains('\n'))
    {
        Span::new(start, end)
    } else {
        Span::new(test_tok.start().offset(), test_tok.end().offset())
    }
}

fn is_test_macro(text: &str, token: erl_tokenize::Token) -> bool {
    match token.value(text) {
        erl_tokenize::TokenValue::Variable(name) => name == "TEST",
        erl_tokenize::TokenValue::Atom(name) => name.as_ref() == "TEST",
        _ => false,
    }
}

fn consume_through_dot(tokens: &[erl_tokenize::Token], from: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut j = from;
    while j < tokens.len() {
        let token = tokens[j];
        if is_open_paren(token) {
            depth += 1;
        } else if is_close_paren(token) {
            depth -= 1;
        } else if is_dot(token) && depth == 0 {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn is_hyphen(token: erl_tokenize::Token) -> bool {
    matches!(
        token.kind(),
        erl_tokenize::TokenKind::Symbol(erl_tokenize::Symbol::Hyphen)
    )
}

fn is_dot(token: erl_tokenize::Token) -> bool {
    matches!(
        token.kind(),
        erl_tokenize::TokenKind::Symbol(erl_tokenize::Symbol::Dot)
    )
}

fn is_open_paren(token: erl_tokenize::Token) -> bool {
    matches!(
        token.kind(),
        erl_tokenize::TokenKind::Symbol(erl_tokenize::Symbol::OpenParen)
    )
}

fn is_close_paren(token: erl_tokenize::Token) -> bool {
    matches!(
        token.kind(),
        erl_tokenize::TokenKind::Symbol(erl_tokenize::Symbol::CloseParen)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    fn finding_starts(src: &str) -> Vec<usize> {
        let ctx = Context::analyze("t.erl", src.to_string()).expect("test source must scan");
        assert!(
            ctx.branches[0].tree.diagnostics().is_empty(),
            "parse diagnostics: {:?}",
            ctx.branches[0].tree.diagnostics()
        );
        check(&ctx, &ctx.branches[0])
            .into_iter()
            .map(|finding| finding.span.start)
            .collect()
    }

    #[test]
    fn accepts_file_without_ifdef_test() {
        let src = "\
-module(t).
% -ifdef(TEST).
f() ->
    \"-ifdef(TEST).\".
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_single_trailing_block() {
        let src = "\
-module(t).
-export([f/0]).
f() ->
    ok.

-ifdef(TEST).
f_test() ->
    ok.
-endif.
% trailing
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_spaced_ifdef_test() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef( TEST ).
g() ->
    ok.
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn flags_block_before_a_later_form() {
        let src = "\
-module(t).
-ifdef(TEST).
f_test() ->
    ok.
-endif.
f() ->
    ok.
";
        assert_eq!(findings(check, src), ["-ifdef(TEST)"]);
    }

    #[test]
    fn flags_only_blocks_that_are_not_the_tail() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
a() ->
    ok.
-endif.
-ifdef(TEST).
b() ->
    ok.
-endif.
";
        let first = src.find("-ifdef(TEST)").expect("first");
        let second = src.rfind("-ifdef(TEST)").expect("second");
        assert_ne!(first, second);
        assert_eq!(finding_starts(src), vec![first]);
    }

    #[test]
    fn flags_every_block_when_none_is_the_tail() {
        let src = "\
-module(t).
-ifdef(TEST).
a() ->
    ok.
-endif.
-ifdef(TEST).
b() ->
    ok.
-endif.
c() ->
    ok.
";
        let first = src.find("-ifdef(TEST)").expect("first");
        let second = src.rfind("-ifdef(TEST)").expect("second");
        assert_eq!(finding_starts(src), vec![first, second]);
    }

    #[test]
    fn accepts_trailing_block_that_has_else() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
a() ->
    ok.
-else.
b() ->
    ok.
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_trailing_block_that_has_elif() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
a() ->
    ok.
-elif(true).
b() ->
    ok.
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_else_of_a_nested_conditional() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
-if(true).
a() ->
    ok.
-else.
b() ->
    ok.
-endif.
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_elif_of_a_preceding_if() {
        let src = "\
-module(t).
-if(false).
f() ->
    ok.
-elif(true).
f() ->
    error.
-endif.
-ifdef(TEST).
g() ->
    ok.
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn flags_nested_ifdef_test_inside_a_trailing_block() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
-ifdef(TEST).
a() ->
    ok.
-endif.
-endif.
";
        let inner = src.rfind("-ifdef(TEST)").expect("inner");
        assert_ne!(inner, src.find("-ifdef(TEST)").expect("outer"));
        assert_eq!(finding_starts(src), vec![inner]);
    }

    #[test]
    fn accepts_ifdef_of_another_macro() {
        let src = "\
-module(t).
-ifdef(OTHER).
f() ->
    ok.
-else.
f() ->
    error.
-endif.
g() ->
    ok.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_ifndef_test() {
        let src = "\
-module(t).
-ifndef(TEST).
f() ->
    ok.
-endif.
g() ->
    ok.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn flags_unclosed_ifdef_test() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(TEST).
g() ->
    ok.
";
        assert_eq!(findings(check, src), ["-ifdef(TEST)"]);
    }

    #[test]
    fn flags_multiline_directive_on_the_test_token() {
        let src = "\
-module(t).
f() ->
    ok.
-ifdef(
TEST).
g() ->
    ok.
-endif.
h() ->
    ok.
";
        assert_eq!(findings(check, src), ["TEST"]);
    }

    #[test]
    fn reports_once_across_branches() {
        let src = "\
-module(t).
-ifdef(A).
f() ->
    ok.
-else.
f() ->
    error.
-endif.
-ifdef(TEST).
g() ->
    ok.
-endif.
h() ->
    ok.
";
        let ctx = Context::analyze("t.erl", src.to_string()).expect("test source must scan");
        assert!(ctx.branches.len() > 1, "expected a side branch");
        let count: usize = ctx
            .branches
            .iter()
            .map(|branch| check(&ctx, branch).len())
            .sum();
        assert_eq!(count, 1);
    }

    #[test]
    fn ng_fixture_has_findings() {
        let src = include_str!("../../rules/ifdef_test_at_end/ng.erl");
        assert!(!findings(check, src).is_empty());
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/ifdef_test_at_end/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
