//! Erlang source linter.
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod context;
pub mod diagnostic;
mod error;
pub mod expect;
pub mod fs;
pub mod rules;
mod span;

pub use context::{BranchContext, Context, PreprocessDiagnostic, SkippedDefine, SkippedInclude};
pub use error::Error;
pub use span::{Finding, Span};

/// Result alias that uses [`Error`].
pub type Result<T = ()> = std::result::Result<T, Error>;
