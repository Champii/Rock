use std::fmt::Display;

use crate::parser::SourceFile;
use crate::parser::Span;

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

    pub fn new_unexpected_token(span: Span) -> Self {
        Self::new(span, DiagnosticKind::UnexpectedToken)
    }

    pub fn new_syntax_error(span: Span, msg: String) -> Self {
        Self::new(span, DiagnosticKind::SyntaxError(msg))
    }

    pub fn new_unknown_identifier(span: Span) -> Self {
        Self::new(span, DiagnosticKind::UnknownIdentifier)
    }

    pub fn new_module_not_found(span: Span) -> Self {
        Self::new(span, DiagnosticKind::ModuleNotFound)
    }

    pub fn new_duplicated_operator(span: Span) -> Self {
        Self::new(span, DiagnosticKind::DuplicatedOperator)
    }

    pub fn new_type_conflict(span: Span) -> Self {
        Self::new(span, DiagnosticKind::TypeConflict)
    }

    pub fn print(&self, file: &SourceFile) {
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
            "{}({}:{}) => ",
            file.file_path.to_str().unwrap(),
            line,
            line_start
        );

        let mut arrow = String::new();

        let mut i = 0;

        while i <= line_start {
            arrow.push(' ');

            i += 1;
        }

        arrow.push('^');

        println!(
            "{}[Error]: {}\n{}\n{}",
            line_ind,
            self.kind.to_string(),
            lines[line - 1].iter().cloned().collect::<String>(),
            arrow,
        );
    }

    pub fn get_kind(&self) -> DiagnosticKind {
        self.kind.clone()
    }
}

#[derive(Clone, Debug)]
pub enum DiagnosticKind {
    UnexpectedToken,
    SyntaxError(String),
    UnknownIdentifier,
    ModuleNotFound,
    NotAFunction,
    UnusedParameter,
    UnusedFunction,
    DuplicatedOperator,
    TypeConflict,
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
            Self::TypeConflict => "TypeConflict".to_string(),
            _ => "ERROR TBD".to_string(),
        };

        write!(f, "{}", s)
    }
}
