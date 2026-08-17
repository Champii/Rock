use crate::lexer::Span;
use crate::lower::ResolveError;

#[derive(Debug, Default, Clone)]
pub(crate) struct LowerDiagnostics {
    errors: Vec<ResolveError>,
}

impl LowerDiagnostics {
    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn errors(&self) -> &[ResolveError] {
        &self.errors
    }

    pub(crate) fn into_errors(self) -> Vec<ResolveError> {
        self.errors
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct LowerDiagnosticSink {
    diagnostics: LowerDiagnostics,
    current_span: Option<Span>,
}

impl LowerDiagnosticSink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_current_span(&mut self, span: Span) {
        self.current_span = Some(span);
    }

    pub(crate) fn current_span(&self) -> &Span {
        self.current_span
            .as_ref()
            .expect("lowering operation must establish its source span")
    }

    pub(crate) fn push(&mut self, message: String) {
        self.diagnostics.errors.push(ResolveError {
            message,
            span: Some(self.current_span().clone()),
        });
    }

    pub(crate) fn push_once(&mut self, message: String) {
        if !self
            .diagnostics
            .errors
            .iter()
            .any(|error| error.message == message)
        {
            self.push(message);
        }
    }

    pub(crate) fn push_with_span(&mut self, message: String, span: Span) {
        self.diagnostics.errors.push(ResolveError {
            message,
            span: Some(span),
        });
    }

    pub(crate) fn push_toolchain(&mut self, message: String) {
        self.diagnostics.errors.push(ResolveError::new(message));
    }

    pub(crate) fn push_toolchain_once(&mut self, message: String) {
        if !self
            .diagnostics
            .errors
            .iter()
            .any(|error| error.message == message)
        {
            self.push_toolchain(message);
        }
    }

    pub(crate) fn extend(&mut self, errors: Vec<ResolveError>) {
        self.diagnostics.errors.extend(errors);
    }

    pub(crate) fn has_message(&self, message: &str) -> bool {
        self.diagnostics
            .errors
            .iter()
            .any(|error| error.message == message)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub(crate) fn errors(&self) -> &[ResolveError] {
        self.diagnostics.errors()
    }

    pub(crate) fn finish(self) -> LowerDiagnostics {
        self.diagnostics
    }

    pub(crate) fn into_errors(self) -> Vec<ResolveError> {
        self.finish().into_errors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::lexer::Span;

    #[test]
    fn diagnostics_attach_current_span_and_deduplicate_messages() {
        let mut diagnostics = LowerDiagnosticSink::new();
        diagnostics.set_current_span(Span {
            file_path: "main.rk".into(),
            start: 3,
            end: 9,
        });

        diagnostics.push("same message".to_string());
        diagnostics.push_once("same message".to_string());
        diagnostics.push_once("other message".to_string());

        assert_eq!(diagnostics.errors().len(), 2);
        assert_eq!(diagnostics.errors()[0].message, "same message");
        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().start, 3);
        assert_eq!(diagnostics.errors()[1].message, "other message");
    }

    #[test]
    fn diagnostics_push_with_explicit_span_overrides_current_span() {
        let mut diagnostics = LowerDiagnosticSink::new();
        diagnostics.set_current_span(Span {
            file_path: "main.rk".into(),
            start: 1,
            end: 2,
        });
        diagnostics.push_with_span(
            "explicit".to_string(),
            Span {
                file_path: "child.rk".into(),
                start: 10,
                end: 14,
            },
        );

        assert_eq!(
            diagnostics.errors()[0].span.as_ref().unwrap().file_path,
            PathBuf::from("child.rk")
        );
        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().start, 10);
    }

    #[test]
    fn diagnostic_sink_finishes_to_result_without_losing_span_or_deduplication() {
        let mut sink = LowerDiagnosticSink::new();
        sink.set_current_span(Span {
            file_path: "main.rk".into(),
            start: 3,
            end: 9,
        });

        sink.push("same message".to_string());
        sink.push_once("same message".to_string());
        sink.push_once("other message".to_string());

        let diagnostics = sink.finish();

        assert_eq!(diagnostics.errors().len(), 2);
        assert_eq!(diagnostics.errors()[0].message, "same message");
        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().start, 3);
        assert_eq!(diagnostics.errors()[1].message, "other message");
    }
}
