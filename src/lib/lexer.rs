use super::{Token, TokenType};

pub struct Lexer {
    pub input: Vec<char>,
    cur_idx: usize,
    last_char: char,
    cur_line: usize,
    end: bool,
}

bitflags! {
    struct Sep: u32 {
        const WS = 0b00000001;
        const EOL = 0b00000010;
        const NOTALPHANUM = 0b00000100;
    }
}

macro_rules! match_consume {
    ($str:literal, $t:expr, $end:expr, $self:ident) => {
        // TODO: optimisation: test the first letter first with last_char

        if $self
            .input
            .iter()
            .skip($self.cur_idx)
            .take($str.len())
            .collect::<String>()
            == $str.to_string()
        {
            if !$self.has_separator($str.len(), $end) {
                return None;
            }

            let start = $self.cur_idx;

            $self.forward($str.len());

            return Some(Token {
                t: $t,
                line: $self.cur_line,
                start,
                end: $self.cur_idx - 1,
                txt: $str.to_string(),
            });
        }

        return None
    };
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

    pub fn has_separator(&self, token_len: usize, sep: Sep) -> bool {
        if sep == Sep::empty() {
            return true;
        }

        let mut seps = vec![];

        if sep & Sep::WS == Sep::WS {
            seps.push(' ');
        }

        if sep & Sep::EOL == Sep::EOL {
            seps.push('\n');
        }

        if sep & Sep::NOTALPHANUM == Sep::NOTALPHANUM
            && !self.input[self.cur_idx + token_len].is_alphanumeric()
        {
            seps.push(self.input[self.cur_idx + token_len]);
        }

        seps.contains(&self.input[self.cur_idx + token_len])
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
            self.forward(1);
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
                self.forward(1);
            }
        }
    }

    fn try_arrow(&mut self) -> Option<Token> {
        match_consume!("->", TokenType::Arrow, Sep::empty(), self);
    }

    fn try_fn_keyword(&mut self) -> Option<Token> {
        match_consume!("fn", TokenType::FnKeyword, Sep::WS, self);
    }

    fn try_extern_keyword(&mut self) -> Option<Token> {
        match_consume!("extern", TokenType::ExternKeyword, Sep::WS, self);
    }

    fn try_if_keyword(&mut self) -> Option<Token> {
        match_consume!("if", TokenType::IfKeyword, Sep::WS, self);
    }

    fn try_else_keyword(&mut self) -> Option<Token> {
        match_consume!("else", TokenType::ElseKeyword, Sep::WS | Sep::EOL, self);
    }

    fn try_for_keyword(&mut self) -> Option<Token> {
        match_consume!("for", TokenType::ForKeyword, Sep::WS | Sep::EOL, self);
    }

    fn try_class_keyword(&mut self) -> Option<Token> {
        match_consume!("class", TokenType::ClassKeyword, Sep::WS, self);
    }

    fn try_then_keyword(&mut self) -> Option<Token> {
        if self.last_char == 't' {
            match_consume!("then", TokenType::ThenKeyword, Sep::WS, self);
        } else if self.last_char == '=' {
            match_consume!("=>", TokenType::ThenKeyword, Sep::empty(), self);
        } else {
            None
        }
    }

    fn try_bool(&mut self) -> Option<Token> {
        if self.last_char == 't' {
            match_consume!("true", TokenType::Bool(true), Sep::NOTALPHANUM, self);
        } else if self.last_char == 'f' {
            match_consume!("false", TokenType::Bool(false), Sep::NOTALPHANUM, self);
        } else {
            None
        }
    }

    fn try_ident(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        if self.last_char.is_alphabetic() || self.last_char == '_' {
            let mut identifier = vec![];

            while self.last_char.is_alphanumeric() || self.last_char == '_' {
                identifier.push(self.last_char);

                self.forward(1);
            }

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Identifier(identifier.iter().collect()),
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: identifier.iter().collect(),
            });
        }

        None
    }

    fn try_parens(&mut self) -> Option<Token> {
        if self.last_char == '(' {
            match_consume!("(", TokenType::OpenParens, Sep::empty(), self);
        } else if self.last_char == ')' {
            match_consume!(")", TokenType::CloseParens, Sep::empty(), self);
        } else {
            None
        }
    }

    fn try_braces(&mut self) -> Option<Token> {
        if self.last_char == '{' {
            match_consume!("{", TokenType::OpenBrace, Sep::empty(), self);
        } else if self.last_char == '}' {
            match_consume!("}", TokenType::CloseBrace, Sep::empty(), self);
        } else {
            None
        }
    }

    fn try_array(&mut self) -> Option<Token> {
        if self.last_char == '[' {
            match_consume!("[", TokenType::OpenArray, Sep::empty(), self);
        } else if self.last_char == ']' {
            match_consume!("]", TokenType::CloseArray, Sep::empty(), self);
        } else {
            None
        }
    }

    fn try_array_decl(&mut self) -> Option<Token> {
        match_consume!("[]", TokenType::ArrayType, Sep::empty(), self);
    }

    fn try_coma(&mut self) -> Option<Token> {
        match_consume!(",", TokenType::Coma, Sep::empty(), self);
    }

    fn try_dot(&mut self) -> Option<Token> {
        match_consume!(".", TokenType::Dot, Sep::empty(), self);
    }

    fn try_double_semi_colon(&mut self) -> Option<Token> {
        match_consume!("::", TokenType::DoubleSemiColon, Sep::empty(), self);
    }

    fn try_semi_colon(&mut self) -> Option<Token> {
        match_consume!(":", TokenType::SemiColon, Sep::empty(), self);
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
        match_consume!("=", TokenType::Equal, Sep::empty(), self);
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

            self.forward(1);

            return res;
        }

        None
    }

    fn try_operator(&mut self) -> Option<Token> {
        if self.last_char == '+' {
            match_consume!(
                "+",
                TokenType::Operator(self.last_char.to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '=' && self.input[self.cur_idx + 1] == '=' {
            match_consume!(
                "==",
                TokenType::Operator("==".to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '!' && self.input[self.cur_idx + 1] == '=' {
            match_consume!(
                "!=",
                TokenType::Operator("!=".to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '<' && self.input[self.cur_idx + 1] == '=' {
            match_consume!(
                "<=",
                TokenType::Operator("<=".to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '<' {
            match_consume!(
                "<",
                TokenType::Operator("<".to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '>' && self.input[self.cur_idx + 1] == '=' {
            match_consume!(
                ">=",
                TokenType::Operator(">=".to_string()),
                Sep::empty(),
                self
            );
        } else if self.last_char == '>' {
            match_consume!(
                ">",
                TokenType::Operator(">".to_string()),
                Sep::empty(),
                self
            );
        } else {
            None
        }
    }

    fn try_digit(&mut self) -> Option<Token> {
        let start = self.cur_idx;
        let mut is_neg = false;

        if self.last_char == '-' {
            is_neg = true;

            self.forward(1);
        }

        if self.last_char.is_digit(10) {
            let mut number = vec![];

            while self.last_char.is_digit(10) {
                number.push(self.last_char);

                self.forward(1);
            }

            let nb_str: String = number.iter().collect();
            let mut nb: i64 = nb_str.parse().unwrap();

            if is_neg {
                nb = -nb;
            }

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Number(nb),
                line: self.cur_line,
                start,
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

            self.forward(1);

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
                    self.forward(1);

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

            self.forward(1);

            while self.last_char != '"' {
                s.push(self.last_char);

                self.forward(1);
            }

            self.forward(1);

            let res: String = s.iter().collect();

            return Some(Token {
                t: TokenType::String(res.clone()),
                line: self.cur_line,
                start,
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

                self.forward(1);
            }

            // if is_keyword, return None

            return Some(Token {
                t: TokenType::Type(identifier.iter().collect()),
                line: self.cur_line,
                start,
                end: self.cur_idx - 1,
                txt: identifier.iter().collect(),
            });
        }

        None
    }

    fn forward(&mut self, n: usize) {
        self.cur_idx += n;

        if self.cur_idx >= self.input.len() {
            self.end = true;

            self.cur_idx = self.input.len() - 1;

            self.last_char = '\0';
        } else {
            self.last_char = self.input[self.cur_idx];
        }
    }
}
