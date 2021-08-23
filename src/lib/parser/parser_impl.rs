use std::convert::TryInto;

use crate::ast::*;
use crate::{ast::resolve::ResolutionMap, diagnostics::Diagnostic};
// use crate::error::Error;
use crate::parser::*;

type Error = Diagnostic;

macro_rules! expect {
    ($tok:expr, $self:expr) => {
        if $tok != $self.cur_tok().t {
            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok();

            $self.consume();

            cur_tok
        }
    };
}

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

macro_rules! error_expect {
    ($expected:expr, $self:expr) => {
        trace!(
            "Error expect: want {:?} got {:?}",
            $expected,
            $self.cur_tok().t
        );

        // This is not the error! macro from env_logger, see below
        error!(
            format!("Expected {:?} but got {:?}", $expected, $self.cur_tok().t),
            $self
        );
    };
}

macro_rules! error {
    ($msg:expr, $self:expr) => {
        return Err(Diagnostic::new_syntax_error(
            $self.cur_tok().span.clone(),
            $msg,
        ))
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

pub trait Parse {
    fn parse(ctx: &mut Parser) -> Result<Self, Error>
    where
        Self: Sized;
}

// TODO: Create getters and setters instead of exposing publicly
pub struct Parser<'a> {
    ctx: &'a mut ParsingCtx,
    pub input: Vec<char>,
    pub tokens: Vec<Token>,
    pub cur_tok_id: TokenId,
    save: Vec<TokenId>,
    pub block_indent: u8,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, ctx: &'a mut ParsingCtx) -> Parser {
        let input: Vec<char> = ctx.get_current_file().content.chars().collect();

        Parser {
            ctx,
            input,
            tokens,
            save: vec![0],
            cur_tok_id: 0,
            block_indent: 0,
        }
    }

    pub fn run_root(&mut self) -> Result<Root, Error> {
        Root::parse(self)
    }

    pub fn run_mod(&mut self) -> Result<Mod, Error> {
        Mod::parse(self)
    }

    pub fn cur_tok(&self) -> Token {
        match self.tokens.get(self.cur_tok_id as usize) {
            Some(a) => a.clone(),
            _ => unreachable!(),
        }
    }

    pub fn consume(&mut self) {
        trace!("Consume token {:?}", self.cur_tok());

        self.cur_tok_id += 1;
    }

    pub fn save(&mut self) {
        trace!("Save()");

        self.save.push(self.cur_tok_id);
    }

    pub fn save_pop(&mut self) {
        trace!("Save_pop()");

        self.save.pop().unwrap();
    }

    pub fn restore(&mut self) {
        trace!("Restore()");

        let save = self.save.pop().unwrap();

        self.cur_tok_id = save;
    }

    pub fn seek(&self, distance: usize) -> Token {
        match self.tokens.get(self.cur_tok_id as usize + distance) {
            Some(a) => a.clone(),
            _ => Token {
                t: TokenType::EOF,
                span: Span::new_placeholder(),
                txt: "".to_string(),
            },
        }
    }
}

impl Parse for Root {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let r#mod = Mod::parse(ctx)?;

        Ok(Root {
            resolutions: ResolutionMap::default(),
            operators_list: ctx.ctx.operators_list.clone(),
            r#mod,
        })
    }
}

impl Parse for Mod {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        while TokenType::EOF != ctx.cur_tok().t {
            match TopLevel::parse(ctx) {
                Ok(top) => res.push(top),
                Err(e) => {
                    ctx.ctx.diagnostics.push(e.clone());

                    return Err(e);
                }
            };
        }

        expect!(TokenType::EOF, ctx);

        Ok(Mod {
            identity: Identity::new(0, ctx.ctx.new_span(0, 0)),
            top_levels: res,
        })
    }
}

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let kind = match ctx.cur_tok().t {
            TokenType::Extern => {
                ctx.consume(); // extern keyword

                TopLevelKind::Prototype(Prototype::parse(ctx)?)
            }
            TokenType::Mod => {
                ctx.consume(); // mod keyword

                let name = Identifier::parse(ctx)?;

                let mod_node = super::parse_mod(name.name.clone(), ctx.ctx)
                    .map_err(|diag| Diagnostic::new(token.span.clone(), diag.get_kind()))?;

                TopLevelKind::Mod(name, mod_node)
            }
            TokenType::Use => {
                ctx.consume(); // use keyword

                TopLevelKind::Use(Use::parse(ctx)?)
            }
            TokenType::Infix => {
                ctx.consume(); // infix keyword

                let identifier = Identifier::parse(ctx)?;

                let precedence = if let TokenType::Number(num) = ctx.cur_tok().t {
                    ctx.consume();

                    num
                } else {
                    panic!("Bad infix syntax");
                };

                ctx.ctx
                    .add_operator(&identifier, precedence.try_into().unwrap())?;

                TopLevelKind::Infix(identifier, precedence as u8)
            }
            _ => TopLevelKind::Function(FunctionDecl::parse(ctx)?),
        };

        while ctx.cur_tok().t == TokenType::EOL {
            ctx.consume();
        }

        Ok(TopLevel {
            kind,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Prototype {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut arguments = vec![];

        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        if TokenType::OpenParens == ctx.cur_tok().t
            || TokenType::Identifier(ctx.cur_tok().txt) == ctx.cur_tok().t
        {
            // manage error and restore here
            arguments = ArgumentsDecl::parse(ctx)?;
        }

        ctx.save_pop();

        Ok(Prototype {
            name,
            arguments,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Use {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        Ok(Use {
            path: IdentifierPath::parse(ctx)?,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for FunctionDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut arguments = vec![];

        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        if TokenType::OpenParens == ctx.cur_tok().t
            || TokenType::Identifier(ctx.cur_tok().txt) == ctx.cur_tok().t
        {
            // manage error and restore here
            arguments = ArgumentsDecl::parse(ctx)?;
        }

        expect_or_restore!(TokenType::Operator("=".to_string()), ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(FunctionDecl {
            name,
            arguments,
            body,
            identity: Identity::new(token_id, token.span),
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
        let token = ctx.cur_tok();

        expect!(TokenType::Identifier(ctx.cur_tok().txt), ctx);

        Ok(ArgumentDecl {
            name: token.txt.clone(),
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut multi = false;

        if ctx.cur_tok().t == TokenType::EOL {
            multi = true;

            ctx.block_indent += 1;

            ctx.consume();

            expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);
        }

        let stmt = Statement::parse(ctx)?;

        if multi {
            ctx.block_indent -= 1;
        }

        Ok(Body { stmt })
    }
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let kind = if ctx.cur_tok().t == TokenType::If {
            match If::parse(ctx) {
                Ok(expr) => StatementKind::If(expr),
                Err(_e) => error!("Expected if".to_string(), ctx),
            }
        } else {
            match Expression::parse(ctx) {
                Ok(expr) => StatementKind::Expression(expr),
                Err(_e) => error!("Expected expression".to_string(), ctx),
            }
        };

        Ok(Statement {
            kind: Box::new(kind),
        })
    }
}

impl Parse for If {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        ctx.save();

        expect_or_restore!(TokenType::If, ctx);

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        expect_or_restore!(TokenType::EOL, ctx);
        expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);
        expect_or_restore!(TokenType::Then, ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);
        // in case of single line body
        if ctx.cur_tok().t == TokenType::EOL {
            expect!(TokenType::EOL, ctx);
            // expect_or_restore!(TokenType::Indent(ctx.block_indent + 2), ctx);
        }

        let next = ctx.seek(1);

        if next.t != TokenType::Else {
            ctx.save_pop();

            return Ok(If {
                predicat: expr,
                body,
                else_: None,
                identity: Identity::new(token_id, token.span),
            });
        }

        expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);

        expect_or_restore!(TokenType::Else, ctx);

        let else_ = try_or_restore!(Else::parse(ctx), ctx);

        ctx.save_pop();

        Ok(If {
            predicat: expr,
            body,
            else_: Some(Box::new(else_)),
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Expression {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.cur_tok().t == TokenType::NativeOperator(ctx.cur_tok().txt) {
            ctx.save();

            let op = NativeOperator::parse(ctx)?;
            let left = Identifier::parse(ctx)?;
            let right = Identifier::parse(ctx)?;

            ctx.save_pop();

            return Ok(Expression {
                kind: ExpressionKind::NativeOperation(op, left, right),
            });
        }

        let left = UnaryExpr::parse(ctx)?;

        let mut res = Expression {
            kind: ExpressionKind::UnaryExpr(left.clone()),
        };

        // FIXME
        match ctx.cur_tok().t {
            TokenType::Operator(_) => (),
            _ => return Ok(res),
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
            TokenType::If => Else::If(If::parse(ctx)?),
            _ => Else::Body(Body::parse(ctx)?),
        })
    }
}

impl Parse for UnaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt) {
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
        Ok(Operator(Identifier::parse(ctx)?))
    }
}

impl Parse for PrimaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let operand = Operand::parse(ctx)?;

        let mut secondaries = vec![];

        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt)
            || ctx.cur_tok().t == TokenType::Equal
        {
            return Ok(PrimaryExpr {
                identity: Identity::new(token_id, token.span),
                op: operand,
                secondaries: None,
            });
        }

        while let Ok(second) = SecondaryExpr::parse(ctx) {
            secondaries.push(second);

            if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone())
                || ctx.cur_tok().t == TokenType::Equal
            {
                break;
            }
        }

        let secondaries = if secondaries.is_empty() {
            None
        } else {
            Some(secondaries)
        };

        Ok(PrimaryExpr {
            identity: Identity::new(token_id, token.span),
            op: operand,
            secondaries,
        })
    }
}

impl Parse for Operand {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let kind = if let Ok(lit) = Literal::parse(ctx) {
            OperandKind::Literal(lit)
        } else if let Ok(ident) = IdentifierPath::parse(ctx) {
            OperandKind::Identifier(ident)
        } else if ctx.cur_tok().t == TokenType::OpenParens {
            ctx.save();

            expect_or_restore!(TokenType::OpenParens, ctx);

            let expr = try_or_restore!(Expression::parse(ctx), ctx);

            expect_or_restore!(TokenType::CloseParens, ctx);

            ctx.save_pop();

            OperandKind::Expression(Box::new(expr))
        } else {
            error!("Expected operand".to_string(), ctx);
        };

        Ok(Operand { kind })
    }
}

impl Parse for SecondaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let Ok(args) = Arguments::parse(ctx) {
            return Ok(SecondaryExpr::Arguments(args));
        }

        error!("Expected secondary".to_string(), ctx);
    }
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        Ok(Self {
            kind: LiteralKind::parse(ctx)?,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for LiteralKind {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::Number(num));
        }

        if let TokenType::Float(float) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::Float(float));
        }

        if let TokenType::Bool(b) = ctx.cur_tok().t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(LiteralKind::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::String(s));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

impl Parse for IdentifierPath {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut path = vec![];

        ctx.save();

        loop {
            let tok = try_or_restore!(Identifier::parse(ctx), ctx);

            path.push(tok.clone());

            if TokenType::DoubleSemiColon != ctx.cur_tok().t {
                break;
            }

            expect!(TokenType::DoubleSemiColon, ctx);
        }

        ctx.save_pop();

        Ok(Self { path })
    }
}

impl Parse for Identifier {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let txt = match ctx.cur_tok().t {
            TokenType::Identifier(id) => id,
            TokenType::Operator(op) => op,
            _ => error!("Not an operator".to_string(), ctx),
        };

        ctx.consume();

        Ok(Self {
            name: txt,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Arguments {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        // TODO: factorise this with a match! macro ?
        if TokenType::OpenParens == ctx.cur_tok().t {
            if TokenType::CloseParens == ctx.seek(1).t {
                ctx.consume();
                ctx.consume();

                ctx.save_pop();

                return Ok(res);
            }
        }

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
        Ok(Argument {
            arg: UnaryExpr::parse(ctx)?,
        })
    }
}

impl Parse for NativeOperator {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let op = match ctx.cur_tok().t {
            TokenType::NativeOperator(op) => op,
            _ => error!("Expected native operator".to_string(), ctx),
        };

        let kind = match op.as_ref() {
            "~IAdd" => NativeOperatorKind::IAdd,
            "~ISub" => NativeOperatorKind::ISub,
            "~IMul" => NativeOperatorKind::IMul,
            "~IDiv" => NativeOperatorKind::IDiv,
            "~Eq" => NativeOperatorKind::Eq,
            "~GT" => NativeOperatorKind::GT,
            "~GE" => NativeOperatorKind::GE,
            "~LT" => NativeOperatorKind::LT,
            "~LE" => NativeOperatorKind::LE,
            "~FAdd" => NativeOperatorKind::FAdd,
            "~FSub" => NativeOperatorKind::FSub,
            "~FMul" => NativeOperatorKind::FMul,
            "~FDiv" => NativeOperatorKind::FDiv,
            _ => error!("Unknown native operator".to_string(), ctx),
        };

        ctx.consume();

        Ok(NativeOperator {
            kind,
            identity: Identity::new(token_id, token.span),
        })
    }
}
