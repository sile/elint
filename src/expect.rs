//! `-elint_expect` attributes that suppress matching findings.

use std::collections::HashMap;

use erl_parse::{NodeView, SyntaxKind};

use crate::{BranchContext, Context, Error, Span};

/// Parsed `-elint_expect` entries collected from the mainline tree.
#[derive(Debug)]
pub struct ExpectRules {
    /// Individual expectations.
    pub rules: Vec<ExpectRule>,
}

impl ExpectRules {
    /// Reads `-elint_expect(Rule, {function, Name, Arity} | module, Reason)`
    /// forms from the mainline tree and resolves each target.
    ///
    /// When `target_lint_names` is non-empty, only expectations whose rule is
    /// listed are validated and registered; the others are ignored. A payload
    /// whose rule name cannot be read is still an error.
    pub fn new(ctx: &Context, target_lint_names: &[String]) -> Result<Self, Error> {
        let branch = &ctx.branches[0];
        let functions = function_index(branch);
        let mut rules = Vec::new();

        for attribute in branch.tree.roots() {
            if attribute.kind() != SyntaxKind::Attribute {
                continue;
            }
            let Some(attr_span) = branch.span_of_range(attribute.range()) else {
                continue;
            };
            let mut children = attribute.children();
            let Some(name) = children.next() else {
                continue;
            };
            if name.kind() != SyntaxKind::AttributeName || !is_elint_expect(branch, name) {
                continue;
            }
            let Some(payload) = children.next() else {
                return Err(Error::new(attr_span, "missing -elint_expect payload"));
            };
            let Some(payload_span) = branch.span_of_range(payload.range()) else {
                return Err(Error::new(attr_span, "missing -elint_expect payload"));
            };
            let payload_text = &ctx.text[payload_span.start..payload_span.end];
            let Some(inner) = payload_text
                .strip_prefix('(')
                .and_then(|t| t.strip_suffix(')'))
            else {
                return Err(Error::new(
                    attr_span,
                    "invalid -elint_expect payload (expected (Rule, {function, Name, Arity} | module, Reason))",
                ));
            };

            let parsed = match parse_expect_payload(inner) {
                Ok(parsed) => parsed,
                Err(PayloadError::Invalid) => {
                    return Err(Error::new(
                        attr_span,
                        "invalid -elint_expect payload (expected {Rule, {function, Name, Arity} | module, Reason})",
                    ));
                }
                Err(PayloadError::MissingReason { rule }) => {
                    if !wanted(&rule, target_lint_names) {
                        continue;
                    }
                    return Err(Error::new(attr_span, "missing reason in -elint_expect"));
                }
            };
            if !wanted(&parsed.rule, target_lint_names) {
                continue;
            }
            let Some(rule) = crate::rules::RULES.iter().find(|v| v.name == parsed.rule) else {
                return Err(Error::new(
                    attr_span,
                    format!("unknown rule: {}", parsed.rule),
                ));
            };
            let scope_spans = match &parsed.target {
                ExpectTarget::Function(name, arity) => {
                    let Some(spans) = functions.get(&(name.clone(), *arity)) else {
                        return Err(Error::new(
                            attr_span,
                            format!("unknown function: {name}/{arity}"),
                        ));
                    };
                    spans.clone()
                }
                ExpectTarget::Module => vec![Span::new(0, ctx.text.len())],
            };

            rules.push(ExpectRule {
                name: rule.name,
                span: attr_span,
                target: parsed.target,
                reason: parsed.reason,
                scope_spans,
                matched: false,
            });
        }

        Ok(Self { rules })
    }

    /// Marks matching expectations. Returns whether `span` was expected for `lint_name`.
    pub fn handle_error(&mut self, lint_name: &'static str, span: Span) -> bool {
        let mut expected = false;
        for rule in &mut self.rules {
            if rule.name == lint_name && rule.scope_spans.iter().any(|s| s.contains(span)) {
                rule.matched = true;
                expected = true;
            }
        }
        expected
    }

    /// Expectations that never matched a finding.
    pub fn unmatched_expectations(&self) -> impl Iterator<Item = &ExpectRule> {
        self.rules.iter().filter(|r| !r.matched)
    }
}

/// Target of an `-elint_expect` expectation.
#[derive(Debug)]
pub enum ExpectTarget {
    /// Findings inside any clause of `Name/Arity` are suppressed.
    Function(String, u64),
    /// Findings anywhere in the current module are suppressed.
    Module,
}

impl ExpectTarget {
    /// Human-readable description, e.g. `foo/1` or `module`.
    pub fn describe(&self) -> String {
        match self {
            ExpectTarget::Function(name, arity) => format!("{name}/{arity}"),
            ExpectTarget::Module => "module".into(),
        }
    }
}

/// One `-elint_expect` entry.
#[derive(Debug)]
pub struct ExpectRule {
    /// Canonical rule name.
    pub name: &'static str,
    /// Span of the attribute form that declared the expectation.
    pub span: Span,
    /// Suppression target.
    pub target: ExpectTarget,
    /// Required suppression reason.
    pub reason: String,
    /// Original-file spans that scope the expectation (the target function's
    /// clauses, or the whole file for a module target).
    pub scope_spans: Vec<Span>,
    /// Whether a finding with this rule name landed inside a scope span.
    pub matched: bool,
}

/// `Name/Arity` of a function mapped to the original-file spans of its clauses.
type FunctionIndex = HashMap<(String, u64), Vec<Span>>;

/// Maps every `FunctionClause` in the mainline branch to its clause span.
fn function_index(branch: &BranchContext) -> FunctionIndex {
    let mut index = FunctionIndex::new();
    for node in branch.tree.nodes() {
        if node.kind() != SyntaxKind::FunctionClause {
            continue;
        }
        let Some(name) = clause_name(branch, node) else {
            continue;
        };
        let Some(arity) = clause_arity(node) else {
            continue;
        };
        let Some(span) = branch.span_of_range(node.range()) else {
            continue;
        };
        index.entry((name, arity)).or_default().push(span);
    }
    index
}

/// Reads the name atom of a function clause: its first lexical token.
fn clause_name(branch: &BranchContext, node: NodeView<'_>) -> Option<String> {
    node.range().find_map(|i| {
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
fn clause_arity(node: NodeView<'_>) -> Option<u64> {
    let args = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ArgumentList)?;
    Some(args.children().count() as u64)
}

/// Returns whether the attribute name node spells `elint_expect`.
fn is_elint_expect(branch: &BranchContext, name: NodeView<'_>) -> bool {
    name.range().any(|i| {
        branch.source_tokens.get(i.get()).is_some_and(|token| {
            matches!(token.value(), erl_tokenize::TokenValue::Atom(a) if a == "elint_expect")
        })
    })
}

/// One decoded `-elint_expect` payload.
struct ParsedExpect {
    rule: String,
    target: ExpectTarget,
    reason: String,
}

/// Returns whether `rule` is in `target_lint_names`, or everything is wanted
/// when the list is empty.
fn wanted(rule: &str, target_lint_names: &[String]) -> bool {
    target_lint_names.is_empty() || target_lint_names.iter().any(|n| n == rule)
}

/// Why a payload could not be decoded.
enum PayloadError {
    /// The payload is not `{Rule, {function, Name, Arity} | module, Reason}`.
    Invalid,
    /// The reason is missing or is not a string. Carries the rule name so a
    /// caller can decide whether to report it.
    MissingReason { rule: String },
}

/// Interprets the payload body (between the attribute's parens) by wrapping
/// it in braces and re-parsing it as a term list.
fn parse_expect_payload(source: &str) -> Result<ParsedExpect, PayloadError> {
    let wrapped = format!("{{{source}}}.");
    let tokens = erl_tokenize::scan_tokens(&wrapped).map_err(|_| PayloadError::Invalid)?;
    let tree = erl_parse::parse(&tokens, erl_parse::ParseMode::TermList);
    if !tree.diagnostics().is_empty() {
        return Err(PayloadError::Invalid);
    }
    let roots: Vec<_> = tree.roots().collect();
    if roots.len() != 1 || roots[0].kind() != SyntaxKind::TupleExpr {
        return Err(PayloadError::Invalid);
    }
    let mut elements = roots[0].children();
    let rule_node = elements.next().ok_or(PayloadError::Invalid)?;
    let target_node = elements.next().ok_or(PayloadError::Invalid)?;
    let rule = read_term_atom(rule_node, &wrapped).ok_or(PayloadError::Invalid)?;
    let target = read_target(target_node, &wrapped).ok_or(PayloadError::Invalid)?;
    let reason_node = match elements.next() {
        Some(node) => node,
        None => return Err(PayloadError::MissingReason { rule }),
    };
    if elements.next().is_some() {
        return Err(PayloadError::Invalid);
    }
    let reason = match read_term_string(reason_node, &wrapped) {
        Some(reason) => reason,
        None => return Err(PayloadError::MissingReason { rule }),
    };
    Ok(ParsedExpect {
        rule,
        target,
        reason,
    })
}

/// Reads the target: `{function, Name, Arity}` or `module`.
fn read_target(node: NodeView<'_>, source: &str) -> Option<ExpectTarget> {
    if node.kind() == SyntaxKind::TupleExpr {
        let mut elements = node.children();
        let tag = read_term_atom(elements.next()?, source)?;
        if tag != "function" {
            return None;
        }
        let name = read_term_atom(elements.next()?, source)?;
        let arity = read_term_integer(elements.next()?, source)?;
        if elements.next().is_some() {
            return None;
        }
        Some(ExpectTarget::Function(name, arity))
    } else if node.kind() == SyntaxKind::AtomExpr {
        let name = read_term_atom(node, source)?;
        if name == "module" {
            Some(ExpectTarget::Module)
        } else {
            None
        }
    } else {
        None
    }
}

fn read_term_atom(node: NodeView<'_>, source: &str) -> Option<String> {
    if node.kind() != SyntaxKind::AtomExpr {
        return None;
    }
    let token = node.tokens().iter().find(|t| t.kind().is_lexical())?;
    match token.value(source) {
        erl_tokenize::TokenValue::Atom(a) => Some(a.into_owned()),
        _ => None,
    }
}

fn read_term_integer(node: NodeView<'_>, source: &str) -> Option<u64> {
    if node.kind() != SyntaxKind::IntegerExpr {
        return None;
    }
    let token = node.tokens().iter().find(|t| t.kind().is_lexical())?;
    match token.value(source) {
        erl_tokenize::TokenValue::Integer(Some(n)) => Some(n),
        _ => None,
    }
}

fn read_term_string(node: NodeView<'_>, source: &str) -> Option<String> {
    if node.kind() != SyntaxKind::StringExpr {
        return None;
    }
    let token = node.tokens().iter().find(|t| t.kind().is_lexical())?;
    match token.value(source) {
        erl_tokenize::TokenValue::String(s) => Some(s.into_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(src: &str) -> Context {
        Context::analyze("t.erl", src.to_string()).expect("test source must scan")
    }

    fn element_bif_findings(ctx: &Context) -> Vec<Span> {
        let rule = crate::rules::RULES
            .iter()
            .find(|rule| rule.name == "element_bif")
            .expect("element_bif rule");
        let mut out = Vec::new();
        for branch in &ctx.branches {
            out.extend((rule.check)(ctx, branch));
        }
        out
    }

    #[test]
    fn reads_attribute_and_suppresses_finding() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, foo, 1}, \"dynamic tuple shape\").
foo(T) ->
    element(1, T).
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        assert_eq!(expect.rules.len(), 1);
        assert_eq!(expect.rules[0].name, "element_bif");
        assert!(matches!(
            expect.rules[0].target,
            ExpectTarget::Function(ref name, 1) if name == "foo"
        ));
        assert_eq!(expect.rules[0].reason, "dynamic tuple shape");
        assert_eq!(expect.rules[0].scope_spans.len(), 1);
        assert_eq!(
            expect.rules[0].scope_spans[0].text(&ctx.text),
            "foo(T) ->\n    element(1, T)"
        );

        let spans = element_bif_findings(&ctx);
        assert_eq!(spans.len(), 1);
        assert!(expect.handle_error("element_bif", spans[0]));
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn suppresses_finding_inside_any_clause_of_target() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, foo, 1}, \"dynamic tuple shape\").
foo(0) ->
    ok;
foo(T) ->
    element(1, T).
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        let spans = element_bif_findings(&ctx);
        assert_eq!(spans.len(), 1);
        assert!(expect.handle_error("element_bif", spans[0]));
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn does_not_suppress_finding_outside_target() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, foo, 1}, \"dynamic tuple shape\").
foo(T) ->
    element(1, T).
bar(T) ->
    element(1, T).
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        let spans = element_bif_findings(&ctx);
        assert_eq!(spans.len(), 2);
        assert!(expect.handle_error("element_bif", spans[0]));
        assert!(!expect.handle_error("element_bif", spans[1]));
        assert_eq!(expect.unmatched_expectations().count(), 0);
    }

    #[test]
    fn distinct_rule_names_do_not_match() {
        let src = "\
-module(t).
-elint_expect(newline_after_arrow, {function, foo, 0}, \"one-line clause\").
foo() -> ok.
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        assert_eq!(expect.rules.len(), 1);
        let spans = element_bif_findings(&ctx);
        assert!(spans.is_empty());
        assert!(!expect.handle_error("element_bif", Span::new(0, 0)));
        assert_eq!(expect.unmatched_expectations().count(), 1);
    }

    #[test]
    fn multiple_attributes_for_one_function() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, foo, 1}, \"dynamic tuple shape\").
-elint_expect(element_bif, {function, bar, 1}, \"dynamic tuple shape\").
foo(T) ->
    element(1, T).
bar(T) ->
    element(1, T).
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        assert_eq!(expect.rules.len(), 2);
        let spans = element_bif_findings(&ctx);
        assert_eq!(spans.len(), 2);
        assert!(expect.handle_error("element_bif", spans[0]));
        assert!(expect.handle_error("element_bif", spans[1]));
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn module_target_suppresses_finding_anywhere() {
        let src = "\
-module(t).
-elint_expect(element_bif, module, \"dynamic tuple shape\").
foo(T) ->
    element(1, T).
bar(T) ->
    element(1, T).
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        assert_eq!(expect.rules.len(), 1);
        assert!(matches!(expect.rules[0].target, ExpectTarget::Module));
        assert_eq!(
            expect.rules[0].scope_spans,
            vec![Span::new(0, ctx.text.len())]
        );
        let spans = element_bif_findings(&ctx);
        assert_eq!(spans.len(), 2);
        assert!(expect.handle_error("element_bif", spans[0]));
        assert!(expect.handle_error("element_bif", spans[1]));
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn module_target_still_requires_rule_name_match() {
        let src = "\
-module(t).
-elint_expect(element_bif, module, \"dynamic tuple shape\").
foo() ->
    ok.
";
        let ctx = analyze(src);
        let mut expect = ExpectRules::new(&ctx, &[]).expect("expect");
        assert!(!expect.handle_error("newline_after_arrow", Span::new(0, 0)));
        assert_eq!(expect.unmatched_expectations().count(), 1);
    }

    #[test]
    fn unknown_target_tag_is_an_error() {
        let src = "\
-module(t).
-elint_expect(element_bif, {record, foo}, \"reason\").
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &[]).expect_err("expect must fail");
        assert!(
            err.reason.contains("invalid -elint_expect payload"),
            "{:?}",
            err.reason
        );
    }

    #[test]
    fn missing_reason_is_an_error() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, foo, 1}).
foo(T) ->
    element(1, T).
";
        let err = ExpectRules::new(&analyze(src), &[]).expect_err("expect must fail");
        assert!(err.reason.contains("missing reason"), "{:?}", err.reason);
    }

    #[test]
    fn unknown_rule_is_an_error() {
        let src = "\
-module(t).
-elint_expect(no_such_rule, {function, foo, 0}, \"reason\").
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &[]).expect_err("expect must fail");
        assert!(err.reason.contains("unknown rule"), "{:?}", err.reason);
    }

    #[test]
    fn nonexistent_function_is_an_error() {
        let src = "\
-module(t).
-elint_expect(element_bif, {function, no_such, 0}, \"reason\").
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &[]).expect_err("expect must fail");
        assert!(err.reason.contains("unknown function"), "{:?}", err.reason);
    }

    #[test]
    fn invalid_payload_is_an_error() {
        let src = "\
-module(t).
-elint_expect(element_bif).
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &[]).expect_err("expect must fail");
        assert!(
            err.reason.contains("invalid -elint_expect payload"),
            "{:?}",
            err.reason
        );
    }

    #[test]
    fn filter_ignores_expectations_for_unlinted_rules() {
        let src = "\
-module(t).
-elint_expect(newline_after_arrow, {function, foo, 0}, \"one-line clause\").
foo() -> ok.
";
        let ctx = analyze(src);
        let expect = ExpectRules::new(&ctx, &["element_bif".to_string()]).expect("expect");
        assert!(expect.rules.is_empty());
        assert!(expect.unmatched_expectations().next().is_none());
    }

    #[test]
    fn filter_ignores_unknown_rule_outside_the_filter() {
        let src = "\
-module(t).
-elint_expect(no_such_rule, {function, foo, 0}, \"reason\").
foo() ->
    ok.
";
        let ctx = analyze(src);
        let expect = ExpectRules::new(&ctx, &["element_bif".to_string()]).expect("expect");
        assert!(expect.rules.is_empty());
    }

    #[test]
    fn filter_ignores_missing_reason_outside_the_filter() {
        let src = "\
-module(t).
-elint_expect(newline_after_arrow, {function, foo, 0}).
foo() ->
    ok.
";
        let ctx = analyze(src);
        let expect = ExpectRules::new(&ctx, &["element_bif".to_string()]).expect("expect");
        assert!(expect.rules.is_empty());
    }

    #[test]
    fn filter_ignores_unknown_function_outside_the_filter() {
        let src = "\
-module(t).
-elint_expect(newline_after_arrow, {function, no_such, 0}, \"reason\").
foo() ->
    ok.
";
        let ctx = analyze(src);
        let expect = ExpectRules::new(&ctx, &["element_bif".to_string()]).expect("expect");
        assert!(expect.rules.is_empty());
    }

    #[test]
    fn filter_keeps_validation_for_linted_rules() {
        let src = "\
-module(t).
-elint_expect(no_such_rule, {function, foo, 0}, \"reason\").
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &["no_such_rule".to_string()])
            .expect_err("expect must fail");
        assert!(err.reason.contains("unknown rule"), "{:?}", err.reason);
    }

    #[test]
    fn filter_keeps_invalid_payload_error_even_when_filtered() {
        let src = "\
-module(t).
-elint_expect(123, module, \"reason\").
foo() ->
    ok.
";
        let err = ExpectRules::new(&analyze(src), &["element_bif".to_string()])
            .expect_err("expect must fail");
        assert!(
            err.reason.contains("invalid -elint_expect payload"),
            "{:?}",
            err.reason
        );
    }
}
