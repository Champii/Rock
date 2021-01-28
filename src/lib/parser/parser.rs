use crate::token::TokenId;

use super::ast::*;
use super::error::Error;
use super::Token;

#[macro_export]
macro_rules! expect {
    ($tok:expr, $self:expr) => {
        if $tok != $self.cur_tok().t {
            // panic!("Expected {:?} but found {:?}", $expr, $tok);
            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok();

            $self.consume();

            cur_tok
        }
    };
}

#[macro_export]
macro_rules! expect_or_restore {
    ($tok:expr, $self:expr) => {
        if $self.cur_tok().t != $tok {
            $self.restore();

            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok();

            $self.consume();

            cur_tok
        }
    };
}

#[macro_export]
macro_rules! error_expect {
    ($expected:expr, $self:expr) => {
        crate::parser::macros::error!(
            format!("Expected {:?} but got {:?}", $expected, $self.cur_tok().t),
            $self
        );
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr, $self:expr) => {
        return Err(Error::new_parse_error(
            $self.input.clone(),
            $self.cur_tok(),
            $msg,
        ));
    };
}

#[macro_export]
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

#[macro_export]
macro_rules! try_or_restore_expect {
    ($expr:expr, $expected:expr, $self:expr) => {
        try_or_restore_and!($expr, error_expect!($expected, $self), $self);
    };
}

#[macro_export]
macro_rules! try_or_restore_and {
    ($expr:expr, $and:expr, $self:expr) => {
        match $expr {
            Ok(t) => t,
            Err(_e) => {
                $self.restore();

                #[allow(unreachable_code)]
                return $and;
            }
        }
    };
}

pub mod macros {
    pub use crate::error;
    pub use crate::error_expect;
    pub use crate::expect;
    pub use crate::expect_or_restore;
    pub use crate::try_or_restore;
    pub use crate::try_or_restore_and;
    pub use crate::try_or_restore_expect;
}

pub trait Parse {
    fn parse(ctx: &mut Parser) -> Result<Self, Error>
    where
        Self: Sized;
}

// TODO: Create getters and setters instead of exposing publicly
pub struct Parser {
    pub input: Vec<char>,
    pub tokens: Vec<Token>,
    pub cur_tok_id: TokenId,
    save: Vec<TokenId>,
    pub block_indent: u8,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, input: Vec<char>) -> Parser {
        Parser {
            input,
            tokens,
            save: vec![0],
            cur_tok_id: 0,
            block_indent: 0,
        }
    }

    pub fn run(&mut self) -> Result<Root, Error> {
        Root::parse(self)
    }

    pub fn cur_tok(&self) -> Token {
        match self.tokens.get(self.cur_tok_id as usize) {
            Some(a) => a.clone(),
            _ => unreachable!(),
        }
    }

    pub fn consume(&mut self) {
        self.cur_tok_id += 1;
    }

    pub fn save(&mut self) {
        self.save.push(self.cur_tok_id);
    }

    pub fn save_pop(&mut self) {
        self.save.pop().unwrap();
    }

    pub fn restore(&mut self) {
        let save = self.save.pop().unwrap();

        self.cur_tok_id = save;
    }

    pub fn seek(&self, distance: usize) -> Token {
        match self.tokens.get(self.cur_tok_id as usize + distance) {
            Some(a) => a.clone(),
            _ => unreachable!(),
        }
    }
}

impl Parse for Root {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        Ok(Root {
            identity: Identity::new(ctx.cur_tok_id),
            r#mod: Mod::parse(ctx)?,
        })
    }
}

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = match ctx.cur_tok().t {
            _ => TopLevelKind::Function(FunctionDecl::parse(ctx)?),
        };

        while ctx.cur_tok().t == TokenType::EOL {
            ctx.consume();
        }

        Ok(TopLevel {
            kind,
            identity: Identity::new(token),
        })
    }
}

impl Parse for FunctionDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut arguments = vec![];

        let token = ctx.cur_tok_id;

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        if TokenType::OpenParens == ctx.cur_tok().t
            || TokenType::Identifier(ctx.cur_tok().txt.clone()) == ctx.cur_tok().t
        {
            // manage error and restore here
            arguments = ArgumentsDecl::parse(ctx)?;
        }

        expect_or_restore!(TokenType::Equal, ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(FunctionDecl {
            name,
            arguments,
            body,
            identity: Identity::new(token),
        })
    }
}

impl Parse for Identifier {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(Self {
            name: token.txt,
            identity: Identity::new(token_id),
        })
    }
}

impl Parse for ArgumentsDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        loop {
            let arg = try_or_restore!(ArgumentDecl::parse(ctx), ctx);

            res.push(arg);

            match ctx.cur_tok().t {
                TokenType::Identifier(_) => {}
                _ => break,
            }
        }

        ctx.save_pop();

        Ok(res)
    }
}

impl Parse for ArgumentDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(ArgumentDecl {
            name: token.txt.clone(),
            identity: Identity::new(token_id),
        })
    }
}

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let stmt = Statement::parse(ctx)?;

        Ok(Body {
            identity: Identity::new(stmt.identity.token_id),
            stmt,
        })
    }
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = Box::new(if let Ok(if_) = If::parse(ctx) {
            StatementKind::If(if_)
        // } else if let Ok(for_) = For::parse(ctx) {
        //     StatementKind::For(for_)
        // } else if let Ok(assign) = Assignation::parse(ctx) {
        //     StatementKind::Assignation(assign)
        } else if let Ok(expr) = Expression::parse(ctx) {
            StatementKind::Expression(expr)
        } else {
            error!("Expected statement".to_string(), ctx);
        });

        Ok(Statement {
            kind,
            identity: Identity::new(token),
        })
    }
}

impl Parse for If {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::IfKeyword, ctx);

        ctx.save();

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        let mut is_multi = true;

        if ctx.cur_tok().t == TokenType::ThenKeyword {
            is_multi = false;

            ctx.consume();
        }

        let body = try_or_restore!(Body::parse(ctx), ctx);

        // in case of single line body
        if !is_multi || ctx.cur_tok().t == TokenType::EOL {
            expect!(TokenType::EOL, ctx);
        }

        let next = ctx.seek(1);

        if next.t != TokenType::ElseKeyword {
            ctx.save_pop();

            return Ok(If {
                predicat: expr,
                body,
                else_: None,
            });
        }

        expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);

        expect_or_restore!(TokenType::ElseKeyword, ctx);

        let else_ = try_or_restore!(Else::parse(ctx), ctx);

        ctx.save_pop();

        Ok(If {
            predicat: expr,
            body,
            else_: Some(Box::new(else_)),
        })
    }
}

impl Parse for Expression {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let left = UnaryExpr::parse(ctx)?;

        let mut res = Expression {
            kind: ExpressionKind::UnaryExpr(left.clone()),
            identity: Identity::new(token),
        };

        ctx.save();

        let op = try_or_restore_and!(Operator::parse(ctx), Ok(res), ctx);

        let right = try_or_restore_and!(Expression::parse(ctx), Ok(res), ctx);

        ctx.save_pop();

        res.kind = ExpressionKind::BinopExpr(left, op, Box::new(right));

        Ok(res)
    }
}

impl Parse for Else {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        Ok(match ctx.cur_tok().t {
            TokenType::IfKeyword => Else::If(If::parse(ctx)?),
            _ => Else::Body(Body::parse(ctx)?),
        })
    }
}

impl Parse for UnaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone()) {
            ctx.save();

            let op = try_or_restore!(Operator::parse(ctx), ctx);

            let unary = try_or_restore!(UnaryExpr::parse(ctx), ctx);

            ctx.save_pop();

            return Ok(UnaryExpr::UnaryExpr(op, Box::new(unary)));
        }

        Ok(UnaryExpr::PrimaryExpr(PrimaryExpr::parse(ctx)?))
    }
}

impl Parse for Operator {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let op = match ctx.cur_tok().t {
            TokenType::Operator(op) => op,
            _ => error!("Expected operator".to_string(), ctx),
        };

        let op = match op.as_ref() {
            "+" => Operator::Add,
            "-" => Operator::Sub,
            "==" => Operator::EqualEqual,
            "!=" => Operator::DashEqual,
            "<" => Operator::Less,
            "<=" => Operator::LessOrEqual,
            ">" => Operator::More,
            ">=" => Operator::MoreOrEqual,
            _ => Operator::Add,
        };

        ctx.consume();

        Ok(op)
    }
}

impl Parse for PrimaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let operand = Operand::parse(ctx)?;

        let mut secondarys = vec![];

        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone())
            || ctx.cur_tok().t == TokenType::Equal
        {
            return Ok(PrimaryExpr::PrimaryExpr(operand, secondarys));
        }

        while let Ok(second) = SecondaryExpr::parse(ctx) {
            secondarys.push(second);

            if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone())
                || ctx.cur_tok().t == TokenType::Equal
            {
                break;
            }
        }

        Ok(PrimaryExpr::PrimaryExpr(operand, secondarys))
    }
}

impl Parse for Operand {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut token = ctx.cur_tok_id;

        if ctx.cur_tok().t == TokenType::OpenParens {
            ctx.save();

            expect_or_restore!(TokenType::OpenParens, ctx);

            let expr = try_or_restore!(Expression::parse(ctx), ctx);

            expect_or_restore!(TokenType::CloseParens, ctx);

            ctx.save_pop();

            return Ok(expr);
        }

        let kind = if let Ok(lit) = Literal::parse(ctx) {
            OperandKind::Literal(lit)
        } else if let Ok(ident) = Identifier::parse(ctx) {
            OperandKind::Identifier(ident)
        // } else if let Ok(c) = ClassInstance::parse(ctx) {
        //     OperandKind::ClassInstance(c)
        // } else if let Ok(array) = Array::parse(ctx) {
        //     OperandKind::Array(array)
        } else if let Ok(expr) = Self::parens_expr(ctx) {
            token = expr.identity.token_id;

            OperandKind::Expression(Box::new(expr))
        } else {
            self::error!("Expected operand".to_string(), ctx);
        };

        return Ok(Operand {
            kind,
            identity: Identity::new(token),
        });
    }
}

impl Parse for SecondaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        // if let Ok(idx) = Self::index(ctx) {
        //     return Ok(SecondaryExpr::Index(idx));
        // }

        // if let Ok(sel) = Selector::parse(ctx) {
        //     return Ok(SecondaryExpr::Selector(sel));
        // }

        if let Ok(args) = Arguments::parse(ctx) {
            return Ok(SecondaryExpr::Arguments(args));
        }

        self::error!("Expected secondary".to_string(), ctx);
    }
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        Ok(Self {
            kind: LiteralKind::parse(ctx)?,
            identity: Identity::new(token_id),
        })
    }
}

impl Parse for LiteralKind {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::Number(num));
        }

        if let TokenType::Bool(b) = ctx.cur_tok().t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(LiteralKind::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok().t.clone() {
            ctx.consume();

            return Ok(LiteralKind::String(s.clone()));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

impl Parse for Arguments {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        loop {
            let arg = try_or_restore!(Argument::parse(ctx), ctx);

            res.push(arg);

            if TokenType::Coma != ctx.cur_tok().t {
                break;
            }

            ctx.consume();
        }

        ctx.save_pop();

        Ok(res)
    }
}

impl Parse for Argument {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        Ok(Argument {
            arg: Expression::parse(ctx)?,
            identity: Identity::new(token),
        })
    }
}
