use super::ast::*;
use super::context::*;
use super::error::Error;
use super::Lexer;
use super::{Token, TokenType};

macro_rules! expect {
    ($tok:expr, $self:expr) => {
        if $tok != $self.cur_tok.t {
            // panic!("Expected {:?} but found {:?}", $expr, $tok);
            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok.clone();

            $self.consume();

            cur_tok
        }
    };
}

macro_rules! expect_or_restore {
    ($tok:expr, $self:expr) => {
        if $self.cur_tok.t != $tok {
            $self.restore();

            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok.clone();

            $self.consume();

            cur_tok
        }
    };
}

macro_rules! error_expect {
    ($expected:expr, $self:expr) => {
        error!(
            format!("Expected {:?} but got {:?}", $expected, $self.cur_tok.t),
            $self
        );
    };
}

macro_rules! error {
    ($msg:expr, $self:expr) => {
        return Err(Error::new_parse_error(
            $self.lexer.input.clone(),
            $self.cur_tok.clone(),
            $msg,
        ));
    };
}

macro_rules! try_or_restore {
    ($expr:expr, $self:expr) => {
        match $expr {
            Ok(t) => t,
            Err(e) => {
                $self.restore();

                return Err(e);
            }
        }
    };
}

macro_rules! try_or_restore_expect {
    ($expr:expr, $expected:expr, $self:expr) => {
        try_or_restore_and!($expr, (error_expect!($expected, $self)), $self);
    };
}

macro_rules! try_or_restore_and {
    ($expr:expr, $and:expr, $self:expr) => {
        match $expr {
            Ok(t) => t,
            Err(e) => {
                $self.restore();

                return $and;
            }
        }
    };
}

// macro_rules! error_expect {
//     ($expected:expr, $got:expr) => {
//         return Err(format!("Expected {:?} but got {:?}", $expected, $got));
//     };
// }

pub struct Parser {
    lexer: Lexer,
    cur_tok: Token,
    save: Vec<Token>,
    ctx: Context,
    block_indent: u8,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Parser {
        let mut lexer = lexer;
        let cur_tok = lexer.next();

        Parser {
            save: vec![cur_tok.clone()],
            cur_tok,
            lexer,
            ctx: Context::new(),
            block_indent: 0,
        }
    }

    pub fn run(&mut self) -> Result<SourceFile, Error> {
        let mut top_levels = vec![];

        while self.cur_tok.t != TokenType::EOF {
            top_levels.push(self.top_level()?);
        }

        expect!(TokenType::EOF, self);

        Ok(SourceFile { top_levels })
    }

    fn consume(&mut self) {
        self.cur_tok = self.lexer.next();
    }

    fn save(&mut self) {
        self.save.push(self.cur_tok.clone());
    }

    fn save_pop(&mut self) {
        self.save.pop().unwrap();
    }

    fn restore(&mut self) {
        let save = self.save.pop().unwrap();

        self.lexer.restore(save.clone());
        self.cur_tok = save;
    }

    fn identifier(&mut self) -> Result<String, Error> {
        Ok(expect!(TokenType::Identifier(self.cur_tok.txt.clone()), self).txt)
    }

    fn type_(&mut self) -> Result<Type, Error> {
        if self.cur_tok.t == TokenType::ArrayType {
            self.save();

            self.consume();

            let t = try_or_restore!(self.type_(), self);

            self.save_pop();

            Ok(Type::Array(Box::new(t)))
        } else {
            Ok(Type::Name(
                expect!(TokenType::Type(self.cur_tok.txt.clone()), self).txt,
            ))
        }
    }

    fn top_level(&mut self) -> Result<TopLevel, Error> {
        let res = if self.cur_tok.t == TokenType::ExternKeyword {
            self.save();

            self.consume();

            let proto = try_or_restore!(self.prototype(), self);

            self.save_pop();

            Ok(TopLevel::Prototype(proto))
        } else {
            Ok(TopLevel::Function(self.function_decl()?))
        };

        while self.cur_tok.t == TokenType::EOL {
            self.consume();
        }

        res
    }

    fn prototype(&mut self) -> Result<Prototype, Error> {
        let mut name = None;
        let mut arguments = vec![];

        self.save();

        if let TokenType::Identifier(ident) = &self.cur_tok.t {
            name = Some(ident.clone());

            self.consume();
        }

        if TokenType::OpenParens == self.cur_tok.t
            || TokenType::Identifier(self.cur_tok.txt.clone()) == self.cur_tok.t
        {
            // manage error and restore here
            arguments = self.arguments_decl_type()?;
        }

        let ret = if self.cur_tok.t == TokenType::DoubleSemiColon {
            expect_or_restore!(TokenType::DoubleSemiColon, self);

            try_or_restore_expect!(
                self.type_(),
                TokenType::Type(self.cur_tok.txt.clone()),
                self
            )
        } else {
            Type::Name("Void".to_string())
        };

        return Ok(Prototype {
            name,
            ret,
            arguments,
        });
    }

    fn function_decl(&mut self) -> Result<FunctionDecl, Error> {
        let mut arguments = vec![];

        self.save();

        let name = try_or_restore!(self.identifier(), self);

        if TokenType::OpenParens == self.cur_tok.t
            || TokenType::Identifier(self.cur_tok.txt.clone()) == self.cur_tok.t
        {
            // manage error and restore here
            arguments = self.arguments_decl()?;
        }

        let ret = if self.cur_tok.t == TokenType::DoubleSemiColon {
            expect_or_restore!(TokenType::DoubleSemiColon, self);

            Some(try_or_restore_expect!(
                self.type_(),
                TokenType::Type(self.cur_tok.txt.clone()),
                self
            ))
        } else {
            None
        };

        expect_or_restore!(TokenType::Arrow, self);

        let body = try_or_restore!(self.body(), self);

        self.save_pop();

        Ok(FunctionDecl {
            name,
            ret,
            arguments,
            body,
        })
    }

    fn arguments_decl(&mut self) -> Result<Vec<ArgumentDecl>, Error> {
        let mut res = vec![];

        self.save();

        let mut has_parens = false;

        if TokenType::OpenParens == self.cur_tok.t {
            self.consume();

            has_parens = true;
        }

        if has_parens && TokenType::CloseParens == self.cur_tok.t {
            self.consume();

            self.save_pop();

            return Ok(res);
        }

        loop {
            let arg = try_or_restore!(self.argument_decl(), self);

            res.push(arg);

            if TokenType::Coma != self.cur_tok.t {
                break;
            }

            self.consume();
        }

        if has_parens {
            expect_or_restore!(TokenType::CloseParens, self);
        }

        self.save_pop();

        Ok(res)
    }

    fn arguments_decl_type(&mut self) -> Result<Vec<Type>, Error> {
        let mut res = vec![];

        self.save();

        self.consume();

        loop {
            let t = try_or_restore!(self.type_(), self);

            res.push(t);

            if TokenType::Coma != self.cur_tok.t {
                break;
            }

            self.consume();
        }

        self.consume();

        self.save_pop();

        Ok(res)
    }

    fn argument_decl(&mut self) -> Result<ArgumentDecl, Error> {
        let name = expect!(TokenType::Identifier(self.cur_tok.txt.clone()), self).txt;

        self.save();

        let t = if self.cur_tok.t == TokenType::SemiColon {
            expect_or_restore!(TokenType::SemiColon, self);

            Some(try_or_restore_expect!(
                self.type_(),
                TokenType::Type(self.cur_tok.txt.clone()),
                self
            ))
        } else {
            None
        };

        self.save_pop();

        Ok(ArgumentDecl { name, t })
    }

    fn arguments(&mut self) -> Result<Vec<Argument>, Error> {
        let mut res = vec![];

        self.save();

        let has_parens = if TokenType::OpenParens == self.cur_tok.t {
            expect!(TokenType::OpenParens, self);

            true
        } else {
            false
        };

        if has_parens && TokenType::CloseParens == self.cur_tok.t {
            self.consume();

            self.save_pop();

            return Ok(res);
        }

        loop {
            let arg = try_or_restore!(self.argument(), self);

            res.push(arg);

            if TokenType::Coma != self.cur_tok.t {
                break;
            }

            self.consume();
        }

        if has_parens {
            expect_or_restore!(TokenType::CloseParens, self);
        }

        self.save_pop();

        Ok(res)
    }

    fn argument(&mut self) -> Result<Argument, Error> {
        Ok(Argument {
            arg: self.expression()?,
        })
    }

    fn body(&mut self) -> Result<Body, Error> {
        let mut stmts = vec![];
        let mut is_multi = false;

        if self.cur_tok.t == TokenType::EOL {
            is_multi = true;

            self.block_indent += 1;

            self.consume();
        }

        if is_multi {
            if let TokenType::Indent(nb) = self.cur_tok.t {
                if nb != self.block_indent {
                    return Ok(Body { stmts });
                }

                self.consume();
            } else {
                return Ok(Body { stmts });
            }
        }

        loop {
            match self.statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => break,
            };

            if self.cur_tok.t != TokenType::EOF && is_multi {
                while self.cur_tok.t == TokenType::EOL {
                    self.consume();
                }
            }

            if is_multi {
                if let TokenType::Indent(nb) = self.cur_tok.t {
                    if nb != self.block_indent {
                        break;
                    }

                    self.consume();
                } else {
                    break;
                }
            }
        }

        if is_multi {
            self.block_indent -= 1;
        }

        Ok(Body { stmts })
    }

    fn statement(&mut self) -> Result<Statement, Error> {
        if let Ok(if_) = self.if_() {
            Ok(Statement::If(if_))
        } else if let Ok(assign) = self.assignation() {
            Ok(Statement::Assignation(assign))
        } else if let Ok(expr) = self.expression() {
            Ok(Statement::Expression(expr))
        } else {
            error!("Expected statement".to_string(), self);
        }
    }

    fn if_(&mut self) -> Result<If, Error> {
        expect!(TokenType::IfKeyword, self);

        self.save();

        let expr = try_or_restore!(self.expression(), self);

        if self.cur_tok.t == TokenType::ThenKeyword {
            self.consume();
        }

        let body = try_or_restore!(self.body(), self);

        // in case of single line body
        if self.cur_tok.t == TokenType::EOL {
            self.consume();
        }

        expect_or_restore!(TokenType::Indent(self.block_indent), self);

        if self.cur_tok.t != TokenType::ElseKeyword {
            self.save_pop();

            return Ok(If {
                predicat: expr,
                body,
                else_: None,
            });
        }

        self.consume();

        let else_ = try_or_restore!(self.else_(), self);

        self.save_pop();

        Ok(If {
            predicat: expr,
            body,
            else_: Some(Box::new(else_)),
        })
    }

    fn else_(&mut self) -> Result<Else, Error> {
        Ok(match self.cur_tok.t {
            TokenType::IfKeyword => Else::If(self.if_()?),
            _ => Else::Body(self.body()?),
        })
    }

    fn assignation(&mut self) -> Result<Assignation, Error> {
        self.save();

        let name = try_or_restore!(self.identifier(), self);

        let t = if self.cur_tok.t == TokenType::SemiColon {
            self.consume();
            // expect_or_restore!(TokenType::SemiColon, self);
            Some(try_or_restore_expect!(
                self.type_(),
                TokenType::Type(self.cur_tok.txt.clone()),
                self
            ))
        } else {
            None
        };

        expect_or_restore!(TokenType::Equal, self);

        let stmt = try_or_restore!(self.statement(), self);

        self.save_pop();

        Ok(Assignation {
            name,
            t,
            value: Box::new(stmt),
        })
    }

    fn expression(&mut self) -> Result<Expression, Error> {
        let left = self.unary_expr()?;

        self.save();

        let op = try_or_restore_and!(self.operator(), Ok(Expression::UnaryExpr(left)), self);

        let right = try_or_restore_and!(self.expression(), Ok(Expression::UnaryExpr(left)), self);

        self.save_pop();

        Ok(Expression::BinopExpr(left, op, Box::new(right)))
    }

    fn unary_expr(&mut self) -> Result<UnaryExpr, Error> {
        if self.cur_tok.t == TokenType::Operator(self.cur_tok.txt.clone()) {
            self.save();

            let op = try_or_restore!(self.operator(), self);

            let unary = try_or_restore!(self.unary_expr(), self);

            self.save_pop();

            return Ok(UnaryExpr::UnaryExpr(op, Box::new(unary)));
        }

        Ok(UnaryExpr::PrimaryExpr(self.primary_expr()?))
    }

    fn primary_expr(&mut self) -> Result<PrimaryExpr, Error> {
        let operand = self.operand()?;

        let mut secondarys = vec![];

        if self.cur_tok.t == TokenType::Operator(self.cur_tok.txt.clone()) {
            return Ok(PrimaryExpr::PrimaryExpr(operand, secondarys));
        }

        while let Ok(second) = self.secondary_expr() {
            secondarys.push(second);
        }

        Ok(PrimaryExpr::PrimaryExpr(operand, secondarys))
    }

    fn secondary_expr(&mut self) -> Result<SecondaryExpr, Error> {
        if let Ok(args) = self.arguments() {
            return Ok(SecondaryExpr::Arguments(args));
        }

        error!("Expected secondary".to_string(), self);
    }

    fn operand(&mut self) -> Result<Operand, Error> {
        if let Ok(lit) = self.literal() {
            return Ok(Operand::Literal(lit));
        }

        if let Ok(ident) = self.identifier() {
            return Ok(Operand::Identifier(ident));
        }

        if let Ok(expr) = self.parens_expr() {
            return Ok(Operand::Expression(Box::new(expr)));
        }

        error!("Expected operand".to_string(), self);
    }

    fn parens_expr(&mut self) -> Result<Expression, Error> {
        if self.cur_tok.t != TokenType::OpenParens {
            error!("No parens expr".to_string(), self);
        } else {
            self.save();

            expect_or_restore!(TokenType::OpenParens, self);

            let expr = try_or_restore!(self.expression(), self);

            expect_or_restore!(TokenType::CloseParens, self);

            self.save_pop();

            Ok(expr)
        }
    }

    fn operator(&mut self) -> Result<Operator, Error> {
        let op = match &self.cur_tok.t {
            TokenType::Operator(op) => op,
            _ => error!("Expected operator".to_string(), self),
        };

        let op = match op.as_ref() {
            "+" => Operator::Add,
            "==" => Operator::EqualEqual,
            _ => Operator::Add,
        };

        self.consume();

        Ok(op)
    }

    fn literal(&mut self) -> Result<Literal, Error> {
        if let TokenType::Number(num) = self.cur_tok.t {
            self.consume();

            return Ok(Literal::Number(num));
        }

        if let TokenType::Bool(b) = self.cur_tok.t {
            self.consume();

            let v = if b { 1 } else { 0 };

            return Ok(Literal::Bool(v));
        }

        if let TokenType::String(s) = self.cur_tok.t.clone() {
            self.consume();

            return Ok(Literal::String(s.clone()));
        }

        error!("Expected literal".to_string(), self);
    }
}
