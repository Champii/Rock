//! Resolution error types

use crate::diagnostic::SpannedError;
use crate::lexer::Span;

/// A resolution error with an optional source location
#[derive(Debug, Clone)]
pub struct ResolveError {
    pub message: String,
    pub span: Option<Span>,
}

impl ResolveError {
    /// Create a new resolve error with a message
    pub fn new(message: String) -> Self {
        Self {
            message,
            span: None,
        }
    }

    /// Create a new resolve error with a message and span
    pub fn with_span(message: String, span: Span) -> Self {
        Self {
            message,
            span: Some(span),
        }
    }
}

impl SpannedError for ResolveError {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn span(&self) -> Option<Span> {
        self.span.clone()
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
