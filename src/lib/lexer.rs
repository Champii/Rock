use super::{Token, TokenType};

pub struct Lexer {
    pub input: Vec<char>,
    cur_idx: usize,
    last_char: char,
    cur_line: usize,
    end: bool,
}

impl Lexer {
    pub fn new(input: Vec<char>) -> Lexer {
        let mut input = input.clone();

        input.push('\0');

        Lexer {
            last_char: input[0],
            input,
            cur_idx: 0,
            cur_line: 1,
            end: false,
        }
    }

    pub fn set_line(&mut self) {
        let lines: Vec<_> = self.input[..self.cur_idx].split(|c| *c == '\n').collect();

        self.cur_line = lines.len();
    }

    pub fn seek(&mut self, n: u32) -> Token {
        let mut n = n;

        let saved = self.cur_idx;
        let lines = self.cur_line;
        let last_char = self.last_char;

        let mut tok = self.next();

        n -= 1;

        while n > 0 {
            tok = self.next();

            n -= 1
        }

        self.cur_idx = saved;
        self.cur_line = lines;
        self.last_char = last_char;

        tok
    }

    pub fn restore(&mut self, token: Token) {
        self.cur_idx = token.end + 1;

        if self.cur_idx >= self.input.len() {
            self.end = true;

            self.cur_idx = self.input.len() - 1;

            self.last_char = '\0';
        } else {
            self.end = false;

            self.last_char = self.input[self.cur_idx];
        }

        self.set_line();
    }

    pub fn next(&mut self) -> Token {
        if self.end {
            return Token {
                t: TokenType::EOF,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "".to_string(),
            };
        }

        if let Some(t) = self.try_indent() {
            return t;
        }

        while self.last_char == ' ' {
            self.forward();
        }

        self.discard_comment();

        if let Some(t) = self.try_fn_keyword() {
            return t;
        }

        if let Some(t) = self.try_extern_keyword() {
            return t;
        }

        if let Some(t) = self.try_if_keyword() {
            return t;
        }

        if let Some(t) = self.try_else_keyword() {
            return t;
        }

        if let Some(t) = self.try_then_keyword() {
            return t;
        }

        if let Some(t) = self.try_for_keyword() {
            return t;
        }

        if let Some(t) = self.try_class_keyword() {
            return t;
        }

        if let Some(t) = self.try_parens() {
            return t;
        }

        if let Some(t) = self.try_braces() {
            return t;
        }

        if let Some(t) = self.try_array() {
            return t;
        }

        if let Some(t) = self.try_type() {
            return t;
        }

        if let Some(t) = self.try_array_decl() {
            return t;
        }

        if let Some(t) = self.try_bool() {
            return t;
        }

        if let Some(t) = self.try_ident() {
            return t;
        }

        if let Some(t) = self.try_arrow() {
            return t;
        }

        if let Some(t) = self.try_digit() {
            return t;
        }

        if let Some(t) = self.try_coma() {
            return t;
        }

        if let Some(t) = self.try_dot() {
            return t;
        }

        if let Some(t) = self.try_double_semi_colon() {
            return t;
        }

        if let Some(t) = self.try_semi_colon() {
            return t;
        }

        if let Some(t) = self.try_operator() {
            return t;
        }

        if let Some(t) = self.try_equal() {
            return t;
        }

        if let Some(t) = self.try_this() {
            return t;
        }

        if let Some(t) = self.try_string() {
            return t;
        }

        if let Some(t) = self.try_end_of() {
            return t;
        }

        if self.cur_idx >= self.input.len() - 1 {
            return Token {
                t: TokenType::EOF,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "".to_string(),
            };
        }

        panic!("Unknown token: '{}'", self.last_char);
    }

    fn discard_comment(&mut self) {
        if self.last_char == '#' {
            while self.last_char != '\n' && self.last_char != '\0' {
                self.forward();
            }
        }
    }

    fn try_arrow(&mut self) -> Option<Token> {
        if self.last_char == '-' && self.input[self.cur_idx + 1] == '>' {
            let start = self.cur_idx;

            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::Arrow,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "->".to_string(),
            });
        }

        None
    }

    fn try_fn_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'f'
            && self.input[self.cur_idx + 1] == 'n'
            && self.input[self.cur_idx + 2] == ' '
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::FnKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "fn".to_string(),
            });
        }

        None
    }

    fn try_extern_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'e' {
            let word: String = self.input[self.cur_idx..self.cur_idx + 6].iter().collect();

            if word == "extern".to_string() && self.input[self.cur_idx + 6] == ' ' {
                let start = self.cur_idx;

                self.forward();
                self.forward();
                self.forward();
                self.forward();
                self.forward();
                self.forward();

                return Some(Token {
                    t: TokenType::ExternKeyword,
                    line: self.cur_line,
                    start,
                    end: self.cur_idx - 1,
                    txt: "extern".to_string(),
                });
            }
        }

        None
    }

    fn try_if_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'i'
            && self.input[self.cur_idx + 1] == 'f'
            && self.input[self.cur_idx + 2] == ' '
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::IfKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "if".to_string(),
            });
        }

        None
    }

    fn try_else_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'e'
            && self.input[self.cur_idx + 1] == 'l'
            && self.input[self.cur_idx + 2] == 's'
            && self.input[self.cur_idx + 3] == 'e'
            && (self.input[self.cur_idx + 4] == ' ' || self.input[self.cur_idx + 4] == '\n')
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::ElseKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "else".to_string(),
            });
        }

        None
    }

    fn try_for_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'f'
            && self.input[self.cur_idx + 1] == 'o'
            && self.input[self.cur_idx + 2] == 'r'
            && (self.input[self.cur_idx + 3] == ' ' || self.input[self.cur_idx + 3] == '\n')
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::ForKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "for".to_string(),
            });
        }

        None
    }

    fn try_class_keyword(&mut self) -> Option<Token> {
        if self.last_char == 'c'
            && self.input[self.cur_idx + 1] == 'l'
            && self.input[self.cur_idx + 2] == 'a'
            && self.input[self.cur_idx + 3] == 's'
            && self.input[self.cur_idx + 4] == 's'
            && (self.input[self.cur_idx + 5] == ' ')
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::ClassKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "class".to_string(),
            });
        }

        None
    }

    fn try_then_keyword(&mut self) -> Option<Token> {
        if self.last_char == 't'
            && self.input[self.cur_idx + 1] == 'h'
            && self.input[self.cur_idx + 2] == 'e'
            && self.input[self.cur_idx + 3] == 'n'
            && self.input[self.cur_idx + 4] == ' '
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::ThenKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "then".to_string(),
            });
        }

        if self.last_char == '=' && self.input[self.cur_idx + 1] == '>' {
            let start = self.cur_idx;

            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::ThenKeyword,
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: "then".to_string(),
            });
        }

        None
    }

    fn try_bool(&mut self) -> Option<Token> {
        if self.last_char == 't'
            && self.input[self.cur_idx + 1] == 'r'
            && self.input[self.cur_idx + 2] == 'u'
            && self.input[self.cur_idx + 3] == 'e'
            && !self.input[self.cur_idx + 4].is_alphanumeric()
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::Bool(true),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: "true".to_string(),
            });
        }

        if self.last_char == 'f'
            && self.input[self.cur_idx + 1] == 'a'
            && self.input[self.cur_idx + 2] == 'l'
            && self.input[self.cur_idx + 3] == 's'
            && self.input[self.cur_idx + 4] == 'e'
            && !self.input[self.cur_idx + 5].is_alphanumeric()
        {
            let start = self.cur_idx;

            self.forward();
            self.forward();
            self.forward();
            self.forward();
            self.forward();

            return Some(Token {
                t: TokenType::Bool(false),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: "false".to_string(),
            });
        }

        None
    }
    
    fn try_ident(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        if self.last_char.is_alphabetic() || self.last_char == '_' {
            let mut identifier = vec![];

            while self.last_char.is_alphanumeric() || self.last_char == '_' {
                identifier.push(self.last_char);

                self.forward();
            }

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Identifier(identifier.iter().collect()),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: identifier.iter().collect(),
            });
        }

        None
    }

    fn try_parens(&mut self) -> Option<Token> {
        if self.last_char == '(' {
            let res = Some(Token {
                t: TokenType::OpenParens,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "(".to_string(),
            });

            self.forward();

            return res;
        } else if self.last_char == ')' {
            let res = Some(Token {
                t: TokenType::CloseParens,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: ")".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_braces(&mut self) -> Option<Token> {
        if self.last_char == '{' {
            let res = Some(Token {
                t: TokenType::OpenBrace,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "{".to_string(),
            });

            self.forward();

            return res;
        } else if self.last_char == '}' {
            let res = Some(Token {
                t: TokenType::CloseBrace,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "}".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_array(&mut self) -> Option<Token> {
        if self.last_char == '[' {
            let res = Some(Token {
                t: TokenType::OpenArray,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "[".to_string(),
            });

            self.forward();

            return res;
        } else if self.last_char == ']' {
            let res = Some(Token {
                t: TokenType::CloseArray,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "]".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_array_decl(&mut self) -> Option<Token> {
        if self.last_char == '[' && self.input[self.cur_idx + 1] == ']' {
            let res = Some(Token {
                t: TokenType::ArrayType,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: "[]".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        None
    }

    fn try_coma(&mut self) -> Option<Token> {
        if self.last_char == ',' {
            let res = Some(Token {
                t: TokenType::Coma,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: ",".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_dot(&mut self) -> Option<Token> {
        if self.last_char == '.' {
            let res = Some(Token {
                t: TokenType::Dot,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: ".".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_double_semi_colon(&mut self) -> Option<Token> {
        if self.last_char == ':' && self.input[self.cur_idx + 1] == ':' {
            let res = Some(Token {
                t: TokenType::DoubleSemiColon,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: "::".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        None
    }

    fn try_semi_colon(&mut self) -> Option<Token> {
        if self.last_char == ':' {
            let res = Some(Token {
                t: TokenType::SemiColon,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: ":".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    // fn try_equal_equal(&mut self) -> Option<Token> {
    //     if self.last_char == '=' && self.input[self.cur_idx + 1] == '=' {
    //         let res = Some(Token {
    //             t: TokenType::EqualEqual,
    //             line: self.cur_line,
    //             start: self.cur_idx,
    //             end: self.cur_idx + 1,
    //             txt: "==".to_string(),
    //         });

    //         self.forward();
    //         self.forward();

    //         return res;
    //     }

    //     None
    // }

    // fn try_dash_equal(&mut self) -> Option<Token> {
    //     if self.last_char == '!' && self.input[self.cur_idx + 1] == '=' {
    //         let res = Some(Token {
    //             t: TokenType::DashEqual,
    //             line: self.cur_line,
    //             start: self.cur_idx,
    //             end: self.cur_idx + 1,
    //             txt: "!=".to_string(),
    //         });

    //         self.forward();
    //         self.forward();

    //         return res;
    //     }

    //     None
    // }

    fn try_equal(&mut self) -> Option<Token> {
        if self.last_char == '=' {
            let res = Some(Token {
                t: TokenType::Equal,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "=".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_this(&mut self) -> Option<Token> {
        if self.last_char == '@' {
            let res = Some(Token {
                t: TokenType::Identifier("this".to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "this".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_operator(&mut self) -> Option<Token> {
        if self.last_char == '+' {
            let res = Some(Token {
                t: TokenType::Operator(self.last_char.to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "+".to_string(),
            });

            self.forward();

            return res;
        }

        if self.last_char == '=' && self.input[self.cur_idx + 1] == '=' {
            let res = Some(Token {
                t: TokenType::Operator("==".to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: "==".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        if self.last_char == '!' && self.input[self.cur_idx + 1] == '=' {
            let res = Some(Token {
                t: TokenType::Operator("!=".to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: "!=".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        if self.last_char == '<' && self.input[self.cur_idx + 1] == '=' {
            let res = Some(Token {
                t: TokenType::Operator("<=".to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: "<=".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        if self.last_char == '<' {
            let res = Some(Token {
                t: TokenType::Operator(self.last_char.to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "<".to_string(),
            });

            self.forward();

            return res;
        }

        if self.last_char == '>' && self.input[self.cur_idx + 1] == '=' {
            let res = Some(Token {
                t: TokenType::Operator(">=".to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx + 1,
                txt: ">=".to_string(),
            });

            self.forward();
            self.forward();

            return res;
        }

        if self.last_char == '>' {
            let res = Some(Token {
                t: TokenType::Operator(self.last_char.to_string()),
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: ">".to_string(),
            });

            self.forward();

            return res;
        }

        None
    }

    fn try_digit(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        if self.last_char.is_digit(10) {
            let mut number = vec![];

            while self.last_char.is_digit(10) {
                number.push(self.last_char);

                self.forward();
            }

            let nb_str: String = number.iter().collect();
            let nb: u64 = nb_str.parse().unwrap();

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Number(nb),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: nb_str,
            });
        }

        None
    }

    fn try_end_of(&mut self) -> Option<Token> {
        if self.last_char == '\n' {
            let res = Some(Token {
                t: TokenType::EOL,
                line: self.cur_line,
                start: self.cur_idx,
                end: self.cur_idx,
                txt: "\n".to_string(),
            });

            self.forward();

            self.cur_line += 1;

            return res;
        }

        None
    }

    fn try_indent(&mut self) -> Option<Token> {
        let save = self.cur_idx;

        if self.cur_idx > 0 && self.input[self.cur_idx - 1] == '\n' {
            let mut indent = 0;

            while self.input[self.cur_idx] == ' ' {
                let mut count = 0;
                while self.input[self.cur_idx] == ' ' && count < 4 {
                    self.forward();

                    count += 1;
                }
                if count == 4 {
                    indent += 1;
                }
            }

            if indent > 0 {
                return Some(Token {
                    t: TokenType::Indent(indent),
                    line: self.cur_line,
                    start: save,
                    end: self.cur_idx,
                    txt: " ".to_string(),
                });
            }

            self.cur_idx = save;
        }

        None
    }

    fn try_string(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        if self.last_char == '"' {
            let mut s = vec![];

            self.forward();

            while self.last_char != '"' {
                s.push(self.last_char);

                self.forward();
            }

            self.forward();

            let res: String = s.iter().collect();

            return Some(Token {
                t: TokenType::String(res.clone()),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: res,
            });
        }

        None
    }

    fn try_type(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        if self.last_char.is_alphabetic() && self.last_char.is_uppercase() {
            let mut identifier = vec![];

            while self.last_char.is_alphanumeric() {
                identifier.push(self.last_char);

                self.forward();
            }

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Type(identifier.iter().collect()),
                line: self.cur_line,
                start: start,
                end: self.cur_idx - 1,
                txt: identifier.iter().collect(),
            });
        }

        None
    }

    fn forward(&mut self) {
        self.cur_idx += 1;

        if self.cur_idx >= self.input.len() {
            self.end = true;

            self.cur_idx = self.input.len() - 1;

            self.last_char = '\0';
        } else {
            self.last_char = self.input[self.cur_idx];
        }
    }
}
