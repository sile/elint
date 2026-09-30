//! `attr_order` lint: conventional module-attribute order and placement.

use super::Rule;
use crate::{BranchContext, Context, Finding, Span};

/// Lint rule that checks module-attribute order and placement.
pub const RULE: Rule = Rule::new(
    "attr_order",
    include_str!("../../rules/attr_order/rule.md"),
    check,
);

/// Ordered attribute classes. Lower rank must appear before higher rank.
/// Rank 5 covers `-record` / `-type` / `-opaque` / `-define` with free order
/// inside the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttrClass {
    Module = 1,
    Behaviour = 2,
    Export = 3,
    Include = 4,
    DefineTypeRecord = 5,
}

#[derive(Debug, Clone, Copy)]
struct AttrSite {
    class: AttrClass,
    /// Attribute name text used in tests (`export`, `include`, ...).
    name: &'static str,
    span: Span,
    /// Tree node when the attribute survived preprocessing; otherwise a
    /// fallback root used only for enclosing-function lookup.
    node: erl_parse::NodeId,
}

fn check(ctx: &Context, branch: &BranchContext) -> Vec<Finding> {
    let Some(fallback_node) = branch.tree.roots().next().map(|r| r.node_id()) else {
        return Vec::new();
    };
    let first_function_start = first_function_start(branch);
    let test_regions = test_ifdef_regions(&ctx.text, &ctx.original_tokens);
    let sites = collect_sites(ctx, branch, fallback_node);

    let mut errors = Vec::new();
    check_header_order(&sites, first_function_start, &mut errors);
    check_after_function_placement(&sites, first_function_start, &test_regions, &mut errors);
    errors
}

fn first_function_start(branch: &BranchContext) -> Option<usize> {
    branch.tree.roots().find_map(|root| {
        if root.kind() != erl_parse::SyntaxKind::FunctionDecl {
            return None;
        }
        branch.span_of_range(root.token_range()).map(|s| s.start)
    })
}

fn collect_sites(
    ctx: &Context,
    branch: &BranchContext,
    fallback_node: erl_parse::NodeId,
) -> Vec<AttrSite> {
    let mut sites = Vec::new();

    for root in branch.tree.roots() {
        if root.kind() != erl_parse::SyntaxKind::Attribute {
            continue;
        }
        let Some(name_node) = root
            .children()
            .find(|c| c.kind() == erl_parse::SyntaxKind::AttributeName)
        else {
            continue;
        };
        let Some(name) = attribute_name(branch, name_node) else {
            continue;
        };
        let Some(class) = classify(&name) else {
            continue;
        };
        let Some(span) = branch.span_of_range(name_node.token_range()) else {
            continue;
        };
        sites.push(AttrSite {
            class,
            name: static_name(&name),
            span,
            node: root.node_id(),
        });
    }

    for include in &branch.skipped_includes {
        let name = if include.is_lib {
            "include_lib"
        } else {
            "include"
        };
        if include.name_span.text(&ctx.text) != name {
            continue;
        }
        sites.push(AttrSite {
            class: AttrClass::Include,
            name,
            span: include.name_span,
            node: fallback_node,
        });
    }

    for define in &branch.skipped_defines {
        if define.name_span.text(&ctx.text) != "define" {
            continue;
        }
        sites.push(AttrSite {
            class: AttrClass::DefineTypeRecord,
            name: "define",
            span: define.name_span,
            node: fallback_node,
        });
    }

    sites.sort_by_key(|s| s.span.start);
    sites
}

fn check_header_order(
    sites: &[AttrSite],
    first_function_start: Option<usize>,
    errors: &mut Vec<Finding>,
) {
    let mut max_rank = 0u8;
    let mut seen_class = [false; 6];

    for site in sites {
        if first_function_start.is_some_and(|start| site.span.start >= start) {
            break;
        }
        let rank = site.class as u8;
        let idx = rank as usize;
        if seen_class[idx] {
            continue;
        }
        seen_class[idx] = true;
        if rank < max_rank {
            errors.push(Finding {
                span: site.span,
                node: site.node,
            });
        }
        max_rank = max_rank.max(rank);
    }
}

fn check_after_function_placement(
    sites: &[AttrSite],
    first_function_start: Option<usize>,
    test_regions: &[Span],
    errors: &mut Vec<Finding>,
) {
    let Some(func_start) = first_function_start else {
        return;
    };

    for site in sites {
        if site.span.start < func_start {
            continue;
        }

        // Never allowed after functions.
        if matches!(
            site.name,
            "type" | "opaque" | "record" | "export" | "export_type"
        ) {
            errors.push(Finding {
                span: site.span,
                node: site.node,
            });
            continue;
        }

        // Allowed after functions only inside `-ifdef(TEST)`.
        if matches!(site.name, "define" | "include" | "include_lib") {
            if in_any_region(site.span, test_regions) {
                continue;
            }
            errors.push(Finding {
                span: site.span,
                node: site.node,
            });
        }
    }
}

fn in_any_region(span: Span, regions: &[Span]) -> bool {
    regions.iter().any(|r| r.contains(span))
}

/// Source ranges of `-ifdef(TEST).` then-arms (up to `-else` / `-endif`).
///
/// Nested non-`TEST` conditionals stay inside the outer TEST region. A
/// malformed or exotic conditional form simply yields fewer regions rather
/// than aborting the rule.
///
/// Macro names in `-ifdef` are uppercase variables in the token stream
/// (`TEST`), not atoms.
fn test_ifdef_regions(text: &str, tokens: &[erl_tokenize::Token]) -> Vec<Span> {
    let lexical: Vec<_> = tokens
        .iter()
        .copied()
        .filter(|t| t.kind().is_lexical())
        .collect();

    let mut regions = Vec::new();
    let mut stack: Vec<CondFrame> = Vec::new();
    let mut i = 0;

    while i < lexical.len() {
        if !is_hyphen(lexical[i]) {
            i += 1;
            continue;
        }
        let Some((kind, end_idx)) = parse_directive_kind(text, &lexical, i) else {
            i += 1;
            continue;
        };
        let directive_start = lexical[i].start().offset();
        let directive_end = lexical[end_idx].end().offset();

        match kind {
            DirectiveKind::IfdefTest => {
                stack.push(CondFrame::TestThen {
                    body_start: directive_end,
                });
            }
            DirectiveKind::OpenOther => {
                stack.push(CondFrame::Other);
            }
            DirectiveKind::Else => {
                if let Some(frame) = stack.last_mut()
                    && let CondFrame::TestThen { body_start } = *frame
                {
                    if body_start <= directive_start {
                        regions.push(Span::new(body_start, directive_start));
                    }
                    *frame = CondFrame::Other;
                }
            }
            DirectiveKind::Endif => match stack.pop() {
                Some(CondFrame::TestThen { body_start }) => {
                    if body_start <= directive_start {
                        regions.push(Span::new(body_start, directive_start));
                    }
                }
                Some(CondFrame::Other) | None => {}
            },
        }
        i = end_idx + 1;
    }

    regions
}

#[derive(Debug, Clone, Copy)]
enum CondFrame {
    TestThen { body_start: usize },
    Other,
}

#[derive(Debug, Clone, Copy)]
enum DirectiveKind {
    IfdefTest,
    OpenOther,
    Else,
    Endif,
}

fn parse_directive_kind(
    text: &str,
    tokens: &[erl_tokenize::Token],
    hyphen_idx: usize,
) -> Option<(DirectiveKind, usize)> {
    let name_idx = hyphen_idx + 1;
    let name_tok = *tokens.get(name_idx)?;
    if name_tok.kind() != erl_tokenize::TokenKind::Atom {
        return None;
    }
    let name = match name_tok.value(text) {
        erl_tokenize::TokenValue::Atom(a) => a,
        _ => return None,
    };

    match name.as_ref() {
        "else" => {
            let dot_idx = name_idx + 1;
            let dot = *tokens.get(dot_idx)?;
            if !is_dot(dot) {
                return None;
            }
            Some((DirectiveKind::Else, dot_idx))
        }
        "endif" => {
            let dot_idx = name_idx + 1;
            let dot = *tokens.get(dot_idx)?;
            if !is_dot(dot) {
                return None;
            }
            Some((DirectiveKind::Endif, dot_idx))
        }
        "ifdef" => {
            // -ifdef(TEST).  Macro names tokenize as variables.
            let open = *tokens.get(name_idx + 1)?;
            let macro_tok = *tokens.get(name_idx + 2)?;
            let close = *tokens.get(name_idx + 3)?;
            let dot = *tokens.get(name_idx + 4)?;
            if !is_open_paren(open) || !is_close_paren(close) || !is_dot(dot) {
                return None;
            }
            let is_test = match macro_tok.value(text) {
                erl_tokenize::TokenValue::Variable(v) => v == "TEST",
                erl_tokenize::TokenValue::Atom(a) => a.as_ref() == "TEST",
                _ => false,
            };
            if is_test {
                Some((DirectiveKind::IfdefTest, name_idx + 4))
            } else {
                Some((DirectiveKind::OpenOther, name_idx + 4))
            }
        }
        "ifndef" | "if" | "elif" => {
            let mut depth = 0i32;
            let mut j = name_idx + 1;
            while j < tokens.len() {
                let t = tokens[j];
                if is_open_paren(t) {
                    depth += 1;
                } else if is_close_paren(t) {
                    depth -= 1;
                } else if is_dot(t) && depth == 0 {
                    return Some((DirectiveKind::OpenOther, j));
                }
                j += 1;
            }
            None
        }
        _ => None,
    }
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

fn classify(name: &str) -> Option<AttrClass> {
    Some(match name {
        "module" => AttrClass::Module,
        "behaviour" | "behavior" => AttrClass::Behaviour,
        "export" | "export_type" => AttrClass::Export,
        "include" | "include_lib" => AttrClass::Include,
        "record" | "type" | "opaque" | "define" => AttrClass::DefineTypeRecord,
        _ => return None,
    })
}

fn static_name(name: &str) -> &'static str {
    match name {
        "module" => "module",
        "behaviour" | "behavior" => "behaviour",
        "export" => "export",
        "export_type" => "export_type",
        "include" => "include",
        "include_lib" => "include_lib",
        "record" => "record",
        "type" => "type",
        "opaque" => "opaque",
        "define" => "define",
        _ => "attribute",
    }
}

fn attribute_name(branch: &BranchContext, name: erl_parse::NodeView<'_>) -> Option<String> {
    name.indexed_tokens().find_map(|(i, _)| {
        let token = branch.source_tokens.get(i.get())?;
        match token.value() {
            erl_tokenize::TokenValue::Atom(a) => Some(a.into_owned()),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_support::findings;

    #[test]
    fn accepts_conventional_header_order() {
        let src = "\
-module(t).
-behaviour(gen_server).
-export([f/0]).
-export_type([t/0]).
-include(\"a.hrl\").
-include_lib(\"b/include/b.hrl\").
-record(r, {a}).
-type t() :: ok.
-define(A, 1).
-spec f() -> ok.
f() ->
    ok.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_define_type_record_any_order_in_group() {
        let src = "\
-module(t).
-export([f/0]).
-include(\"a.hrl\").
-define(A, 1).
-type t() :: ok.
-record(r, {a}).
f() ->
    ok.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn accepts_define_and_include_inside_ifdef_test() {
        let src = "\
-module(t).
-export([f/0]).
-include(\"a.hrl\").
-type t() :: ok.
f() ->
    ok.

-ifdef(TEST).
-define(TEST_MACRO, 1).
-include_lib(\"eunit/include/eunit.hrl\").
-endif.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn flags_define_after_function_outside_test() {
        let src = "\
-module(t).
-export([f/0]).
f() ->
    ok.
-define(TEST_MACRO, 1).
";
        assert_eq!(findings(check, src), ["define"]);
    }

    #[test]
    fn flags_export_after_include() {
        let src = "\
-module(t).
-include(\"a.hrl\").
-export([f/0]).
f() ->
    ok.
";
        assert_eq!(findings(check, src), ["export"]);
    }

    #[test]
    fn flags_include_after_type() {
        let src = "\
-module(t).
-export([f/0]).
-type t() :: ok.
-include(\"a.hrl\").
f() ->
    ok.
";
        assert_eq!(findings(check, src), ["include"]);
    }

    #[test]
    fn flags_type_after_function() {
        let src = "\
-module(t).
-export([f/0]).
f() ->
    ok.
-type t() :: ok.
";
        assert_eq!(findings(check, src), ["type"]);
    }

    #[test]
    fn flags_record_after_function_even_inside_test() {
        let src = "\
-module(t).
-export([f/0]).
f() ->
    ok.
-ifdef(TEST).
-record(r, {a}).
-endif.
";
        assert_eq!(findings(check, src), ["record"]);
    }

    #[test]
    fn flags_opaque_after_function() {
        let src = "\
-module(t).
-export([f/0]).
f() ->
    ok.
-opaque t() :: ok.
";
        assert_eq!(findings(check, src), ["opaque"]);
    }

    #[test]
    fn ignores_spec_beside_function() {
        let src = "\
-module(t).
-export([f/0]).
-spec f() -> ok.
f() ->
    ok.
";
        assert!(findings(check, src).is_empty());
    }

    #[test]
    fn ng_fixture_has_findings() {
        let src = include_str!("../../rules/attr_order/ng.erl");
        assert!(!findings(check, src).is_empty());
    }

    #[test]
    fn ok_fixture_has_no_findings() {
        let src = include_str!("../../rules/attr_order/ok.erl");
        assert!(findings(check, src).is_empty());
    }
}
