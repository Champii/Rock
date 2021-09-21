use std::fmt::Display;

use crate::{
    ast::{Type, TypeSignature},
    hir::HirId,
    infer::TypeId,
    parser::SourceFile,
};
use crate::{diagnostics::DiagnosticType, parser::Span};
use colored::*;

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

    pub fn new_module_not_found(span: Span) -> Self {
        Self::new(span, DiagnosticKind::ModuleNotFound)
    }

    pub fn new_unresolved_type(span: Span, t: TypeId, hir_id: HirId) -> Self {
        Self::new(span, DiagnosticKind::UnresolvedType(t, hir_id))
    }

    pub fn new_unresolved_trait_call(
        span: Span,
        call_hir_id: HirId,
        given_sig: TypeSignature,
        existing_impls: Vec<TypeSignature>,
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

    pub fn new_no_main() -> Self {
        Self::new(Span::new_placeholder(), DiagnosticKind::NoMain)
    }
    pub fn new_type_conflict(span: Span, t1: Type, t2: Type, in1: Type, in2: Type) -> Self {
        Self::new(span, DiagnosticKind::TypeConflict(t1, t2, in1, in2))
    }

    pub fn print(&self, file: &SourceFile, diag_type: &DiagnosticType) {
        let input: Vec<char> = file.content.chars().collect();

        let line = input[..self.span.start].split(|c| *c == '\n').count();

        let lines: Vec<_> = input.split(|c| *c == '\n').collect();

        let count: usize = lines.clone()[..line - 1].iter().map(|v| v.len()).sum();

        let count = count + line;

        let line_start = if count > self.span.start {
            0
        } else {
            self.span.start - count
        };

        let line_ind = format!(
            " -> {}({}:{})",
            file.file_path.to_str().unwrap(),
            line,
            line_start
        );

        let mut arrow = String::new();

        let mut i = 0;

        while line_start > 0 && i <= line_start {
            arrow.push(' ');

            i += 1;
        }

        arrow.push('^');

        let mut i = 0;

        while i < self.span.end - self.span.start {
            arrow.push('~');

            i += 1;
        }

        let diag_type_str = match diag_type {
            DiagnosticType::Error => "Error".red(),
            DiagnosticType::Warning => "Warning".yellow(),
        };

        let color = |x: String| match diag_type {
            DiagnosticType::Error => x.red(),
            DiagnosticType::Warning => x.yellow(),
        };

        println!(
            "[{}]: {}\n{}\n{:>4} {}\n{:>4} {} {}\n{:>4} {} {}",
            diag_type_str,
            color(self.kind.to_string()).bold(),
            line_ind.bright_blue(),
            "",
            "|".cyan(),
            color(line.to_string()),
            "|".cyan(),
            lines[line - 1].iter().cloned().collect::<String>(),
            "",
            "|".cyan(),
            color(arrow),
        );
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
    ModuleNotFound,
    NotAFunction,
    UnusedParameter,
    UnresolvedTraitCall {
        call_hir_id: HirId,
        given_sig: TypeSignature,
        existing_impls: Vec<TypeSignature>,
    },
    UnusedFunction,
    DuplicatedOperator,
    TypeConflict(Type, Type, Type, Type),
    UnresolvedType(TypeId, HirId),
    CodegenError(HirId, String),
    NoMain,
    NoError, //TODO: remove that
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::UnexpectedToken => "UnexpectedToken".to_string(),
            Self::SyntaxError(msg) => format!("SyntaxError: {}", msg),
            Self::UnknownIdentifier => "UnknownIdentifier".to_string(),
            Self::ModuleNotFound => "ModuleNotFound".to_string(),
            Self::DuplicatedOperator => "DuplicatedOperator".to_string(),
            Self::TypeConflict(t1, t2, _in1, _in2) => {
                format!("Type conflict: Expected {:?} but got {:?} ", t1, t2)
            }
            Self::UnresolvedType(t_id, hir_id) => {
                format!("Unresolved type_id {} (hir_id {:?})", t_id, hir_id)
            }
            Self::UnresolvedTraitCall {
                call_hir_id: _,
                given_sig,
                existing_impls,
            } => {
                format!(
                    "Unresolved trait call {}\n{}",
                    given_sig,
                    existing_impls
                        .iter()
                        .map(|sig| format!("Found impl: {}", sig))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            Self::FileNotFound(path) => format!("FileNotFound {}", path),
            Self::CodegenError(hir_id, msg) => format!("CodegenError: {} {:?}", msg, hir_id),
            DiagnosticKind::NotAFunction => "NotAFunction".to_string(),
            DiagnosticKind::UnusedParameter => "UnusedParameter".to_string(),
            DiagnosticKind::UnusedFunction => "UnusedFunction".to_string(),
            DiagnosticKind::NoMain => "NoMain".to_string(),
            DiagnosticKind::NoError => "NoError".to_string(),
        };

        write!(f, "{}", s)
    }
}
