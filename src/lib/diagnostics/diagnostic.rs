use ariadne::{Color, Label, Report, ReportKind, Source};
use std::fmt::Display;

use crate::parser::Parser;
use crate::{diagnostics::DiagnosticType, parser::span::Span};
use crate::{
    hir::HirId,
    parser::SourceFile,
    ty::{FuncType, Type},
};
use nom::error::{VerboseError, VerboseErrorKind};

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub span: Span,
    kind: DiagnosticKind,
}

impl Diagnostic {
    pub fn new(span: Span, kind: DiagnosticKind) -> Self {
        Self { span, kind }
    }

    pub fn new_empty() -> Self {
        Self {
            span: Span::new_placeholder(),
            kind: DiagnosticKind::NoError,
        }
    }

    pub fn new_file_not_found(span: Span, path: String) -> Self {
        Self::new(span, DiagnosticKind::FileNotFound(path))
    }

    pub fn new_unexpected_token(span: Span) -> Self {
        Self::new(span, DiagnosticKind::UnexpectedToken)
    }

    pub fn new_syntax_error(span: Span, msg: String) -> Self {
        Self::new(span, DiagnosticKind::SyntaxError(msg))
    }

    pub fn new_unknown_identifier(span: Span) -> Self {
        Self::new(span, DiagnosticKind::UnknownIdentifier)
    }

    pub fn new_unused_function(span: Span) -> Self {
        Self::new(span, DiagnosticKind::UnusedFunction)
    }

    pub fn new_module_not_found(span: Span, path: String) -> Self {
        Self::new(span, DiagnosticKind::ModuleNotFound(path))
    }

    pub fn new_unresolved_type(span: Span, t: Type) -> Self {
        Self::new(span, DiagnosticKind::UnresolvedType(t))
    }

    pub fn new_out_of_bounds(span: Span, got: u64, expected: u64) -> Self {
        Self::new(span, DiagnosticKind::OutOfBounds(got, expected))
    }

    pub fn new_unresolved_trait_call(
        span: Span,
        call_hir_id: HirId,
        given_sig: FuncType,
        existing_impls: Vec<FuncType>,
    ) -> Self {
        Self::new(
            span,
            DiagnosticKind::UnresolvedTraitCall {
                call_hir_id,
                given_sig,
                existing_impls,
            },
        )
    }

    pub fn new_codegen_error(span: Span, hir_id: HirId, msg: &str) -> Self {
        Self::new(span, DiagnosticKind::CodegenError(hir_id, msg.to_string()))
    }

    pub fn new_duplicated_operator(span: Span) -> Self {
        Self::new(span, DiagnosticKind::DuplicatedOperator)
    }

    pub fn new_orphane_signature(span: Span, name: String) -> Self {
        Self::new(span, DiagnosticKind::OrphaneSignature(name))
    }

    pub fn new_no_main() -> Self {
        Self::new(Span::new_placeholder(), DiagnosticKind::NoMain)
    }

    pub fn new_is_not_a_property_of(span: Span, span2: Span, t: Type) -> Self {
        Self::new(span, DiagnosticKind::IsNotAPropertyOf(t, span2))
    }

    pub fn new_type_conflict(span: Span, expected: Type, got: Type, in1: Type, in2: Type) -> Self {
        Self::new(span, DiagnosticKind::TypeConflict(expected, got, in1, in2))
    }

    pub fn print(&self, file: &SourceFile, diag_type: &DiagnosticType) {
        self.kind.report_builder(file, &self.span, diag_type);
    }

    pub fn get_kind(&self) -> DiagnosticKind {
        self.kind.clone()
    }
}

#[derive(Clone, Debug)]
pub enum DiagnosticKind {
    FileNotFound(String),
    UnexpectedToken,
    SyntaxError(String),
    UnknownIdentifier,
    ModuleNotFound(String),
    NotAFunction,
    UnusedParameter,
    UnresolvedTraitCall {
        call_hir_id: HirId,
        given_sig: FuncType,
        existing_impls: Vec<FuncType>,
    },
    UnusedFunction,
    DuplicatedOperator,
    TypeConflict(Type, Type, Type, Type), // expected -> got
    UnresolvedType(Type),
    CodegenError(HirId, String),
    IsNotAPropertyOf(Type, Span),
    OutOfBounds(u64, u64),
    OrphaneSignature(String),
    NoMain,
    NoError, //TODO: remove that
}

impl DiagnosticKind {
    pub fn report_builder<'a>(
        &self,
        file: &SourceFile,
        span: &'a Span,
        diag_type: &DiagnosticType,
    ) {
        let filename = file.file_path.to_str().unwrap();

        let (error_ty, color) = match diag_type {
            DiagnosticType::Error => (ReportKind::Error, Color::Red),
            DiagnosticType::Warning => (ReportKind::Warning, Color::Yellow),
        };
        let builder = Report::build(error_ty, filename, span.start);

        let mut span = span.clone();
        if span.start == span.end {
            span.end += 1;
        }

        match self {
            DiagnosticKind::FileNotFound(path) => builder
                .with_message(format!("File not found: {}", path))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::UnexpectedToken => builder
                .with_message("Unexpected token".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::SyntaxError(msg) => builder
                .with_message(format!("Syntax error: {}", msg))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::UnknownIdentifier => builder
                .with_message("Unknown identifier".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::ModuleNotFound(path) => builder
                .with_message(format!("Module not found: {}", path))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::UnusedFunction => builder
                .with_message("Unused function".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::UnusedParameter => builder
                .with_message("Unused parameter".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::UnresolvedTraitCall {
                call_hir_id: _,
                given_sig: _,
                existing_impls,
            } => {
                let note = &format!(
                    "Existing implementations ({}): {}{}",
                    existing_impls.len(),
                    existing_impls
                        .iter()
                        .take(3)
                        .map(|t| format!("\n            - {:?}", t))
                        .collect::<Vec<String>>()
                        .join(", "),
                    if existing_impls.len() > 3 {
                        "\n            - ..."
                    } else {
                        ""
                    },
                );
                builder
                    .with_message(format!("{}", self))
                    .with_note(note)
                    .with_label(
                        Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                            .with_message("Unresolved trait call")
                            .with_color(color),
                    )
            }
            DiagnosticKind::UnresolvedType(t) => builder
                .with_message(format!("Unresolved type: {}", t.to_string()))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::CodegenError(_hir_id, msg) => builder
                .with_message(format!("Codegen error: {}", msg))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::IsNotAPropertyOf(t, span2) => builder
                .with_message(format!("{}", self,))
                .with_label(
                    Label::new((span2.file_path.to_str().unwrap(), span2.start..span2.end))
                        .with_message(format!("This is of type {:?}", t))
                        .with_color(Color::Blue),
                )
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::TypeConflict(_t1, _t2, _in1, _in2) => {
                // add spans here
                builder.with_message("Type conflict").with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                )
            }
            DiagnosticKind::OutOfBounds(got, expected) => builder
                .with_message(format!("Out of bounds: got {}, expected {}", got, expected))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::OrphaneSignature(name) => builder
                .with_message(format!("Orpheline signature: {}", name))
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::NoMain => builder
                .with_message("No main function".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::NoError => builder.with_message("No error".to_string()).with_label(
                Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                    .with_message(format!("{}", self))
                    .with_color(color),
            ),
            DiagnosticKind::DuplicatedOperator => builder
                .with_message("Duplicated operator".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
            DiagnosticKind::NotAFunction => builder
                .with_message("Not a function".to_string())
                .with_label(
                    Label::new((span.file_path.to_str().unwrap(), span.start..span.end))
                        .with_message(format!("{}", self))
                        .with_color(color),
                ),
        }
        .finish()
        .print((filename, Source::from(file.content.clone())))
        .unwrap();
    }
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::UnexpectedToken => "UnexpectedToken".to_string(),
            Self::SyntaxError(msg) => format!("SyntaxError: {}", msg),
            Self::UnknownIdentifier => "UnknownIdentifier".to_string(),
            Self::ModuleNotFound(path) => format!("Module not found: {}", path),
            Self::DuplicatedOperator => "DuplicatedOperator".to_string(),
            Self::TypeConflict(expected, got, _in1, _in2) => {
                use colored::*;
                format!(
                    "Expected {}\n{:<18}But got  {}",
                    format!("{}", expected).blue(),
                    "",
                    format!("{}", got).red(),
                )
            }
            Self::UnresolvedType(t) => {
                format!(
                    "Unresolved type: Type {:?} should be known at this point",
                    t
                )
            }
            Self::UnresolvedTraitCall {
                call_hir_id: _,
                given_sig,
                existing_impls: _,
            } => {
                format!("Unresolved trait call: {:?}", given_sig,)
            }
            Self::FileNotFound(path) => format!("FileNotFound {}", path),
            Self::CodegenError(hir_id, msg) => format!("CodegenError: {} {:?}", msg, hir_id),
            Self::OutOfBounds(got, expected) => format!(
                "Out of bounds error: got indice {} but array len is {}",
                got, expected
            ),
            DiagnosticKind::NotAFunction => "NotAFunction".to_string(),
            DiagnosticKind::UnusedParameter => "UnusedParameter".to_string(),
            DiagnosticKind::UnusedFunction => "UnusedFunction".to_string(),
            DiagnosticKind::OrphaneSignature(_name) => "OrphelineSignature".to_string(),
            DiagnosticKind::NoMain => "NoMain".to_string(),
            DiagnosticKind::NoError => "NoError".to_string(),
            DiagnosticKind::IsNotAPropertyOf(t, _span2) => {
                format!("Not a property of {:?}", t)
            }
        };

        write!(f, "{}", s)
    }
}

impl<'a> From<Parser<'a>> for Diagnostic {
    fn from(err: Parser<'a>) -> Self {
        let span = Span::from(err);

        let msg = "Syntax error".to_string();

        Diagnostic::new_syntax_error(span, msg)
    }
}

impl<'a> From<VerboseError<Parser<'a>>> for Diagnostic {
    fn from(err: VerboseError<Parser<'a>>) -> Self {
        let (input, _kind) = err.errors.iter().next().unwrap().clone();

        let span = Span::from(input);

        let msg = err.to_string();

        Diagnostic::new_syntax_error(span, msg)
    }
}

impl<I> From<(I, VerboseErrorKind)> for Diagnostic
where
    Span: From<I>,
{
    fn from((input, _kind): (I, VerboseErrorKind)) -> Self {
        let span = Span::from(input);

        let msg = "Syntax error".to_string();

        Diagnostic::new_syntax_error(span, msg)
    }
}
