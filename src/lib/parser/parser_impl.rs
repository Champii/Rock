use crate::ast::*;
use crate::{ast::resolve::ResolutionMap, diagnostics::Diagnostic};
// use crate::error::Error;
use crate::parser::*;

type Error = Diagnostic;

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

// macro_rules! try_or_restore_expect {
//     ($expr:expr, $expected:expr, $self:expr) => {
//         try_or_restore_and!($expr, error_expect!($expected, $self), $self);
//     };
// }

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
            resolutions: ResolutionMap::default(),
            // identity: Identity::new(ctx.cur_tok_id),
            r#mod: Mod::parse(ctx)?,
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
            TokenType::ModKeyword => {
                ctx.consume(); // mod keyword
                let name = Identifier::parse(ctx)?;

                let (mod_, _) = super::parse_mod(name.name.clone(), ctx.ctx)
                    .map_err(|diag| Diagnostic::new(token.span.clone(), diag.get_kind()))?;

                TopLevelKind::Mod(name, mod_)
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

        expect_or_restore!(TokenType::Equal, ctx);

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

        let token_res = expect!(TokenType::Identifier(ctx.cur_tok().txt), ctx);

        Ok(Self {
            name: token_res.txt,
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
        let stmt = Statement::parse(ctx)?;

        Ok(Body {
            // identity: Identity::new(stmt.identity.token_id),
            stmt,
        })
    }
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        // let token = ctx.cur_tok_id;

        let kind = Box::new(match If::parse(ctx) {
            Ok(if_) => StatementKind::If(if_),
            Err(_e) => match Expression::parse(ctx) {
                Ok(expr) => StatementKind::Expression(expr),
                Err(_e) => error!("Expected statement".to_string(), ctx),
            },
        });
        // } else if let Ok(for_) = For::parse(ctx) {
        //     StatementKind::For(for_)
        // } else if let Ok(assign) = Assignation::parse(ctx) {
        //     StatementKind::Assignation(assign)
        // } else if let Ok(expr) = Expression::parse(ctx) {
        //     StatementKind::Expression(expr)
        // } else {
        //     error!("Expected statement".to_string(), ctx);
        // });

        Ok(Statement {
            kind,
            // identity: Identity::new(token),
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
        // let token = ctx.cur_tok_id;

        let left = UnaryExpr::parse(ctx)?;

        let mut res = Expression {
            kind: ExpressionKind::UnaryExpr(left.clone()),
            // identity: Identity::new(token),
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

        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt)
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
        // let mut token = ctx.cur_tok_id;

        let kind = if let Ok(lit) = Literal::parse(ctx) {
            OperandKind::Literal(lit)
        } else if let Ok(ident) = IdentifierPath::parse(ctx) {
            OperandKind::Identifier(ident)
        // } else if let Ok(c) = ClassInstance::parse(ctx) {
        //     OperandKind::ClassInstance(c)
        // } else if let Ok(array) = Array::parse(ctx) {
        //     OperandKind::Array(array)
        } else if ctx.cur_tok().t == TokenType::OpenParens {
            ctx.save();

            expect_or_restore!(TokenType::OpenParens, ctx);

            let expr = try_or_restore!(Expression::parse(ctx), ctx);

            expect_or_restore!(TokenType::CloseParens, ctx);

            ctx.save_pop();

            // token = expr.identity.token_id;

            OperandKind::Expression(Box::new(expr))
        } else {
            error!("Expected operand".to_string(), ctx);
        };

        Ok(Operand {
            kind,
            // identity: Identity::new(token),
        })
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
        // let token = ctx.cur_tok_id;

        Ok(Argument {
            arg: Expression::parse(ctx)?,
            // identity: Identity::new(token),
        })
    }
}
