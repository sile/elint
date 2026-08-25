//! Lint rules.

mod element_bif;
mod newline_after_arrow;

use crate::{BranchContext, Context, Span};

/// One lint: a name, the markdown description, and a check function.
#[derive(Debug)]
pub struct Rule {
    /// Canonical name used in CLI output and `-elint_expect`.
    pub name: &'static str,
    /// Markdown text describing the rule.
    pub text: &'static str,
    /// Returns original-file spans that violate the rule in one branch.
    pub check: fn(&Context, &BranchContext) -> Vec<Span>,
}

impl Rule {
    /// Builds a [`Rule`].
    pub const fn new(
        name: &'static str,
        text: &'static str,
        check: fn(&Context, &BranchContext) -> Vec<Span>,
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
pub const RULES: &[Rule] = &[element_bif::RULE, newline_after_arrow::RULE];
