//! `%% ELINT_EXPECT:` comments that suppress matching findings.

use crate::Context;
use crate::Span;

/// Parsed `ELINT_EXPECT` entries collected from original-file comments.
#[derive(Debug)]
pub struct ExpectRules {
    /// Individual expectations.
    pub rules: Vec<ExpectRule>,
}

impl ExpectRules {
    /// Reads `%% ELINT_EXPECT:` comments and binds each to the outermost
    /// syntax node that starts immediately after the comment.
    pub fn new(ctx: &Context) -> Result<Self, crate::Error> {
        let syntax_spans = ctx.syntax_spans();
        let mut rules = Vec::new();

        for token in &ctx.original_tokens {
            if token.kind() != erl_tokenize::TokenKind::Comment {
                continue;
            }

            let text = token.text(&ctx.text);
            let Some(text) = text.strip_prefix("%%").and_then(|t| t.lines().next()) else {
                continue;
            };
            let text = text.trim();

            let Some(text) = text.strip_prefix("ELINT_EXPECT:") else {
                continue;
            };
            let text = text.trim();

            let comment_span = Span::new(token.start().offset(), token.end().offset());
            let Some(target_span) = outermost_after(comment_span, &syntax_spans) else {
                continue;
            };

            for name in text.split(',') {
                let name = name.trim();
                let Some(rule) = crate::rules::RULES.iter().find(|v| v.name == name) else {
                    return Err(crate::Error::new(
                        comment_span,
                        format!("unknown rule: {name}"),
                    ));
                };

                rules.push(ExpectRule {
                    name: rule.name,
                    comment_span,
                    target_span,
                    matched: false,
                });
            }
        }

        Ok(Self { rules })
    }

    /// Marks matching expectations. Returns whether `span` was expected for `lint_name`.
    pub fn handle_error(&mut self, lint_name: &'static str, span: Span) -> bool {
        let mut expected = false;
        for rule in &mut self.rules {
            if rule.name == lint_name && rule.target_span.contains(span) {
                rule.matched = true;
                expected = true;
            }
        }
        expected
    }

    /// Expectations that never matched a finding.
    pub fn unmatched_expectations(&self) -> impl Iterator<Item = (&'static str, Span)> {
        self.rules
            .iter()
            .filter(|r| !r.matched)
            .map(|r| (r.name, r.comment_span))
    }
}

/// One `ELINT_EXPECT` entry.
#[derive(Debug)]
pub struct ExpectRule {
    /// Canonical rule name.
    pub name: &'static str,
    /// Span of the comment that declared the expectation.
    pub comment_span: Span,
    /// Span of the syntax node the comment binds to.
    pub target_span: Span,
    /// Whether a finding with this rule name landed inside [`ExpectRule::target_span`].
    pub matched: bool,
}

fn outermost_after(comment: Span, spans: &[Span]) -> Option<Span> {
    let mut best = None;
    for &span in spans {
        if span.start < comment.end {
            continue;
        }
        match best {
            None => best = Some(span),
            Some(current) => {
                if span.start < current.start
                    || (span.start == current.start && span.end > current.end)
                {
                    best = Some(span);
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_to_nested_call_not_only_module_root() {
        let src = "\
-module(t).
foo() ->
    g(
        %% ELINT_EXPECT: element_bif
        element(1, T)
    ).
";
        let ctx = Context::analyze("t.erl", src.to_string()).expect("scan");
        let expect = ExpectRules::new(&ctx).expect("expect");
        assert_eq!(expect.rules.len(), 1);
        assert_eq!(expect.rules[0].target_span.text(&ctx.text), "element(1, T)");
    }

    #[test]
    fn handle_error_matches_finding_inside_target() {
        let src = "\
-module(t).
foo(T) ->
    %% ELINT_EXPECT: element_bif
    element(1, T).
";
        let ctx = Context::analyze("t.erl", src.to_string()).expect("scan");
        let mut expect = ExpectRules::new(&ctx).expect("expect");
        let findings = (crate::rules::RULES[0].check)(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(expect.handle_error("element_bif", findings[0]));
        assert!(expect.unmatched_expectations().next().is_none());
    }
}
