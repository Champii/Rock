use crate::diagnostics::Diagnostic;

use super::{ParsingCtx, Token, TokenType};

bitflags! {
    struct Sep: u32 {
        const WS = 0b00000001;
        const EOL = 0b00000010;
        const NOTALPHANUM = 0b00000100;
    }
}

macro_rules! closure_vec {
    ($($m:path),*,) => {
        {
            let mut res = vec![];

            $(res.push(Box::new($m as for<'r> fn(&'r mut Lexer<'a>) -> Option<Token>));)*

            res
        }
    };
}

pub struct Lexer<'a> {
    ctx: &'a mut ParsingCtx,
    pub input: Vec<char>,
    cur_idx: usize,
    last_char: char,
    cur_line: usize,
    end: bool,
    base_indent: u8,
    accepted_operator_chars: Vec<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(ctx: &'a mut ParsingCtx) -> Lexer {
        let mut input: Vec<char> = ctx.get_current_file().content.chars().collect();

        input.push('\0');

        Lexer {
            ctx,
            last_char: input[0],
            input,
            cur_idx: 0,
            cur_line: 1,
            end: false,
            base_indent: 0,
            accepted_operator_chars: super::accepted_operator_chars(),
        }
    }

    pub fn next(&mut self) -> Token {
        if self.end {
            return self.new_token(TokenType::EOF, self.cur_idx, "".to_string());
        }

        if let Some(t) = self.try_indent() {
            return t;
        }

        while self.last_char == ' ' {
            self.forward(1);
        }

        self.discard_comment();

        let v = closure_vec![
            Self::try_fn_keyword,
            Self::try_let_keyword,
            Self::try_mod_keyword,
            Self::try_extern_keyword,
            Self::try_if_keyword,
            Self::try_else_keyword,
            Self::try_then_keyword,
            Self::try_for_keyword,
            Self::try_class_keyword,
            Self::try_infix_keyword,
            Self::try_use_keyword,
            Self::try_trait_keyword,
            Self::try_impl_keyword,
            Self::try_parens,
            Self::try_braces,
            Self::try_array,
            Self::try_type,
            Self::try_array_decl,
            Self::try_bool,
            Self::try_ident,
            Self::try_arrow,
            Self::try_operator_ident,
            Self::try_native_operator,
            // Self::try_primitive_operator,
            Self::try_digit,
            Self::try_coma,
            Self::try_dot,
            Self::try_double_semi_colon,
            Self::try_semi_colon,
            Self::try_equal,
            // Self::try_operator,
            Self::try_this,
            Self::try_string,
            Self::try_end_of,
        ];

        for method in v {
            if let Some(t) = method(self) {
                return t;
            }
        }

        if self.cur_idx >= self.input.len() - 1 {
            return self.new_token(TokenType::EOF, self.cur_idx, "".to_string());
        }

        self.ctx.diagnostics.push(Diagnostic::new_unexpected_token(
            self.ctx.new_span(self.cur_idx, self.cur_idx),
        ));

        self.end = true;

        self.new_token(TokenType::EOF, self.cur_idx, "".to_string())
    }

    fn has_separator(&self, token_len: usize, sep: Sep) -> bool {
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

    fn discard_comment(&mut self) {
        if self.last_char == '#' {
            while self.last_char != '\n' && self.last_char != '\0' {
                self.forward(1);
            }
        }
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

    fn new_token(&self, t: TokenType, start: usize, txt: String) -> Token {
        Token {
            t,
            span: self.ctx.new_span(start, self.cur_idx - 1),
            txt,
        }
    }

    fn match_consume(&mut self, s: &str, t: TokenType, end: Sep) -> Option<Token> {
        // TODO: optimisation: test the first letter first with last_char

        if self
            .input
            .iter()
            .skip(self.cur_idx)
            .take(s.len())
            .collect::<String>()
            == *s
        {
            if !self.has_separator(s.len(), end) {
                return None;
            }

            let start = self.cur_idx;

            self.forward(s.len());

            return Some(self.new_token(t, start, s.to_string()));
        }

        None
    }

    // Actual lexer methods

    fn try_arrow(&mut self) -> Option<Token> {
        self.match_consume("->", TokenType::Arrow, Sep::empty())
    }

    fn try_fn_keyword(&mut self) -> Option<Token> {
        self.match_consume("fn", TokenType::Fn, Sep::WS)
    }

    fn try_let_keyword(&mut self) -> Option<Token> {
        self.match_consume("let", TokenType::Let, Sep::WS)
    }

    fn try_mod_keyword(&mut self) -> Option<Token> {
        self.match_consume("mod", TokenType::Mod, Sep::WS)
    }

    fn try_extern_keyword(&mut self) -> Option<Token> {
        self.match_consume("extern", TokenType::Extern, Sep::WS)
    }

    fn try_if_keyword(&mut self) -> Option<Token> {
        self.match_consume("if", TokenType::If, Sep::WS)
    }

    fn try_else_keyword(&mut self) -> Option<Token> {
        self.match_consume("else", TokenType::Else, Sep::WS | Sep::EOL)
    }

    fn try_for_keyword(&mut self) -> Option<Token> {
        self.match_consume("for", TokenType::For, Sep::WS | Sep::EOL)
    }

    fn try_class_keyword(&mut self) -> Option<Token> {
        self.match_consume("class", TokenType::Class, Sep::WS)
    }

    fn try_then_keyword(&mut self) -> Option<Token> {
        if self.last_char == 't' {
            self.match_consume("then", TokenType::Then, Sep::WS)
        } else if self.last_char == '=' {
            self.match_consume("=>", TokenType::Then, Sep::empty())
        } else {
            None
        }
    }

    fn try_infix_keyword(&mut self) -> Option<Token> {
        self.match_consume("infix", TokenType::Infix, Sep::WS)
    }

    fn try_use_keyword(&mut self) -> Option<Token> {
        self.match_consume("use", TokenType::Use, Sep::WS)
    }

    fn try_trait_keyword(&mut self) -> Option<Token> {
        self.match_consume("trait", TokenType::Trait, Sep::WS)
    }

    fn try_impl_keyword(&mut self) -> Option<Token> {
        self.match_consume("impl", TokenType::Impl, Sep::WS)
    }

    fn try_bool(&mut self) -> Option<Token> {
        if self.last_char == 't' {
            self.match_consume("true", TokenType::Bool(true), Sep::NOTALPHANUM)
        } else if self.last_char == 'f' {
            self.match_consume("false", TokenType::Bool(false), Sep::NOTALPHANUM)
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

            return Some(self.new_token(
                TokenType::Identifier(identifier.iter().collect()),
                start,
                identifier.iter().collect(),
            ));
        }

        None
    }

    fn try_parens(&mut self) -> Option<Token> {
        if self.last_char == '(' {
            self.match_consume("(", TokenType::OpenParens, Sep::empty())
        } else if self.last_char == ')' {
            self.match_consume(")", TokenType::CloseParens, Sep::empty())
        } else {
            None
        }
    }

    fn try_braces(&mut self) -> Option<Token> {
        if self.last_char == '{' {
            self.match_consume("{", TokenType::OpenBrace, Sep::empty())
        } else if self.last_char == '}' {
            self.match_consume("}", TokenType::CloseBrace, Sep::empty())
        } else {
            None
        }
    }

    fn try_array(&mut self) -> Option<Token> {
        if self.last_char == '[' {
            self.match_consume("[", TokenType::OpenArray, Sep::empty())
        } else if self.last_char == ']' {
            self.match_consume("]", TokenType::CloseArray, Sep::empty())
        } else {
            None
        }
    }

    fn try_array_decl(&mut self) -> Option<Token> {
        self.match_consume("[]", TokenType::ArrayType, Sep::empty())
    }

    fn try_coma(&mut self) -> Option<Token> {
        self.match_consume(",", TokenType::Coma, Sep::empty())
    }

    fn try_dot(&mut self) -> Option<Token> {
        self.match_consume(".", TokenType::Dot, Sep::empty())
    }

    fn try_double_semi_colon(&mut self) -> Option<Token> {
        self.match_consume("::", TokenType::DoubleSemiColon, Sep::empty())
    }

    fn try_semi_colon(&mut self) -> Option<Token> {
        self.match_consume(":", TokenType::SemiColon, Sep::empty())
    }

    fn try_equal(&mut self) -> Option<Token> {
        self.match_consume("=", TokenType::Equal, Sep::empty())
    }

    fn try_this(&mut self) -> Option<Token> {
        if self.last_char == '@' {
            self.forward(1);

            let res = self.new_token(
                TokenType::Identifier("this".to_string()),
                self.cur_idx,
                "this".to_string(),
            );

            return Some(res);
        }

        None
    }

    // fn try_operator(&mut self) -> Option<Token> {
    //     if self.last_char == '+' {
    //         self.match_consume(
    //             "+",
    //             TokenType::Operator(self.last_char.to_string()),
    //             Sep::empty(),
    //         )
    //     } else if self.last_char == '-' {
    //         self.match_consume("-", TokenType::Operator("-".to_string()), Sep::empty())
    //     } else if self.last_char == '=' && self.input[self.cur_idx + 1] == '=' {
    //         self.match_consume("==", TokenType::Operator("==".to_string()), Sep::empty())
    //     } else if self.last_char == '!' && self.input[self.cur_idx + 1] == '=' {
    //         self.match_consume("!=", TokenType::Operator("!=".to_string()), Sep::empty())
    //     } else if self.last_char == '<' && self.input[self.cur_idx + 1] == '=' {
    //         self.match_consume("<=", TokenType::Operator("<=".to_string()), Sep::empty())
    //     } else if self.last_char == '<' {
    //         self.match_consume("<", TokenType::Operator("<".to_string()), Sep::empty())
    //     } else if self.last_char == '>' && self.input[self.cur_idx + 1] == '=' {
    //         self.match_consume(">=", TokenType::Operator(">=".to_string()), Sep::empty())
    //     } else if self.last_char == '>' {
    //         self.match_consume(">", TokenType::Operator(">".to_string()), Sep::empty())
    //     } else {
    //         None
    //     }
    // }

    fn try_digit(&mut self) -> Option<Token> {
        let start = self.cur_idx;
        let mut is_neg = false;

        if self.last_char == '-' {
            is_neg = true;

            self.forward(1);
        }

        let mut is_float = false;

        if self.last_char.is_digit(10) {
            let mut number = vec![];

            while self.last_char.is_digit(10) || self.last_char == '.' {
                if self.last_char == '.' {
                    is_float = true;
                }

                number.push(self.last_char);

                self.forward(1);
            }

            let mut nb_str: String = number.iter().collect();

            if is_neg {
                nb_str.insert(0, '-');
            }

            let nb = if is_float {
                TokenType::Float(nb_str.parse::<f64>().unwrap())
            } else {
                TokenType::Number(nb_str.parse::<i64>().unwrap())
            };

            // if is_keyword, return None

            return Some(self.new_token(nb, start, nb_str));
        }

        None
    }

    fn try_operator_ident(&mut self) -> Option<Token> {
        let start = self.cur_idx;

        let mut identifier = vec![];

        if self
            .accepted_operator_chars
            .iter()
            .find(|c| **c == self.last_char)
            .is_some()
        {
            while self
                .accepted_operator_chars
                .iter()
                .find(|c| **c == self.last_char)
                .is_some()
            {
                identifier.push(self.last_char);

                self.forward(1);
            }

            // if is_keyword, return None

            return Some(self.new_token(
                TokenType::Operator(identifier.iter().collect()),
                start,
                identifier.iter().collect(),
            ));
        }

        None
    }

    fn try_native_operator(&mut self) -> Option<Token> {
        if self.last_char == '~' {
            let mut res = None;
            let ops = vec![
                "~IAdd", "~ISub", "~IMul", "~IDiv", "~FAdd", "~FSub", "~FMul", "~FDiv", "~IEq",
                "~IGT", "~IGE", "~ILT", "~ILE", "~FEq", "~FGT", "~FGE", "~FLT", "~FLE", "~BEq",
            ];

            for op in ops {
                res = self.match_consume(op, TokenType::NativeOperator(op.to_string()), Sep::WS);

                if res.is_some() {
                    break;
                }
            }

            res
        } else {
            None
        }
    }

    fn try_end_of(&mut self) -> Option<Token> {
        if self.last_char == '\n' {
            let res = Some(Token {
                t: TokenType::EOL,
                span: self.ctx.new_span(self.cur_idx, self.cur_idx),
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

                while self.input[self.cur_idx] == ' '
                    && (count < self.base_indent || self.base_indent == 0)
                {
                    self.forward(1);

                    count += 1;
                }

                if self.base_indent == 0 {
                    self.base_indent = count;
                }

                if count == self.base_indent {
                    indent += 1;
                }
            }

            if indent > 0 {
                return Some(self.new_token(TokenType::Indent(indent), save, " ".to_string()));
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

            return Some(self.new_token(TokenType::String(res.clone()), start, res));
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

            return Some(self.new_token(
                TokenType::Type(identifier.iter().collect()),
                start,
                identifier.iter().collect(),
            ));
        }

        None
    }

    //

    pub fn collect(&mut self) -> Vec<Token> {
        let mut res = vec![];

        loop {
            let next = self.next();

            res.push(next.clone());

            if next.t == TokenType::EOF {
                break;
            }
        }

        res
    }
}
