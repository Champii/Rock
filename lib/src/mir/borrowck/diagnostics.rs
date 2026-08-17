use crate::diagnostic::Diagnostic;
use crate::lexer::Span;
use crate::mir::Place;

pub fn borrow_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}

pub fn borrow_conflict(
    place: &Place,
    use_span: Option<Span>,
    borrow_span: Option<Span>,
) -> Diagnostic {
    let message = format!("borrow conflict on {:?}", place);
    let mut diagnostic = match use_span {
        Some(use_span) => Diagnostic::new(message, use_span.clone())
            .with_label("conflicting access".to_string(), use_span),
        None => Diagnostic::for_toolchain(message),
    };
    if let Some(borrow_span) = borrow_span {
        diagnostic = diagnostic.with_label("borrow introduced here".to_string(), borrow_span);
    }
    diagnostic
}
