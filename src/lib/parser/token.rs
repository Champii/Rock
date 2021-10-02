use super::Span;

pub fn accepted_operator_chars() -> Vec<char> {
    return vec!['+', '-', '/', '*', '|', '<', '>', '=', '!', '$', '@', '&'];
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    // keywords
    Fn,
    Let,
    Mod,
    Use,
    Extern,
    If,
    Then,
    Else,
    For,
    // In,
    Struct,
    Infix,
    Trait,
    Impl,

    // punct
    Arrow,
    Coma,
    Dot,
    SemiColon,
    DoubleSemiColon,
    Equal,
    ArrayType,
    // EqualEqual, // ==
    // DashEqual,  // !=
    OpenParens,
    CloseParens,
    OpenArray,
    CloseArray,
    OpenBrace,
    CloseBrace,

    //Operator
    Operator(String),
    NativeOperator(String),

    // primitives
    Identifier(String),
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Type(String),

    // indent
    Indent(u8),

    // whitespaces
    Eol,
    Eof,
}

impl Default for TokenType {
    fn default() -> TokenType {
        TokenType::Eof
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub t: TokenType,
    pub span: Span,
    pub txt: String,
}

pub type TokenId = usize;
