use std::fmt::Display;

use crate::{diagnostics::DiagnosticType, parser::Span};
use crate::{
    hir::HirId,
    parser::SourceFile,
    ty::{FuncType, Type},
};
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

        let color_bright = |x: String| match diag_type {
            DiagnosticType::Error => x.bright_red(),
            DiagnosticType::Warning => x.bright_yellow(),
        };

        let diag_type_str = format!(
            "{}{}{} {}{}",
            "[".bright_black(),
            diag_type_str,
            "]".bright_black(),
            color(self.kind.to_string()).bold(),
            ":".bright_black(),
        );

        let line_span_start = line_start;
        let mut line_span_stop = line_start + (self.span.end - self.span.start) + 1;

        let line_colored = lines[line - 1].iter().cloned().collect::<String>();
        if line_span_stop > line_colored.len() {
            line_span_stop = line_colored.len() - 1;
        }

        let first_part = &line_colored[..line_span_start];
        let colored_part = color(line_colored[line_span_start..=line_span_stop].to_string());
        let last_part = if line_span_stop + 1 >= line_colored.len() {
            String::new()
        } else {
            line_colored[line_span_stop + 1..].to_owned()
        };

        let line_colored = format!("{}{}{}", first_part, colored_part, last_part,);

        println!(
            "{}\n{}\n{:>4} {}\n{:>4} {} {}\n{:>4} {} {}",
            diag_type_str,
            line_ind.bright_black(),
            "",
            "|".bright_black(),
            color_bright(line.to_string()),
            "|".bright_black(),
            line_colored,
            "",
            "|".bright_black(),
            color_bright(arrow),
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
    TypeConflict(Type, Type, Type, Type),
    UnresolvedType(Type),
    CodegenError(HirId, String),
    OutOfBounds(u64, u64),
    NoMain,
    NoError, //TODO: remove that
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::UnexpectedToken => "UnexpectedToken".to_string(),
            Self::SyntaxError(msg) => format!("SyntaxError: {}", msg),
            Self::UnknownIdentifier => "UnknownIdentifier".to_string(),
            Self::ModuleNotFound(path) => format!("Module not found: {}", path),
            Self::DuplicatedOperator => "DuplicatedOperator".to_string(),
            Self::TypeConflict(t1, t2, _in1, _in2) => {
                format!("Type conflict: Expected {:?} but got {:?} ", t1, t2)
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
                existing_impls,
            } => {
                format!(
                    "Unresolved trait call {:?}\n{}",
                    given_sig,
                    existing_impls
                        .iter()
                        .map(|sig| format!("        Found impl: {:?}", sig))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
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
            DiagnosticKind::NoMain => "NoMain".to_string(),
            DiagnosticKind::NoError => "NoError".to_string(),
        };

        write!(f, "{}", s)
    }
}
