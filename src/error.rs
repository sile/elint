use std::backtrace::{Backtrace, BacktraceStatus};
use std::panic::Location;

use crate::span::Span;

/// Error type for elint.
///
/// Deliberately does not implement [`std::error::Error`] so that we can provide
/// explicit `From` impls for specific error types while keeping the rewrap
/// behavior controlled.
pub struct Error {
    /// Why the error occurred.
    pub reason: String,

    /// Position in the Erlang source that the error refers to.
    ///
    /// Set to [`Span::ZERO`] when no Erlang source position applies.
    pub span: Span,

    /// Rust source location where the `Error` was constructed.
    pub location: &'static Location<'static>,

    /// Backtrace captured at construction time.
    ///
    /// Only captured when `RUST_BACKTRACE` is set.
    pub backtrace: Backtrace,
}

impl Error {
    /// Creates a new [`Error`].
    #[track_caller]
    pub fn new<T: Into<String>>(span: Span, reason: T) -> Self {
        Self {
            reason: reason.into(),
            span,
            location: Location::caller(),
            backtrace: Backtrace::capture(),
        }
    }

    /// Prepends a context string to the error reason.
    pub fn with_context(mut self, context: impl AsRef<str>) -> Self {
        self.reason = format!("{}: {}", context.as_ref(), self.reason);
        self
    }

    fn fmt_detailed(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)?;
        if self.span != Span::ZERO {
            write!(f, " ({:?})", self.span)?;
        }
        write!(f, " (at {}:{})", self.location.file(), self.location.line())?;

        if self.backtrace.status() == BacktraceStatus::Disabled {
            write!(f, " [RUST_BACKTRACE=1 for backtrace]")?;
        }
        if self.backtrace.status() == BacktraceStatus::Captured {
            write!(f, "\n\nBacktrace:\n{}", self.backtrace)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_detailed(f)
    }
}

impl From<Error> for noargs::Error {
    fn from(error: Error) -> Self {
        noargs::Error::Other {
            metadata: None,
            error: format!("{error:?}"),
        }
    }
}

// We intentionally do not provide blanket `From<E: std::error::Error>` or
// `From<E: std::fmt::Display>` impls. Rewrapping our own `Error` through such
// a blanket would duplicate reason / location / backtrace in the output.
//
// To convert a new foreign error type into `crate::Error` via `?`, add a
// dedicated `impl From<NewError> for Error` in this file.
impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(e: std::io::Error) -> Self {
        Self::new(Span::ZERO, e.to_string())
    }
}

impl From<erl_tokenize::Error> for Error {
    #[track_caller]
    fn from(e: erl_tokenize::Error) -> Self {
        let offset = e.position.offset();
        Self::new(Span::new(offset, offset), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_context_adds_prefix() {
        let err = Error::new(Span::ZERO, "inner reason").with_context("outer context");
        assert_eq!(err.reason, "outer context: inner reason");
    }

    #[test]
    fn with_context_preserves_location_and_backtrace_status() {
        let err = Error::new(Span::ZERO, "inner");
        let location = err.location;
        let backtrace_status = err.backtrace.status();

        let err = err.with_context("outer");

        assert_eq!(err.location.file(), location.file());
        assert_eq!(err.location.line(), location.line());
        assert_eq!(err.backtrace.status(), backtrace_status);
    }

    #[test]
    fn from_error_to_noargs_error_uses_other_variant() {
        let err = Error::new(Span::ZERO, "reason");
        let noargs_err = noargs::Error::from(err);

        match noargs_err {
            noargs::Error::Other { metadata, .. } => {
                assert!(metadata.is_none());
            }
            _ => panic!("expected noargs::Error::Other"),
        }
    }
}
