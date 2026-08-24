//! Erlang source linter.
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod expect;
pub mod fs;
pub mod rules;

mod context;
mod error;
mod span;

pub use context::{BranchContext, Context, PreprocessDiagnostic};
pub use error::Error;
pub use span::Span;

/// Result alias that uses [`Error`].
pub type Result<T = ()> = std::result::Result<T, Error>;
