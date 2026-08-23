//! Erlang source linter.
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod expect;
pub mod fs;

mod context;
mod error;
mod rule_element_bif;
mod span;

pub use context::{Context, PreprocessDiagnostic};
pub use error::Error;
pub use span::Span;

/// Result alias that uses [`Error`].
pub type Result<T = ()> = std::result::Result<T, Error>;

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
pub const RULES: &[Rule] = &[rule_element_bif::RULE];
