use crate::lexer::{Token, TokenType};
use crate::parser::items::tests::common::*;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_macro_entry() {
    let input = "$a:ident =>\n        statement";
    let tokens = lex(input);
    // let tokens = &tokens[1..]; // skip the Indent(0)
    let config = Config::default();

    let mut parse_ctx = ParseCtx::from(&tokens, &config);

    // Hack to force the indent step
    parse_ctx.indent_step = 4;

    let (rest, macro_entry) = macro_entry(parse_ctx).unwrap();

    assert_eq!(macro_entry.defs.len(), 1);
    assert_eq!(rest.len(), 0);
}

#[test]
fn macro_entry_reports_odd_body_indent() {
    let tokens = vec![
        Token::from(TokenType::Indent(4)),
        Token::from(TokenType::MacroVar("name".to_string())),
        Token::from(TokenType::Colon),
        Token::from(TokenType::Ident("ident".to_string())),
        Token::from(TokenType::FatArrow),
        Token::from(TokenType::Eol),
        Token::from(TokenType::Indent(5)),
        Token::from(TokenType::Ident("statement".to_string())),
        Token::from(TokenType::Eof),
    ];
    let config = Config::default();
    let mut parse_ctx = ParseCtx::from(&tokens, &config);
    parse_ctx.indent_level = 4;
    parse_ctx.invalid_indent = None;

    let result = macro_entry(parse_ctx);

    assert!(matches!(result, Err(ParseError::UnexpectedIndent(5))));
}
