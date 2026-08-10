use crate::diagnostic::Diagnostic;
use crate::lexer::Span;
use crate::mir::Place;

pub fn borrow_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}

pub fn borrow_conflict(place: &Place, use_span: Span, borrow_span: Span) -> Diagnostic {
    Diagnostic::new(format!("borrow conflict on {:?}", place), use_span.clone())
        .with_label("conflicting access".to_string(), use_span)
        .with_label("borrow introduced here".to_string(), borrow_span)
}
