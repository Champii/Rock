use super::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    // keywords
    FnKeyword,
    ExternKeyword,
    IfKeyword,
    ThenKeyword,
    ElseKeyword,
    ForKeyword,
    InKeyword,
    ClassKeyword,

    // punct
    Arrow,
    Coma,
    Dot,
    SemiColon,
    DoubleSemiColon,
    Equal,
    ArrayType,
    EqualEqual, // ==
    DashEqual,  // !=
    OpenParens,
    CloseParens,
    OpenArray,
    CloseArray,
    OpenBrace,
    CloseBrace,

    //Operator
    Operator(String),

    // primitives
    Identifier(String),
    Number(i64),
    String(String),
    Bool(bool),
    Type(String),

    // indent
    Indent(u8),

    // whitespaces
    EOL,
    EOF,
}

impl Default for TokenType {
    fn default() -> TokenType {
        TokenType::EOF
    }
}

#[derive(Clone, Debug, Default)]
pub struct Token {
    pub t: TokenType,
    pub span: Span,
    // pub line: usize,
    // pub start: usize,
    // pub end: usize,
    pub txt: String,
}

pub type TokenId = usize;
