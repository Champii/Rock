use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::lexer::Span;
use crate::mir::Place;

pub fn borrow_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(message.into(), span).with_code(DiagnosticCode::Borrow)
}

pub fn borrow_conflict(
    _place: &Place,
    use_span: Option<Span>,
    borrow_span: Option<Span>,
) -> Diagnostic {
    let Some(use_span) = use_span else {
        return Diagnostic::for_internal("borrow conflict is missing its use origin");
    };
    let Some(borrow_span) = borrow_span else {
        return Diagnostic::for_internal("borrow conflict is missing its loan origin");
    };
    Diagnostic::new(
        "borrow conflict on this value".to_string(),
        use_span.clone(),
    )
    .with_code(DiagnosticCode::Borrow)
    .with_label("conflicting access".to_string(), use_span)
    .with_label("borrow introduced here".to_string(), borrow_span)
}

#[cfg(test)]
mod tests {
    use super::borrow_conflict;
    use crate::diagnostic::{DiagnosticCode, DiagnosticLocation};
    use crate::lexer::Span;
    use crate::mir::{Local, Place};

    #[test]
    fn borrow_conflict_uses_use_and_loan_origins_as_labels() {
        let use_span = Span::new("/virtual/main.rk".into(), 4, 7);
        let loan_span = Span::new("/virtual/main.rk".into(), 12, 15);
        let diagnostic = borrow_conflict(
            &Place {
                local: Local(1),
                projection: vec![],
            },
            Some(use_span.clone()),
            Some(loan_span.clone()),
        );

        assert_eq!(diagnostic.code, Some(DiagnosticCode::Borrow));
        assert_eq!(diagnostic.location, DiagnosticLocation::Source(use_span));
        assert_eq!(diagnostic.primary.as_ref().unwrap().span.start, 4);
        assert_eq!(diagnostic.secondary.len(), 2);
        assert_eq!(diagnostic.secondary[1].span.start, loan_span.start);
    }

    #[test]
    fn borrow_conflict_without_origin_is_internal() {
        let diagnostic = borrow_conflict(
            &Place {
                local: Local(1),
                projection: vec![],
            },
            None,
            None,
        );

        assert_eq!(diagnostic.code, Some(DiagnosticCode::Internal));
        assert_eq!(diagnostic.location, DiagnosticLocation::Toolchain);
        assert!(diagnostic.primary.is_none());
    }
}
