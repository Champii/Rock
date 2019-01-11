#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    // keywords
    FnKeyword,
    ExternKeyword,
    IfKeyword,
    ThenKeyword,
    ElseKeyword,
    ForKeyword,

    // punct
    Arrow,
    Coma,
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

    //Operator
    Operator(String),

    // primitives
    Identifier(String),
    Number(u64),
    String(String),
    Bool(bool),
    Type(String),

    // indent
    Indent(u8),

    // whitespaces
    EOL,
    EOF,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub t: TokenType,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub txt: String,
}
