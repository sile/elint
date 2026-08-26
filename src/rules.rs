//! Lint rules.

mod case_over_if;
mod deep_case_nesting;
mod element_bif;
mod newline_after_arrow;
mod strict_generator;

use crate::{BranchContext, Context, Finding, Span};

/// One lint: a name, the markdown description, and a check function.
#[derive(Debug)]
pub struct Rule {
    /// Canonical name used in CLI output and `-elint_expect`.
    pub name: &'static str,
    /// Markdown text describing the rule.
    pub text: &'static str,
    /// Returns findings that violate the rule in one branch.
    pub check: fn(&Context, &BranchContext) -> Vec<Finding>,
}

impl Rule {
    /// Builds a [`Rule`].
    pub const fn new(
        name: &'static str,
        text: &'static str,
        check: fn(&Context, &BranchContext) -> Vec<Finding>,
    ) -> Self {
        Self { name, text, check }
    }

    /// One-line summary: the first paragraph of [`Rule::text`] after the
    /// markdown title.
    pub fn summary(&self) -> &str {
        self.text
            .lines()
            .skip(1)
            .find(|line| !line.trim().is_empty())
            .unwrap_or(self.name)
    }
}

/// Registered lint rules.
pub const RULES: &[Rule] = &[
    case_over_if::RULE,
    deep_case_nesting::RULE,
    element_bif::RULE,
    newline_after_arrow::RULE,
    strict_generator::RULE,
];

/// Returns the span of the first token inside `node`'s range that satisfies
/// `predicate`, walking the node's tokens with [`erl_parse::NodeView::indexed_tokens`].
pub(crate) fn token_span_in_node(
    branch: &BranchContext,
    node: erl_parse::NodeView<'_>,
    predicate: impl Fn(erl_tokenize::Token) -> bool,
) -> Option<Span> {
    let index = node
        .indexed_tokens()
        .find_map(|(i, token)| predicate(token).then_some(i))?;
    token_span(branch, index)
}

/// Returns the span of the single token at `index`.
pub(crate) fn token_span(branch: &BranchContext, index: erl_parse::TokenIndex) -> Option<Span> {
    branch.span_of_range(erl_parse::TokenRange::single(index))
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// Runs a rule's `check` against `src` and returns the finding texts.
    pub(crate) fn findings(
        check: fn(&Context, &BranchContext) -> Vec<Finding>,
        src: &str,
    ) -> Vec<String> {
        let ctx = Context::analyze("t.erl", src.to_string()).expect("test source must scan");
        assert!(
            ctx.branches[0].tree.diagnostics().is_empty(),
            "parse diagnostics: {:?}",
            ctx.branches[0].tree.diagnostics()
        );
        check(&ctx, &ctx.branches[0])
            .into_iter()
            .map(|f| f.span.text(&ctx.text).to_string())
            .collect()
    }
}
