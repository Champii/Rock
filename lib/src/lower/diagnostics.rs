use crate::diagnostic::DiagnosticCode;
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
}

impl LowerDiagnosticSink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, message: String, span: Span) {
        self.push_with_code(message, span, DiagnosticCode::Resolve);
    }

    #[cfg(test)]
    pub(crate) fn push_once(&mut self, message: String, span: Span) {
        if !self
            .diagnostics
            .errors
            .iter()
            .any(|error| error.message == message)
        {
            self.push(message, span);
        }
    }

    pub(crate) fn push_with_span(&mut self, message: String, span: Span) {
        self.push(message, span);
    }

    pub(crate) fn push_with_span_and_labels(
        &mut self,
        message: String,
        span: Span,
        labels: Vec<crate::diagnostic::DiagnosticLabel>,
    ) {
        self.diagnostics
            .errors
            .push(ResolveError::with_span_and_labels(message, span, labels));
    }

    pub(crate) fn push_with_code(&mut self, message: String, span: Span, code: DiagnosticCode) {
        self.diagnostics
            .errors
            .push(ResolveError::with_span_code(message, span, code));
    }

    pub(crate) fn push_type_with_span(&mut self, message: String, span: Span) {
        self.push_with_code(message, span, DiagnosticCode::Type);
    }

    pub(crate) fn push_selection_with_span(&mut self, message: String, span: Span) {
        self.push_with_code(message, span, DiagnosticCode::Selection);
    }

    pub(crate) fn push_toolchain(&mut self, message: String) {
        self.diagnostics.errors.push(ResolveError::non_source_code(
            message,
            DiagnosticCode::Toolchain,
        ));
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
    fn diagnostics_require_explicit_spans_and_deduplicate_messages() {
        let mut diagnostics = LowerDiagnosticSink::new();
        let span = Span {
            file_path: "main.rk".into(),
            start: 3,
            end: 9,
        };

        diagnostics.push("same message".to_string(), span.clone());
        diagnostics.push_once("same message".to_string(), span.clone());
        diagnostics.push_once("other message".to_string(), span);

        assert_eq!(diagnostics.errors().len(), 2);
        assert_eq!(diagnostics.errors()[0].message, "same message");
        assert_eq!(diagnostics.errors()[0].span().unwrap().start, 3);
        assert_eq!(diagnostics.errors()[1].message, "other message");
    }

    #[test]
    fn diagnostics_push_with_explicit_span_preserves_operation_span() {
        let mut diagnostics = LowerDiagnosticSink::new();
        diagnostics.push_with_span(
            "explicit".to_string(),
            Span {
                file_path: "child.rk".into(),
                start: 10,
                end: 14,
            },
        );

        assert_eq!(
            diagnostics.errors()[0].span().unwrap().file_path,
            PathBuf::from("child.rk")
        );
        assert_eq!(diagnostics.errors()[0].span().unwrap().start, 10);
    }

    #[test]
    fn diagnostic_sink_finishes_to_result_without_losing_span_or_deduplication() {
        let mut sink = LowerDiagnosticSink::new();
        let span = Span {
            file_path: "main.rk".into(),
            start: 3,
            end: 9,
        };

        sink.push("same message".to_string(), span.clone());
        sink.push_once("same message".to_string(), span.clone());
        sink.push_once("other message".to_string(), span);

        let diagnostics = sink.finish();

        assert_eq!(diagnostics.errors().len(), 2);
        assert_eq!(diagnostics.errors()[0].message, "same message");
        assert_eq!(diagnostics.errors()[0].span().unwrap().start, 3);
        assert_eq!(diagnostics.errors()[1].message, "other message");
    }

    #[test]
    fn diagnostic_sink_assigns_stable_type_and_selection_codes() {
        let mut diagnostics = LowerDiagnosticSink::new();
        let span = Span {
            file_path: "main.rk".into(),
            start: 1,
            end: 2,
        };

        diagnostics.push_type_with_span("type mismatch".to_string(), span.clone());
        diagnostics.push_selection_with_span(
            "Call to unsafe function 'danger' requires an unsafe block".to_string(),
            span,
        );

        assert_eq!(diagnostics.errors()[0].code, DiagnosticCode::Type);
        assert_eq!(diagnostics.errors()[1].code, DiagnosticCode::Selection);
        assert!(diagnostics.errors()[1].message.contains("unsafe function"));
    }
}
