use crate::ast::Literal;
use crate::parser::{lex_test, literal, ParseCtx, Parser};
use crate::Config;

/// Helper function to parse a literal from input string
pub fn parse_literal(input: &str) -> Literal {
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, lit) = literal.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(rest.len(), 0);

    lit
}

/// Helper function to lex input (used by macro tests)
/// This version keeps the indent and EOF tokens
pub fn lex(input: &str) -> Vec<crate::lexer::Token> {
    use crate::lexer::Lexer;
    use std::path::PathBuf;

    Lexer::new(PathBuf::new(), input)
        .unwrap()
        .with_newline_at_end(false)
        .collect()
        .unwrap()
}
