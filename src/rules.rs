//! Lint rules.

mod element_bif;

use crate::Context;
use crate::Span;

/// One lint: a name, the markdown description, and a check function.
#[derive(Debug)]
pub struct Rule {
    /// Canonical name used in CLI output and `ELINT_EXPECT`.
    pub name: &'static str,
    /// Markdown text describing the rule.
    pub text: &'static str,
    /// Returns original-file spans that violate the rule.
    pub check: fn(&Context) -> Vec<Span>,
}

impl Rule {
    /// Builds a [`Rule`].
    pub const fn new(
        name: &'static str,
        text: &'static str,
        check: fn(&Context) -> Vec<Span>,
    ) -> Self {
        Self { name, text, check }
    }
}

/// Registered lint rules.
pub const RULES: &[Rule] = &[element_bif::RULE];
