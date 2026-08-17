use std::collections::HashMap;

use crate::ast::{MacroDecl, MacroFragment};
use crate::lexer::{Token, TokenType};
use crate::macro_expansion::{ExpansionId, GeneratedSourceId, TokenStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMacro {
    pub name: String,
    pub arms: Vec<DeclarativeMacroArm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMacroArm {
    pub matcher: MacroMatcher,
    pub template: MacroTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroMatcher {
    pub fragments: Vec<MatcherFragment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroTemplate {
    pub fragments: Vec<TemplateFragment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Ident,
    Expr,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatcherFragment {
    Capture { name: String, kind: CaptureKind },
    Token(Token),
    Repetition(Vec<MatcherFragment>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateFragment {
    Capture { name: String },
    Token(Token),
    Repetition(Vec<TemplateFragment>),
}

#[derive(Debug, Clone, Default)]
pub struct CaptureSet {
    direct: HashMap<String, Vec<TokenStream>>,
    repeated: HashMap<String, Vec<TokenStream>>,
}

impl CaptureSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_direct(&mut self, name: String, stream: TokenStream) {
        let streams = self.direct.entry(name).or_default();
        if !streams.contains(&stream) {
            streams.push(stream);
        }
    }

    pub fn get_direct(&self, name: &str) -> Option<&[TokenStream]> {
        self.direct.get(name).map(Vec::as_slice)
    }

    pub fn insert_repeated(&mut self, name: String, streams: Vec<TokenStream>) {
        self.repeated.insert(name, streams);
    }

    pub fn get_repeated(&self, name: &str) -> Option<&[TokenStream]> {
        self.repeated.get(name).map(Vec::as_slice)
    }
}

impl DeclarativeMacro {
    pub fn from_ast(decl: &MacroDecl) -> Self {
        Self {
            name: decl.name.name.clone(),
            arms: decl
                .entries
                .iter()
                .map(|entry| DeclarativeMacroArm {
                    matcher: MacroMatcher {
                        fragments: compile_matcher_fragments(&entry.defs),
                    },
                    template: MacroTemplate {
                        fragments: compile_template_fragments(&entry.body),
                    },
                })
                .collect(),
        }
    }
}

impl MacroTemplate {
    #[cfg(test)]
    pub fn expand(&self, captures: &CaptureSet) -> Result<TokenStream, String> {
        expand_template_fragments(&self.fragments, captures, None)
    }

    pub fn expand_with_origins(
        &self,
        captures: &CaptureSet,
        expansion: ExpansionId,
        generated_source: GeneratedSourceId,
    ) -> Result<TokenStream, String> {
        expand_template_fragments_with_origins(
            &self.fragments,
            captures,
            None,
            expansion,
            generated_source,
        )
    }
}

#[cfg(test)]
fn expand_template_fragments(
    fragments: &[TemplateFragment],
    captures: &CaptureSet,
    repeated_index: Option<usize>,
) -> Result<TokenStream, String> {
    let mut trees = Vec::new();
    for fragment in fragments {
        match fragment {
            TemplateFragment::Capture { name } => {
                if let Some(index) = repeated_index {
                    if let Some(streams) = captures.get_repeated(name) {
                        let Some(stream) = streams.get(index) else {
                            return Err(format!("missing repeated macro capture '{}'", name));
                        };
                        trees.extend(stream.trees.clone());
                        continue;
                    }
                }

                let Some(streams) = captures.get_direct(name) else {
                    return Err(format!("missing macro capture '{}'", name));
                };
                for stream in streams {
                    trees.extend(stream.trees.clone());
                }
            }
            TemplateFragment::Token(token) => {
                trees.extend(TokenStream::from_tokens(vec![token.clone()]).trees);
            }
            TemplateFragment::Repetition(inner) => {
                let capture_names = capture_names_in_template(inner);
                let mut len = None;

                for name in &capture_names {
                    if let Some(streams) = captures.get_repeated(name) {
                        match len {
                            Some(len) if streams.len() != len => {
                                return Err(format!(
                                    "mismatched repeated macro capture lengths for '{}'",
                                    name
                                ));
                            }
                            None => len = Some(streams.len()),
                            _ => {}
                        }
                    };
                }

                let Some(len) = len else { continue };

                for index in 0..len {
                    let expanded = expand_template_fragments(inner, captures, Some(index))?;
                    let last_token_type = expanded
                        .to_tokens()
                        .last()
                        .map(|token| token.token_type.clone());
                    trees.extend(expanded.trees);
                    if index + 1 < len {
                        match last_token_type {
                            Some(TokenType::Indent(_)) => {}
                            Some(TokenType::Eol) => trees.extend(
                                TokenStream::from_tokens(vec![Token::from(TokenType::Indent(0))])
                                    .trees,
                            ),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    Ok(TokenStream { trees })
}

fn expand_template_fragments_with_origins(
    fragments: &[TemplateFragment],
    captures: &CaptureSet,
    repeated_index: Option<usize>,
    expansion: ExpansionId,
    generated_source: GeneratedSourceId,
) -> Result<TokenStream, String> {
    let mut trees = Vec::new();
    for fragment in fragments {
        match fragment {
            TemplateFragment::Capture { name } => {
                if let Some(index) = repeated_index {
                    if let Some(streams) = captures.get_repeated(name) {
                        let Some(stream) = streams.get(index) else {
                            return Err(format!("missing repeated macro capture '{}'", name));
                        };
                        trees.extend(stream.trees.clone());
                        continue;
                    }
                }

                let Some(streams) = captures.get_direct(name) else {
                    return Err(format!("missing macro capture '{}'", name));
                };
                for stream in streams {
                    trees.extend(stream.trees.clone());
                }
            }
            TemplateFragment::Token(token) => {
                trees.extend(
                    TokenStream::generated_from_tokens(
                        vec![token.clone()],
                        expansion,
                        generated_source,
                    )
                    .trees,
                );
            }
            TemplateFragment::Repetition(inner) => {
                let capture_names = capture_names_in_template(inner);
                let mut len = None;

                for name in &capture_names {
                    if let Some(streams) = captures.get_repeated(name) {
                        match len {
                            Some(len) if streams.len() != len => {
                                return Err(format!(
                                    "mismatched repeated macro capture lengths for '{}'",
                                    name
                                ));
                            }
                            None => len = Some(streams.len()),
                            _ => {}
                        }
                    };
                }

                let Some(len) = len else { continue };

                for index in 0..len {
                    let expanded = expand_template_fragments_with_origins(
                        inner,
                        captures,
                        Some(index),
                        expansion,
                        generated_source,
                    )?;
                    let last_token = expanded.to_tokens().last().cloned();
                    trees.extend(expanded.trees);
                    if index + 1 < len {
                        match last_token {
                            Some(Token {
                                token_type: TokenType::Indent(_),
                                ..
                            }) => {}
                            Some(Token {
                                token_type: TokenType::Eol,
                                span,
                            }) => trees.extend(
                                TokenStream::generated_from_tokens(
                                    vec![Token {
                                        token_type: TokenType::Indent(0),
                                        span,
                                    }],
                                    expansion,
                                    generated_source,
                                )
                                .trees,
                            ),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    Ok(TokenStream { trees })
}

fn capture_names_in_template(fragments: &[TemplateFragment]) -> Vec<String> {
    let mut names = Vec::new();
    for fragment in fragments {
        match fragment {
            TemplateFragment::Capture { name } => {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            TemplateFragment::Token(_) => {}
            TemplateFragment::Repetition(inner) => {
                for name in capture_names_in_template(inner) {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }
    }
    names
}

fn compile_matcher_fragments(fragments: &[MacroFragment]) -> Vec<MatcherFragment> {
    fragments
        .iter()
        .map(|fragment| match fragment {
            MacroFragment::Ident(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Ident,
            },
            MacroFragment::Expr(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Expr,
            },
            MacroFragment::Type(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Type,
            },
            MacroFragment::Token(token) => MatcherFragment::Token(token.clone()),
            MacroFragment::Repetition(inner) => MatcherFragment::Repetition(
                compile_matcher_fragments(&trim_repetition_close(inner)),
            ),
        })
        .collect()
}

fn compile_template_fragments(fragments: &[MacroFragment]) -> Vec<TemplateFragment> {
    fragments
        .iter()
        .map(|fragment| match fragment {
            MacroFragment::Ident(name) | MacroFragment::Expr(name) | MacroFragment::Type(name) => {
                TemplateFragment::Capture {
                    name: name.name.clone(),
                }
            }
            MacroFragment::Token(token) => TemplateFragment::Token(token.clone()),
            MacroFragment::Repetition(inner) => TemplateFragment::Repetition(
                compile_template_fragments(&trim_repetition_close(inner)),
            ),
        })
        .collect()
}

fn trim_repetition_close(fragments: &[MacroFragment]) -> Vec<MacroFragment> {
    let mut fragments = fragments.to_vec();
    let close_index = fragments.iter().rposition(|fragment| {
        !matches!(
            fragment,
            MacroFragment::Token(Token {
                token_type: TokenType::Eol | TokenType::Indent(_),
                ..
            })
        )
    });
    if matches!(
        close_index.and_then(|index| fragments.get(index)),
        Some(MacroFragment::Token(Token {
            token_type: TokenType::CloseParen,
            ..
        }))
    ) {
        fragments.remove(close_index.unwrap());
    }
    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Ident, MacroDecl, MacroEntry, MacroFragment};
    use crate::lexer::{Span, Token, TokenType};
    use crate::macro_expansion::{ExpansionId, GeneratedSourceId, TokenOrigin, TokenStream};

    fn ident(name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: Span::test(),
        }
    }

    fn span(start: usize, end: usize) -> Span {
        Span {
            file_path: "declarative_test.rk".into(),
            start,
            end,
        }
    }

    #[test]
    fn declarative_macro_compiles_capture_kinds_and_template_tokens() {
        let decl = MacroDecl {
            name: ident("make"),
            entries: vec![MacroEntry {
                defs: vec![MacroFragment::Ident(ident("name"))],
                body: vec![
                    MacroFragment::Ident(ident("name")),
                    MacroFragment::Token(Token::from(TokenType::Equal)),
                ],
            }],
        };

        let compiled = DeclarativeMacro::from_ast(&decl);

        assert_eq!(compiled.name, "make");
        assert_eq!(compiled.arms.len(), 1);
        assert!(matches!(
            compiled.arms[0].matcher.fragments[0],
            MatcherFragment::Capture {
                kind: CaptureKind::Ident,
                ..
            }
        ));
        assert!(matches!(
            compiled.arms[0].template.fragments[0],
            TemplateFragment::Capture { .. }
        ));
    }

    #[test]
    fn declarative_template_expands_direct_capture_tokens() {
        let mut captures = CaptureSet::new();
        captures.insert_direct(
            "name".to_string(),
            TokenStream::from_tokens(vec![Token::from(TokenType::Ident("main".to_string()))]),
        );
        let template = MacroTemplate {
            fragments: vec![
                TemplateFragment::Capture {
                    name: "name".to_string(),
                },
                TemplateFragment::Token(Token::from(TokenType::Equal)),
            ],
        };

        let expanded = template.expand(&captures).unwrap();

        assert!(matches!(
            expanded.to_tokens()[0].token_type,
            TokenType::Ident(_)
        ));
        assert!(matches!(
            expanded.to_tokens()[1].token_type,
            TokenType::Equal
        ));
    }

    #[test]
    fn declarative_template_marks_captured_and_generated_origins() {
        let expansion = ExpansionId(9);
        let generated_source = GeneratedSourceId(4);
        let invocation_span = span(1, 7);
        let capture_span = span(8, 12);
        let template_span = span(20, 21);
        let mut captures = CaptureSet::new();
        captures.insert_direct(
            "name".to_string(),
            TokenStream::captured_from_tokens(
                vec![Token {
                    token_type: TokenType::Ident("main".to_string()),
                    span: capture_span.clone(),
                }],
                expansion,
                invocation_span.clone(),
            ),
        );
        let template = MacroTemplate {
            fragments: vec![
                TemplateFragment::Capture {
                    name: "name".to_string(),
                },
                TemplateFragment::Token(Token {
                    token_type: TokenType::Equal,
                    span: template_span.clone(),
                }),
            ],
        };

        let expanded = template
            .expand_with_origins(&captures, expansion, generated_source)
            .unwrap();
        let tokens = expanded.to_tokens_with_origins();

        match &tokens[0].1 {
            TokenOrigin::Captured {
                expansion: id,
                invocation_span: span,
                capture_span: captured,
            } => {
                assert_eq!(*id, expansion);
                assert_eq!(
                    (span.start, span.end),
                    (invocation_span.start, invocation_span.end)
                );
                assert_eq!(
                    (captured.start, captured.end),
                    (capture_span.start, capture_span.end)
                );
            }
            origin => panic!("expected captured origin, got {origin:?}"),
        }
        match &tokens[1].1 {
            TokenOrigin::Generated {
                expansion: id,
                generated_source: source,
                definition_span: span,
            } => {
                assert_eq!(*id, expansion);
                assert_eq!(*source, generated_source);
                assert_eq!(
                    (span.start, span.end),
                    (template_span.start, template_span.end)
                );
            }
            origin => panic!("expected generated origin, got {origin:?}"),
        }
    }

    #[test]
    fn declarative_template_expands_repeated_capture_groups() {
        let mut captures = CaptureSet::new();
        captures.insert_repeated(
            "name".to_string(),
            vec![
                TokenStream::from_tokens(vec![Token::from(TokenType::Ident("one".to_string()))]),
                TokenStream::from_tokens(vec![Token::from(TokenType::Ident("two".to_string()))]),
            ],
        );
        let template = MacroTemplate {
            fragments: vec![TemplateFragment::Repetition(vec![
                TemplateFragment::Capture {
                    name: "name".to_string(),
                },
                TemplateFragment::Token(Token::from(TokenType::Equal)),
            ])],
        };

        let expanded = template.expand(&captures).unwrap().to_tokens();

        assert_eq!(expanded.len(), 4);
        assert!(matches!(
            expanded[0].token_type,
            TokenType::Ident(ref name) if name == "one"
        ));
        assert!(matches!(expanded[1].token_type, TokenType::Equal));
        assert!(matches!(
            expanded[2].token_type,
            TokenType::Ident(ref name) if name == "two"
        ));
        assert!(matches!(expanded[3].token_type, TokenType::Equal));
    }

    #[test]
    fn declarative_template_repeats_direct_capture_with_repeated_capture_groups() {
        let mut captures = CaptureSet::new();
        captures.insert_direct(
            "ret".to_string(),
            TokenStream::from_tokens(vec![Token::from(TokenType::Ident("value".to_string()))]),
        );
        captures.insert_repeated(
            "name".to_string(),
            vec![
                TokenStream::from_tokens(vec![Token::from(TokenType::Ident("one".to_string()))]),
                TokenStream::from_tokens(vec![Token::from(TokenType::Ident("two".to_string()))]),
            ],
        );
        let template = MacroTemplate {
            fragments: vec![TemplateFragment::Repetition(vec![
                TemplateFragment::Capture {
                    name: "name".to_string(),
                },
                TemplateFragment::Token(Token::from(TokenType::Equal)),
                TemplateFragment::Token(Token::from(TokenType::Arrow)),
                TemplateFragment::Capture {
                    name: "ret".to_string(),
                },
            ])],
        };

        let expanded = template.expand(&captures).unwrap().to_tokens();

        assert_eq!(expanded.len(), 8);
        assert!(matches!(
            expanded[0].token_type,
            TokenType::Ident(ref name) if name == "one"
        ));
        assert!(matches!(expanded[1].token_type, TokenType::Equal));
        assert!(matches!(expanded[2].token_type, TokenType::Arrow));
        assert!(matches!(
            expanded[3].token_type,
            TokenType::Ident(ref name) if name == "value"
        ));
        assert!(matches!(
            expanded[4].token_type,
            TokenType::Ident(ref name) if name == "two"
        ));
        assert!(matches!(expanded[5].token_type, TokenType::Equal));
        assert!(matches!(expanded[6].token_type, TokenType::Arrow));
        assert!(matches!(
            expanded[7].token_type,
            TokenType::Ident(ref name) if name == "value"
        ));
    }
}
