use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ariadne::{Color, ColorGenerator, Label, Report, ReportKind};

use crate::lexer::{LexerError, Span};
use crate::parser::ParseError;

/// Trait for errors that carry span information.
/// Any error type implementing this trait can be converted to a Diagnostic.
pub trait SpannedError {
    /// The error message
    fn message(&self) -> String;
    /// The primary span where the error occurred
    fn span(&self) -> Option<Span>;
    /// Additional labels with their own spans
    fn labels(&self) -> Vec<(String, Span)> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub enum DiagnosticType {
    Error,
    /* Warning,
    Note, */
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLocation {
    Source(Span),
    File(PathBuf),
    Project(PathBuf),
    Artifact(PathBuf),
    Toolchain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSourceOrigin {
    FileSystem,
    Virtual,
    Artifact { artifact_path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSource {
    pub display_path: PathBuf,
    pub text: String,
    pub origin: DiagnosticSourceOrigin,
    pub related: BTreeMap<PathBuf, DiagnosticRelatedSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticRelatedSource {
    pub display_path: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct DiagnosticSourceMap {
    by_path: BTreeMap<PathBuf, DiagnosticSource>,
}

impl DiagnosticSourceMap {
    pub fn from_source_database(database: &crate::source_loader::SourceDatabase) -> Self {
        let mut sources = Self::default();
        for file in database.source_files() {
            sources.insert_source_file(file);
        }
        sources
    }

    pub fn source_for_path(&self, path: &Path) -> Option<&DiagnosticSource> {
        self.by_path.get(path)
    }

    pub fn attach(&self, diagnostic: &mut Diagnostic) {
        let DiagnosticLocation::Source(primary_span) = &diagnostic.location else {
            return;
        };
        let source = diagnostic
            .source
            .clone()
            .or_else(|| self.source_for_path(&primary_span.file_path).cloned());
        if let Some(mut source) = source {
            for (_, span) in &diagnostic.labels {
                if span.file_path != primary_span.file_path {
                    if let Some(related) = self.source_for_path(&span.file_path) {
                        source.related.insert(
                            span.file_path.clone(),
                            DiagnosticRelatedSource {
                                display_path: related.display_path.clone(),
                                text: related.text.clone(),
                            },
                        );
                    }
                }
            }
            diagnostic.source = Some(source);
            if diagnostic.labels.is_empty() {
                diagnostic
                    .labels
                    .push((diagnostic.message.clone(), primary_span.clone()));
            }
        }
    }

    fn insert_source_file(&mut self, file: &crate::source_loader::SourceFile) {
        let origin = match &file.origin {
            crate::source_loader::SourceOrigin::FileSystem => DiagnosticSourceOrigin::FileSystem,
            crate::source_loader::SourceOrigin::Virtual => DiagnosticSourceOrigin::Virtual,
            crate::source_loader::SourceOrigin::Artifact { artifact_path } => {
                DiagnosticSourceOrigin::Artifact {
                    artifact_path: artifact_path.clone(),
                }
            }
        };
        let source = DiagnosticSource {
            display_path: file.display_path.clone(),
            text: file.text.clone(),
            origin,
            related: BTreeMap::new(),
        };
        for path in [
            &file.original_path,
            &file.canonical_path,
            &file.display_path,
        ] {
            self.by_path.insert(path.clone(), source.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub labels: Vec<(String, Span)>,
    pub kind: DiagnosticType,
    pub source: Option<DiagnosticSource>,
    pub location: DiagnosticLocation,
}

impl From<ParseError> for Diagnostic {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::UnexpectedToken(_expected_desc, got_token) => {
                let got_display = if got_token.token_type.to_string().is_empty() {
                    format!("end of file")
                } else {
                    format!("'{}'", got_token.token_type.to_string())
                };

                Diagnostic {
                    message: format!("Unexpected token: {}", got_display),
                    labels: vec![(
                        format!("Unexpected {}", got_display),
                        got_token.span.clone(),
                    )],
                    kind: DiagnosticType::Error,
                    source: None,
                    location: DiagnosticLocation::Source(got_token.span.clone()),
                }
            }
            /* ParseError::UnexpectedKeyword(token, expected) => Diagnostic {
                message: format!("Unexpected keyword: {:?}", token.token_type,),
                labels: vec![(
                    format!("Expected one of {:?}", expected),
                    token.span.clone(),
                )],
                span: token.span,
                kind: DiagnosticType::Error,
            }, */
            ParseError::Lexer(LexerError::UnknownToken(c, span)) => Diagnostic {
                message: format!("Lexer: Unknown token: {:?}", c),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Source(span.clone()),
            },
            ParseError::MacroNoCorrespondance {
                macro_name,
                invoc_name,
                invoc_arg,
            } => {
                let mut labels = vec![
                    ("For this macro".to_string(), macro_name.clone()),
                    ("In this macro invocation".to_string(), invoc_name.clone()),
                ];

                if let Some(invoc_arg) = invoc_arg {
                    labels.push(("With this token".to_string(), invoc_arg.clone()));
                }

                Diagnostic {
                    message: "Macro: Nothing expected this token".to_string(),
                    labels,
                    kind: DiagnosticType::Error,
                    source: None,
                    location: DiagnosticLocation::Source(macro_name),
                }
            }
            /* ParseError::InvalidPrecedence(precedence, token) => Diagnostic {
                message: format!("Invalid precedence: {:?}", precedence),
                labels: vec![(
                    format!("Precedence must be between 0 and 9"),
                    token.span.clone(),
                )],
                span: token.span,
                kind: DiagnosticType::Error,
            },
            ParseError::IndentMismatch(got, expected, span) => Diagnostic {
                message: format!("Indent mismatch: got {}, expected {}", got, expected),
                labels: vec![
                    (format!("Expected indent level: {}", expected), span.clone()),
                    (format!("Got indent level: {}", got), span.clone()),
                ],
                kind: DiagnosticType::Error,
            }, */
            ParseError::UnexpectedEOF(span) => Diagnostic {
                message: "Unexpected end of file".to_string(),
                labels: vec![("Expected more input".to_string(), span.clone())],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Source(span.clone()),
            },
            /* ParseError::LeftoverTokens(tokens) => Diagnostic {
                message: format!("Leftover tokens"),
                labels: vec![(format!("Expected end of file"), tokens[0].span.clone())],
                span: tokens[0].span.clone(),
                kind: DiagnosticType::Error,
            }, */
            ParseError::UnknownFile(file) => Diagnostic {
                message: format!("Unknown file: {:?}", file),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::File(PathBuf::from(file)),
            },
            ParseError::UnexpectedIndent(level) => Diagnostic {
                message: format!("Unexpected indent level: {level}"),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Toolchain,
            },
            ParseError::ExpectedOneOrMore => Diagnostic {
                message: "Expected one or more".to_string(),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Toolchain,
            },
            ParseError::Fail => Diagnostic {
                message: "Fail".to_string(),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Toolchain,
            },
            ParseError::ShortCircuit => Diagnostic {
                message: "Short circuit, should never be printed !".to_string(),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Toolchain,
            },
            ParseError::AssertFailed => Diagnostic {
                message: "Assert failed".to_string(),
                labels: vec![],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Toolchain,
            },
            ParseError::HardError(msg, span) => Diagnostic {
                message: msg.clone(),
                labels: vec![(msg, span.clone())],
                kind: DiagnosticType::Error,
                source: None,
                location: DiagnosticLocation::Source(span.clone()),
            },
            ParseError::WithContext { context, error } => {
                // Build the full context chain by collecting all contexts
                // and find the innermost non-context error
                let mut context_chain = vec![context.clone()];
                let mut current_error = error.as_ref();

                // Unwrap all nested WithContext layers
                loop {
                    match current_error {
                        ParseError::WithContext {
                            context: inner_context,
                            error: inner_error,
                        } => {
                            context_chain.push(inner_context.clone());
                            current_error = inner_error.as_ref();
                        }
                        _ => break,
                    }
                }

                // Now current_error is the innermost non-context error
                // Get its base message and span
                let (base_message, span) = match current_error {
                    ParseError::UnexpectedToken(_expected_desc, got_token) => {
                        let got_display = if got_token.token_type.to_string().is_empty() {
                            format!("end of file")
                        } else {
                            format!("'{}'", got_token.token_type.to_string())
                        };
                        (
                            format!("Unexpected token: {}", got_display),
                            Some(got_token.span.clone()),
                        )
                    }
                    ParseError::UnexpectedEOF(span) => {
                        ("Unexpected end of file".to_string(), Some(span.clone()))
                    }
                    ParseError::UnexpectedIndent(level) => {
                        (format!("Unexpected indent level: {}", level), None)
                    }
                    _ => ("Parse error".to_string(), None),
                };

                // Create diagnostic with context
                let message = format!(
                    "{}\n\nParsing context (innermost first):\n{}",
                    base_message,
                    context_chain
                        .iter()
                        .enumerate()
                        .map(|(i, ctx)| format!("  {}. {}", i + 1, ctx))
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                // Get the label from the innermost error
                let labels = match current_error {
                    ParseError::UnexpectedToken(_expected_desc, got_token) => {
                        let got_display = if got_token.token_type.to_string().is_empty() {
                            format!("end of file")
                        } else {
                            format!("'{}'", got_token.token_type.to_string())
                        };
                        vec![(
                            format!("Unexpected {}", got_display),
                            got_token.span.clone(),
                        )]
                    }
                    ParseError::UnexpectedEOF(span) => {
                        vec![("Expected more input".to_string(), span.clone())]
                    }
                    _ => vec![],
                };

                Diagnostic {
                    message,
                    labels,
                    kind: DiagnosticType::Error,
                    source: None,
                    location: span
                        .map(DiagnosticLocation::Source)
                        .unwrap_or(DiagnosticLocation::Toolchain),
                }
            }
        }
    }
}

impl Diagnostic {
    /// Create a new diagnostic with a message and primary span
    pub fn new(message: String, span: Span) -> Self {
        assert!(
            !span.file_path.as_os_str().is_empty(),
            "source diagnostics require a source path"
        );
        assert!(
            span.start <= span.end,
            "source diagnostic span start must not exceed its end"
        );
        let labels = vec![(message.clone(), span.clone())];
        Self {
            message,
            location: DiagnosticLocation::Source(span),
            labels,
            kind: DiagnosticType::Error,
            source: None,
        }
    }

    pub fn for_file(message: String, path: PathBuf) -> Self {
        Self::at_location(message, DiagnosticLocation::File(path))
    }

    pub fn for_project(message: String, path: PathBuf) -> Self {
        Self::at_location(message, DiagnosticLocation::Project(path))
    }

    pub fn for_artifact(message: String, path: PathBuf) -> Self {
        Self::at_location(message, DiagnosticLocation::Artifact(path))
    }

    pub fn for_toolchain(message: String) -> Self {
        Self::at_location(message, DiagnosticLocation::Toolchain)
    }

    fn at_location(message: String, location: DiagnosticLocation) -> Self {
        Self {
            message,
            labels: Vec::new(),
            kind: DiagnosticType::Error,
            source: None,
            location,
        }
    }

    /// Add a label with its own span
    pub fn with_label(mut self, label: String, span: Span) -> Self {
        self.labels.push((label, span));
        self
    }

    pub fn with_labels(mut self, labels: Vec<(String, Span)>) -> Self {
        self.labels.extend(labels);
        self
    }

    pub fn with_source(mut self, source: DiagnosticSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Create a diagnostic from any SpannedError
    pub fn from_spanned<E: SpannedError>(err: &E) -> Self {
        let span = err.span();
        let mut labels = err.labels();

        // Virtual and artifact sources need labels even when no filesystem path exists.
        if labels.is_empty() {
            if let Some(span) = &span {
                labels.push((err.message(), span.clone()));
            }
        }

        Self {
            message: err.message(),
            labels,
            location: span
                .map(DiagnosticLocation::Source)
                .unwrap_or(DiagnosticLocation::Toolchain),
            kind: DiagnosticType::Error,
            source: None,
        }
    }

    pub fn report(&self) {
        let source_content = match &self.location {
            DiagnosticLocation::Source(span) => {
                if let Some(source) = &self.source {
                    Some((
                        source.display_path.to_string_lossy().to_string(),
                        source.text.clone(),
                        source.related.clone(),
                        span.clone(),
                    ))
                } else if span.file_path.exists() {
                    std::fs::read_to_string(&span.file_path).ok().map(|source| {
                        (
                            span.file_path.to_string_lossy().to_string(),
                            source,
                            BTreeMap::new(),
                            span.clone(),
                        )
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((file_id, source, related_sources, primary_span)) = source_content {
            // File exists - use ariadne for pretty printing
            let mut colors = ColorGenerator::new();

            let mut builder = Report::build(ReportKind::Error, file_id.clone(), primary_span.start)
                .with_message(self.message.clone());

            for (i, (message, span)) in self.labels.iter().enumerate() {
                let label_file_id = if span.file_path == primary_span.file_path {
                    file_id.clone()
                } else if let Some(related) = related_sources.get(&span.file_path) {
                    related.display_path.to_string_lossy().to_string()
                } else {
                    span.file_path.to_string_lossy().to_string()
                };
                let color = if i == 0 {
                    Color::Fixed(9)
                } else {
                    colors.next()
                };
                builder = builder.with_label(
                    Label::new((label_file_id, span.start..span.end))
                        .with_message(message)
                        .with_color(color),
                );
            }

            let mut report_sources = BTreeMap::from([(file_id, source)]);
            for related in related_sources.values() {
                report_sources.insert(
                    related.display_path.to_string_lossy().to_string(),
                    related.text.clone(),
                );
            }
            for (_, span) in &self.labels {
                if span.file_path != primary_span.file_path
                    && !related_sources.contains_key(&span.file_path)
                {
                    if let Ok(text) = std::fs::read_to_string(&span.file_path) {
                        report_sources.insert(span.file_path.to_string_lossy().to_string(), text);
                    }
                }
            }
            builder
                .finish()
                .print(ariadne::sources(report_sources))
                .unwrap();

            println!();
        } else {
            // No source available - print a simple error message with location info
            eprintln!("Error: {}", self.message);

            match &self.location {
                DiagnosticLocation::Source(span) => eprintln!(
                    "  --> {}:{}-{}",
                    span.file_path.display(),
                    span.start,
                    span.end
                ),
                DiagnosticLocation::File(path) => eprintln!("  --> file {}", path.display()),
                DiagnosticLocation::Project(path) => {
                    eprintln!("  --> project {}", path.display())
                }
                DiagnosticLocation::Artifact(path) => {
                    eprintln!("  --> artifact {}", path.display())
                }
                DiagnosticLocation::Toolchain => eprintln!("  --> toolchain"),
            }

            for (message, span) in &self.labels {
                if !span.file_path.as_os_str().is_empty() {
                    eprintln!(
                        "      {}: {} ({}-{})",
                        message,
                        span.file_path.display(),
                        span.start,
                        span.end
                    );
                }
            }

            println!();
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn push<T: Into<Diagnostic>>(&mut self, diagnostic: T) -> Self {
        let diagnostic = diagnostic.into();

        self.0.push(diagnostic);

        self.clone()
    }

    /// Push a SpannedError as a diagnostic
    pub fn push_error<E: SpannedError>(&mut self, err: &E) -> Self {
        self.0.push(Diagnostic::from_spanned(err));
        self.clone()
    }

    /// Push multiple errors
    pub fn extend_errors<E: SpannedError>(&mut self, errors: &[E]) -> Self {
        for err in errors {
            self.0.push(Diagnostic::from_spanned(err));
        }
        self.clone()
    }

    pub fn merge(&mut self, mut diagnostics: Diagnostics) {
        self.0.append(&mut diagnostics.0);
    }

    pub fn attach_sources(&mut self, sources: &DiagnosticSourceMap) {
        for diagnostic in &mut self.0 {
            sources.attach(diagnostic);
        }
    }

    pub fn with_sources(mut self, sources: &DiagnosticSourceMap) -> Self {
        self.attach_sources(sources);
        self
    }

    pub fn report(&self) {
        for diagnostic in &self.0 {
            diagnostic.report();
        }
    }

    pub fn return_if_err(&self) -> Result<(), Self> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self.clone())
        }
    }
}

impl From<ParseError> for Diagnostics {
    fn from(err: ParseError) -> Self {
        let mut diagnostics = Diagnostics::default();

        diagnostics.push(Diagnostic::from(err));

        diagnostics
    }
}

impl<E: SpannedError> From<E> for Diagnostic {
    fn from(err: E) -> Self {
        Diagnostic::from_spanned(&err)
    }
}

impl<E: SpannedError + Clone> From<Vec<E>> for Diagnostics {
    fn from(errors: Vec<E>) -> Self {
        let mut diagnostics = Diagnostics::default();
        for err in errors {
            diagnostics.push(Diagnostic::from_spanned(&err));
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    use crate::{Config, SourceProvider};

    #[test]
    fn diagnostic_report_handles_more_than_three_labels() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let source_path = std::env::temp_dir().join(format!(
            "rock-diagnostic-many-labels-{}-{nonce}.rk",
            std::process::id()
        ));
        fs::write(&source_path, "main = -> 0\n").unwrap();
        let span = Span {
            file_path: source_path.clone(),
            start: 0,
            end: 4,
        };
        let diagnostic =
            Diagnostic::new("many labels".to_string(), span.clone()).with_labels(vec![
                ("first".to_string(), span.clone()),
                ("second".to_string(), span.clone()),
                ("third".to_string(), span.clone()),
                ("fourth".to_string(), span.clone()),
            ]);

        let result = std::panic::catch_unwind(|| diagnostic.report());

        let _ = fs::remove_file(source_path);
        assert!(result.is_ok());
    }

    #[test]
    fn file_diagnostic_does_not_fabricate_a_source_range() {
        let path = PathBuf::from("/missing/main.rk");

        let diagnostic = Diagnostic::for_file("missing file".to_string(), path.clone());

        assert_eq!(diagnostic.location, DiagnosticLocation::File(path));
        assert!(diagnostic.labels.is_empty());
    }

    #[test]
    #[should_panic(expected = "source diagnostics require a source path")]
    fn source_diagnostic_rejects_an_empty_span() {
        let _ = Diagnostic::new(
            "invalid source diagnostic".to_string(),
            Span {
                file_path: PathBuf::new(),
                start: 0,
                end: 0,
            },
        );
    }

    #[test]
    fn diagnostic_source_map_attaches_virtual_source_by_span_path() {
        let path = PathBuf::from("/virtual/main.rk");
        let text = "main = -> missing\n";
        let mut database = crate::source_loader::SourceDatabase::new();
        database.add_source_provider(&SourceProvider::Virtual {
            path: path.clone(),
            text: text.to_string(),
        });
        database
            .load_entry(path.clone(), &Config::default())
            .expect("virtual source should load");
        let sources = DiagnosticSourceMap::from_source_database(&database);
        let mut diagnostic = Diagnostic::new(
            "unknown value".to_string(),
            Span {
                file_path: path.clone(),
                start: 10,
                end: 17,
            },
        );

        sources.attach(&mut diagnostic);

        let source = diagnostic.source.expect("source should be attached");
        assert_eq!(source.display_path, path);
        assert_eq!(source.text, text);
        assert_eq!(source.origin, DiagnosticSourceOrigin::Virtual);
    }

    #[test]
    fn diagnostic_source_map_attaches_sources_for_cross_file_labels() {
        let primary_path = PathBuf::from("/virtual/main.rk");
        let related_path = PathBuf::from("/virtual/dep.rk");
        let mut database = crate::source_loader::SourceDatabase::new();
        for (path, text) in [
            (primary_path.clone(), "main = -> dep\n"),
            (related_path.clone(), "dep = -> 0\n"),
        ] {
            database.add_source_provider(&SourceProvider::Virtual {
                path: path.clone(),
                text: text.to_string(),
            });
            database
                .load_entry(path, &Config::default())
                .expect("virtual source should load");
        }
        let sources = DiagnosticSourceMap::from_source_database(&database);
        let mut diagnostic = Diagnostic::new(
            "cross-file error".to_string(),
            Span {
                file_path: primary_path,
                start: 10,
                end: 13,
            },
        )
        .with_label(
            "defined here".to_string(),
            Span {
                file_path: related_path.clone(),
                start: 0,
                end: 3,
            },
        );

        sources.attach(&mut diagnostic);

        let report_result = std::panic::catch_unwind(|| diagnostic.report());
        let source = diagnostic.source.expect("source should be attached");
        let related = source
            .related
            .get(&related_path)
            .expect("related source should be attached");
        assert_eq!(related.display_path, related_path);
        assert_eq!(related.text, "dep = -> 0\n");
        assert!(report_result.is_ok());
    }

    #[test]
    fn diagnostic_source_map_preserves_artifact_source_origin() {
        let path = PathBuf::from("/artifact/dep/lib.rk");
        let artifact_path = PathBuf::from("/artifact/dep.rkca");
        let text = "answer = -> 42\n";
        let mut database = crate::source_loader::SourceDatabase::new();
        database.add_source_provider(&SourceProvider::Artifact {
            path: path.clone(),
            artifact_path: artifact_path.clone(),
            text: text.to_string(),
        });
        database
            .load_entry(path.clone(), &Config::default())
            .expect("artifact source should load");
        let sources = DiagnosticSourceMap::from_source_database(&database);
        let mut diagnostic = Diagnostic::new(
            "artifact source error".to_string(),
            Span {
                file_path: path,
                start: 0,
                end: 6,
            },
        );

        sources.attach(&mut diagnostic);

        let source = diagnostic.source.expect("source should be attached");
        assert_eq!(source.text, text);
        assert_eq!(
            source.origin,
            DiagnosticSourceOrigin::Artifact { artifact_path }
        );
    }

    #[test]
    fn unexpected_eof_diagnostic_points_to_actual_source_end() {
        let path = PathBuf::from("/virtual/main.rk");
        let text = "lang sized\n";
        let error = crate::parser::parse_source(path.clone(), text, &Config::default())
            .expect_err("incomplete language item should fail");
        let diagnostic = Diagnostic::from(error);

        let DiagnosticLocation::Source(span) = diagnostic.location else {
            panic!("EOF diagnostic should have a source location");
        };
        assert_eq!(span.file_path, path);
        assert_eq!(span.start, text.len());
        assert_eq!(span.end, text.len());
        assert!(!diagnostic.labels.is_empty());
    }
}

/* // Old
impl From<crate::parser::ParseError> for Diagnostics {
    fn from(err: crate::parser::ParseError) -> Self {
        let mut diagnostics = Diagnostics::default();

        diagnostics.push(Diagnostic::from(err));

        diagnostics
    }
}

impl From<crate::parser::ParseError> for Diagnostic {
    fn from(err: crate::parser::ParseError) -> Self {
        match err {
            crate::parser::ParseError::UnexpectedToken(got, expected) => Diagnostic {
                message: format!("Unexpected token: {:?}", got),
                labels: vec![(format!("Expected {:?}", expected), got.span.clone())],
                span: got.span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::UnexpectedKeyword(token, expected) => Diagnostic {
                message: format!("Unexpected keyword: {:?}", token.token_type,),
                labels: vec![(
                    format!("Expected one of {:?}", expected),
                    token.span.clone(),
                )],
                span: token.span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::Lexer(LexerError::UnknownToken(c, span)) => Diagnostic {
                message: format!("Lexer: Unknown token: {:?}", c),
                labels: vec![],
                span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::MacroNoCorrespondance {
                macro_name,
                invoc_name,
                invoc_arg,
            } => {
                let mut labels = vec![
                    (format!("For this macro"), macro_name.clone()),
                    (format!("In this macro invocation"), invoc_name.clone()),
                ];

                if let Some(invoc_arg) = invoc_arg {
                    labels.push((format!("With this token"), invoc_arg.clone()));
                }

                Diagnostic {
                    message: format!("Macro: Nothing expected this token"),
                    labels,
                    span: macro_name.clone(),
                    kind: DiagnosticType::Error,
                }
            }
            crate::parser::ParseError::InvalidPrecedence(precedence, token) => Diagnostic {
                message: format!("Invalid precedence: {:?}", precedence),
                labels: vec![(
                    format!("Precedence must be between 0 and 9"),
                    token.span.clone(),
                )],
                span: token.span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::IndentMismatch(got, expected, span) => Diagnostic {
                message: format!("Indent mismatch: got {}, expected {}", got, expected),
                labels: vec![
                    (format!("Expected indent level: {}", expected), span.clone()),
                    (format!("Got indent level: {}", got), span.clone()),
                ],
                span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::UnexpectedEof(_token) => Diagnostic {
                message: format!("Unexpected end of file"),
                labels: vec![],
                span: Span::test(),
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::LeftoverTokens(tokens) => Diagnostic {
                message: format!("Leftover tokens"),
                labels: vec![(format!("Expected end of file"), tokens[0].span.clone())],
                span: tokens[0].span.clone(),
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::UnknownFile(file) => Diagnostic {
                message: format!("Unknown file: {:?}", file),
                labels: vec![],
                span: Span::test(),
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::InternalError(message) => Diagnostic {
                message: format!("Internal error: {:?}", message),
                labels: vec![],
                span: Span::test(),
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::ShortCircuit => Diagnostic {
                message: format!("Short circuit, should never be printed !"),
                labels: vec![],
                span: Span::test(),
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::ExpectedType(span) => Diagnostic {
                message: format!("Expected a type"),
                labels: vec![(format!("But got"), span.clone())],
                span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::InvalidLHS(span) => Diagnostic {
                message: format!("Malformed assignment"),
                labels: vec![(format!("Invalid left-hand side"), span.clone())],
                span,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::InvalidVariant(enum_name, variant) => Diagnostic {
                message: format!("Invalid variant"),
                labels: vec![
                    (format!("In this enum"), enum_name.clone()),
                    (format!("Expected a variant"), variant.clone()),
                ],
                span: enum_name,
                kind: DiagnosticType::Error,
            },
            crate::parser::ParseError::InvalidType(span) => Diagnostic {
                message: format!("Invalid type"),
                labels: vec![(format!("Expected a type"), span.clone())],
                span,
                kind: DiagnosticType::Error,
            },
        }
    }
} */
