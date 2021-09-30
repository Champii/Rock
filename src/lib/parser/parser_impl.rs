use std::{collections::HashMap, convert::TryInto};

use crate::ast::*;
use crate::parser::*;
use crate::{ast::resolve::ResolutionMap, diagnostics::Diagnostic};

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
    func_sigs: HashMap<String, FuncType>,
    pub struct_types: HashMap<String, StructDecl>,
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
            func_sigs: HashMap::new(),
            struct_types: HashMap::new(),
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

    pub fn add_func_sig(&mut self, name: String, sig: FuncType) {
        self.func_sigs.insert(name, sig);
    }

    pub fn add_struct_type(&mut self, s: &StructDecl) {
        self.struct_types.insert(s.name.get_name(), s.clone());
    }
}

impl Parse for Root {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let r#mod = Mod::parse(ctx)?;

        r#mod
            .top_levels
            .iter()
            .find(|top| match &top.kind {
                TopLevelKind::Function(f) => f.name.name == "main",
                _ => false,
            })
            .ok_or_else(Diagnostic::new_no_main)?;

        Ok(Root {
            spans: HashMap::new(),
            unused: vec![],
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
                    ctx.ctx.diagnostics.push_error(e.clone());

                    return Err(e);
                }
            };
        }

        expect!(TokenType::EOF, ctx);

        Ok(Mod {
            tokens: ctx.tokens.clone(),
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
            TokenType::Trait => {
                ctx.consume();

                TopLevelKind::Trait(Trait::parse(ctx)?)
            }
            TokenType::Impl => {
                ctx.consume();

                TopLevelKind::Impl(Impl::parse(ctx)?)
            }
            TokenType::Struct => {
                ctx.consume();

                TopLevelKind::Struct(StructDecl::parse(ctx)?)
            }
            TokenType::Mod => {
                ctx.consume(); // mod keyword

                let name = Identifier::parse(ctx)?;

                let mod_node = super::parse_mod(name.name.clone(), ctx.ctx)
                    .map_err(|diag| Diagnostic::new(name.identity.span.clone(), diag.get_kind()))?;

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
            _ => {
                // ctx.save();
                if let Ok(proto) = Prototype::parse(ctx) {
                    ctx.add_func_sig(proto.name.name.clone(), proto.signature.clone());

                    TopLevelKind::Prototype(proto)
                } else {
                    let mut f = FunctionDecl::parse(ctx)?;

                    let mut has_applied = false;

                    if let Some(sig) = ctx.func_sigs.get(&f.name.name) {
                        f.signature = sig.clone();

                        has_applied = true;
                    }
                    if has_applied {
                        ctx.func_sigs.remove(&f.name.name);
                    }

                    TopLevelKind::Function(f)
                }
            }
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

impl Parse for StructDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let mut defs = vec![];

        ctx.save();

        let name = try_or_restore!(Type::parse(ctx), ctx);

        // TODO: resolve type here ? else it is infered as Trait(name)
        //

        ctx.consume(); // EOL

        loop {
            if let TokenType::Indent(_) = ctx.cur_tok().t {
                ctx.consume(); // indent

                let f = Prototype::parse(ctx)?;

                defs.push(f);

                expect!(TokenType::EOL, ctx);
            } else {
                break;
            }
        }

        ctx.save_pop();

        let struct_decl = StructDecl {
            identity: Identity::new(token_id, token.span),
            name,
            defs,
        };

        ctx.add_struct_type(&struct_decl);

        Ok(struct_decl)
    }
}

impl Parse for Trait {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut types = vec![];
        let mut defs = vec![];

        ctx.save();

        let name = try_or_restore!(Type::parse(ctx), ctx);

        while ctx.cur_tok().t != TokenType::EOL {
            types.push(Type::parse(ctx)?);
        }

        ctx.consume(); // EOL

        loop {
            if let TokenType::Indent(_) = ctx.cur_tok().t {
                ctx.consume(); // indent

                let f = Prototype::parse(ctx)?;

                // f.mangle(name.get_name());

                defs.push(f);

                expect!(TokenType::EOL, ctx);
            } else {
                break;
            }
        }

        ctx.save_pop();

        Ok(Trait { name, types, defs })
    }
}

impl Parse for Impl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut types = vec![];
        let mut defs = vec![];

        ctx.save();

        let name = try_or_restore!(Type::parse(ctx), ctx);

        while ctx.cur_tok().t != TokenType::EOL {
            types.push(Type::parse(ctx)?);
        }

        ctx.consume(); // EOL

        ctx.block_indent += 1;
        loop {
            if let TokenType::Indent(_) = ctx.cur_tok().t {
                ctx.consume(); // indent
                let f = FunctionDecl::parse(ctx)?;

                // f.mangle(&types.iter().map(|t| t.get_name()).collect::<Vec<_>>());

                defs.push(f);

                expect!(TokenType::EOL, ctx);
            } else {
                break;
            }
        }
        ctx.block_indent -= 1;

        ctx.save_pop();

        Ok(Impl { name, types, defs })
    }
}

impl Parse for Prototype {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        expect_or_restore!(TokenType::DoubleSemiColon, ctx);

        let signature = try_or_restore!(FuncType::parse(ctx), ctx);

        ctx.save_pop();

        Ok(Prototype {
            name,
            signature,
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
            signature: FuncType::from_args_nb(arguments.len()),
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
        }

        let mut stmts = vec![];

        if multi {
            loop {
                ctx.save();

                if ctx.cur_tok().t == TokenType::EOL {
                    while ctx.cur_tok().t == TokenType::EOL {
                        ctx.consume();
                    }
                } else {
                    // Deactivated because of struct last EOF
                    // ctx.restore();
                    // break;
                }

                if ctx.cur_tok().t != TokenType::Indent(ctx.block_indent) {
                    ctx.restore();
                    break;
                } else {
                    ctx.save_pop();
                    ctx.consume();
                }

                ctx.save();
                let stmt = match Statement::parse(ctx) {
                    Ok(stmt) => stmt,
                    Err(_) => {
                        ctx.restore();
                        break;
                    }
                };

                ctx.save_pop();

                stmts.push(stmt);
            }
        } else {
            stmts.push(Statement::parse(ctx)?)
        }

        if multi {
            ctx.block_indent -= 1;
        }

        Ok(Body { stmts })
    }
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let kind = if ctx.cur_tok().t == TokenType::If {
            match If::parse(ctx) {
                Ok(expr) => StatementKind::If(expr),
                Err(_e) => error!("Expected if".to_string(), ctx),
            }
        } else if let Ok(assign) = Assign::parse(ctx) {
            StatementKind::Assign(assign)
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

impl Parse for AssignLeftSide {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.seek(1).t == TokenType::OpenArray {
            if let Ok(expr) = PrimaryExpr::parse(ctx) {
                // TODO:
                // if expr.is_indice() {

                return Ok(AssignLeftSide::Indice(Expression::from_unary(
                    &UnaryExpr::PrimaryExpr(expr),
                )));
                // }
            }
        }
        if ctx.seek(1).t == TokenType::Dot {
            if let Ok(expr) = PrimaryExpr::parse(ctx) {
                // TODO:
                // if expr.is_indice() {

                return Ok(AssignLeftSide::Dot(Expression::from_unary(
                    &UnaryExpr::PrimaryExpr(expr),
                )));
                // }
            }
        }

        if let Ok(id) = Identifier::parse(ctx) {
            return Ok(AssignLeftSide::Identifier(id));
        }

        error!(
            "Expected Identifier or Indice as assignation left side".to_string(),
            ctx
        )
    }
}

impl Parse for Assign {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let (is_let, name) = if TokenType::Let == ctx.cur_tok().t {
            expect!(TokenType::Let, ctx);

            let name = AssignLeftSide::Identifier(try_or_restore!(Identifier::parse(ctx), ctx));

            (true, name)
        } else {
            let name = try_or_restore!(AssignLeftSide::parse(ctx), ctx);

            (false, name)
        };

        expect_or_restore!(TokenType::Operator("=".to_string()), ctx);

        let value = try_or_restore!(Expression::parse(ctx), ctx);

        ctx.save_pop();

        Ok(Assign {
            name,
            value,
            is_let,
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

        if let TokenType::Type(_) = ctx.cur_tok().t {
            if let Ok(s) = StructCtor::parse(ctx) {
                return Ok(Expression {
                    kind: ExpressionKind::StructCtor(s),
                });
            } else {
                println!("NOT A CTOR");
            }
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

impl Parse for StructCtor {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;
        let token = ctx.cur_tok();

        let mut defs = HashMap::new();

        ctx.save();

        let name = try_or_restore!(Type::parse(ctx), ctx);

        ctx.consume(); // EOL

        let mut cur_indent = ctx.block_indent + 1;
        loop {
            if let TokenType::Indent(i) = ctx.cur_tok().t {
                if i != cur_indent {
                    break;
                }

                cur_indent = i;

                ctx.consume(); // indent

                let def_name = try_or_restore!(Identifier::parse(ctx), ctx);

                expect_or_restore!(TokenType::SemiColon, ctx);

                let expr = try_or_restore!(Expression::parse(ctx), ctx);

                defs.insert(def_name, expr);

                expect!(TokenType::EOL, ctx);
            } else {
                break;
            }
        }

        ctx.save_pop();

        Ok(StructCtor {
            identity: Identity::new(token_id, token.span),
            name,
            defs,
        })
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
        if TokenType::OpenArray == ctx.cur_tok().t {
            ctx.save();

            ctx.consume();

            if let Ok(expr) = Expression::parse(ctx) {
                expect_or_restore!(TokenType::CloseArray, ctx);

                ctx.save_pop();

                return Ok(SecondaryExpr::Indice(expr));
            }
        } else if TokenType::Dot == ctx.cur_tok().t {
            ctx.save();

            ctx.consume();

            if let Ok(expr) = Identifier::parse(ctx) {
                ctx.save_pop();

                return Ok(SecondaryExpr::Dot(expr));
            }
        } else if let Ok(args) = Arguments::parse(ctx) {
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

            let v = b;

            return Ok(LiteralKind::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::String(s));
        }

        if TokenType::OpenArray == ctx.cur_tok().t {
            return Ok(LiteralKind::Array(Array::parse(ctx)?));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

impl Parse for Array {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::OpenArray, ctx);

        let mut values = vec![];

        if TokenType::CloseArray == ctx.cur_tok().t {
            ctx.consume();
        } else {
            loop {
                values.push(Expression::parse(ctx)?);

                if TokenType::CloseArray == ctx.cur_tok().t {
                    ctx.consume();

                    break;
                } else {
                    expect!(TokenType::Coma, ctx);
                }
            }
        }
        Ok(Array { values })
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
        if TokenType::OpenParens == ctx.cur_tok().t && TokenType::CloseParens == ctx.seek(1).t {
            ctx.consume();
            ctx.consume();

            ctx.save_pop();

            return Ok(res);
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
            "~FAdd" => NativeOperatorKind::FAdd,
            "~FSub" => NativeOperatorKind::FSub,
            "~FMul" => NativeOperatorKind::FMul,
            "~FDiv" => NativeOperatorKind::FDiv,
            "~IEq" => NativeOperatorKind::IEq,
            "~Igt" => NativeOperatorKind::Igt,
            "~Ige" => NativeOperatorKind::Ige,
            "~Ilt" => NativeOperatorKind::Ilt,
            "~Ile" => NativeOperatorKind::Ile,
            "~FEq" => NativeOperatorKind::FEq,
            "~Fgt" => NativeOperatorKind::Fgt,
            "~Fge" => NativeOperatorKind::Fge,
            "~Flt" => NativeOperatorKind::Flt,
            "~Fle" => NativeOperatorKind::Fle,
            "~BEq" => NativeOperatorKind::BEq,
            _ => error!("Unknown native operator".to_string(), ctx),
        };

        ctx.consume();

        Ok(NativeOperator {
            kind,
            identity: Identity::new(token_id, token.span),
        })
    }
}

impl Parse for Type {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok();

        if let TokenType::Type(_t) = token.t {
            ctx.consume();

            if let Some(prim) = PrimitiveType::from_name(&token.txt) {
                Ok(Type::Primitive(prim))
            } else {
                if let Some(s) = ctx.struct_types.get(&token.txt) {
                    return Ok(s.to_type());
                }
                Ok(Type::Trait(token.txt))
            }
        } else if token.txt.len() == 1 && token.txt.chars().next().unwrap().is_lowercase() {
            ctx.consume();

            Ok(Type::ForAll(token.txt))
        } else if TokenType::OpenArray == token.t {
            ctx.consume();

            let inner_t = Type::parse(ctx)?;

            // FIXME: FIXED ARRAY SIZE OF 1KB !
            let t = Type::Primitive(PrimitiveType::Array(Box::new(inner_t), 1024));

            expect!(TokenType::CloseArray, ctx);

            Ok(t)
        } else if TokenType::OpenParens == token.t {
            ctx.consume();

            let t = Type::FuncType(FuncType::parse(ctx)?);

            expect!(TokenType::CloseParens, ctx);

            Ok(t)
        } else {
            panic!("Not a type");
        }
    }
}

impl Parse for FuncType {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut args = vec![];

        loop {
            let t = Type::parse(ctx)?;

            args.push(Box::new(t));

            if ctx.cur_tok().t != TokenType::Arrow {
                break;
            }

            ctx.consume();
        }

        let ret = args.pop().unwrap();

        let mut t_sig = FuncType::default();

        t_sig.arguments = args;
        t_sig.ret = ret;

        Ok(t_sig)
    }
}
