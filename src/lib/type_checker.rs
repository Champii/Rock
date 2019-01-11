use super::ast::*;
use super::context::*;
use super::error::*;
use std::collections::HashMap;

pub struct TypeChecker {
    pub ctx: Context,
    ast: SourceFile,
}

impl TypeChecker {
    pub fn new(ast: SourceFile) -> TypeChecker {
        TypeChecker {
            ctx: Context::new(),
            ast,
        }
    }

    pub fn infer(&mut self) -> SourceFile {
        self.ast.infer(&mut self.ctx).unwrap();

        self.ast.clone()
    }
}

pub trait TypeInferer {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error>;
}

impl TypeInferer for SourceFile {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = Err(Error::ParseError(ParseError::new_empty()));

        for top in &mut self.top_levels {
            last = Ok(top.infer(ctx)?);
        }

        last
    }
}

impl TypeInferer for TopLevel {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            TopLevel::Function(fun) => fun.infer(ctx),
            TopLevel::Prototype(fun) => fun.infer(ctx),
            TopLevel::Mod(_) => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl TypeInferer for Prototype {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.externs
            .insert(self.name.clone().unwrap(), self.name.clone().unwrap());

        ctx.scopes
            .add(self.name.clone().unwrap(), TypeInfer::Proto(self.clone()));

        Ok(TypeInfer::Type(Some(Type::Name("Void".to_string()))))
    }
}

impl TypeInferer for FunctionDecl {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.scopes.push();

        for arg in &mut self.arguments {
            arg.infer(ctx)?;
        }

        let last = self.body.infer(ctx)?;

        if self.ret.is_none() {
            self.ret = last.get_ret();
        }

        for arg in &mut self.arguments {
            arg.t = ctx.scopes.get(arg.name.clone()).unwrap().get_ret();
        }

        ctx.scopes.pop();

        ctx.scopes
            .add(self.name.clone(), TypeInfer::FuncType(self.clone()));

        Ok(last)
    }
}

impl TypeInferer for ArgumentDecl {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.scopes
            .add(self.name.clone(), TypeInfer::Type(self.t.clone()));

        Ok(TypeInfer::Type(self.t.clone()))
    }
}

impl TypeInferer for Body {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = Err(Error::ParseError(ParseError::new_empty()));

        for stmt in &mut self.stmts {
            last = Ok(stmt.infer(ctx)?);
        }

        last
    }
}

impl TypeInferer for Statement {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            Statement::If(if_) => if_.infer(ctx),
            Statement::For(for_) => for_.infer(ctx),
            Statement::Expression(expr) => expr.infer(ctx),
            Statement::Assignation(assign) => assign.infer(ctx),
        }
    }
}

impl TypeInferer for For {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            For::In(in_) => in_.infer(ctx),
            For::While(while_) => while_.infer(ctx),
        }
    }
}

impl TypeInferer for ForIn {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        self.body.infer(ctx)
    }
}

impl TypeInferer for While {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        self.body.infer(ctx)
    }
}

impl TypeInferer for If {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        self.body.infer(ctx)
    }
}

impl TypeInferer for Expression {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            Expression::BinopExpr(unary, op, expr) => {
                let t = match op {
                    Operator::Add => TypeInfer::Type(Some(Type::Name("Int".to_string()))),
                    Operator::EqualEqual => TypeInfer::Type(Some(Type::Name("Bool".to_string()))),
                    _ => TypeInfer::Type(Some(Type::Name("Int".to_string()))),
                };

                ctx.cur_type = t.clone();

                let left = unary.infer(ctx)?;
                let right = expr.infer(ctx)?;

                // if left != right {
                //     return Err(TypeError::new());
                // }

                // check left == right

                ctx.cur_type = TypeInfer::Type(None);

                Ok(t)
            }
            Expression::UnaryExpr(unary) => unary.infer(ctx),
        }
    }
}

impl TypeInferer for Assignation {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        // if
        let res = ctx.scopes.get(self.name.clone());

        if let Some(t) = res {
            self.t = t.clone().get_ret();

            Ok(t)
        } else {
            let t = self.value.infer(ctx)?;

            self.t = t.clone().get_ret();

            ctx.scopes.add(self.name.clone(), t.clone());

            Ok(t)
        }
    }
}

impl TypeInferer for UnaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.infer(ctx),
            UnaryExpr::UnaryExpr(op, unary) => unary.infer(ctx),
        }
    }
}

impl TypeInferer for PrimaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, vec) => {
                ctx.cur_type = operand.infer(ctx)?;

                for second in vec {
                    match second {
                        SecondaryExpr::Arguments(args) => {
                            if let Operand::Identifier(id) = operand {
                                let mut res = vec![];
                                let mut name = id.clone();

                                for arg in args {
                                    let t = arg.infer(ctx)?;

                                    res.push(t.clone());

                                    name = name + &t.get_ret().unwrap().get_name();
                                }

                                ctx.calls
                                    .entry(id.clone())
                                    .or_insert(HashMap::new())
                                    .insert(name, res);
                            }
                        }

                        SecondaryExpr::Index(args) => {
                            if let TypeInfer::Type(t) = ctx.cur_type.clone() {
                                let t = t.clone().unwrap();

                                if let Type::Array(a, n) = t.clone() {
                                    ctx.cur_type = TypeInfer::Type(Some(a.get_inner()));
                                }

                                if let Type::Name(n) = t {
                                    ctx.cur_type =
                                        TypeInfer::Type(Some(Type::Name("Int8".to_string())));
                                }
                            }
                        }
                        _ => (),
                    };
                }

                Ok(ctx.cur_type.clone())
            }
        }
    }
}

impl TypeInferer for SecondaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            // SecondaryExpr::Arguments(args) => {
            //     //
            // }
            _ => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl TypeInferer for Argument {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        self.arg.infer(ctx)
    }
}

impl TypeInferer for Operand {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            Operand::Literal(lit) => lit.infer(ctx),
            Operand::Identifier(ident) => {
                // println!("LOL {} {:#?}", ident, ctx.scopes);
                let res = ctx.scopes.get(ident.clone()).unwrap();

                if let TypeInfer::Type(None) = res {
                    ctx.scopes.add(ident.clone(), ctx.cur_type.clone());

                    return Ok(ctx.cur_type.clone());
                } else {
                    Ok(res)
                }
            }

            Operand::Array(arr) => arr.infer(ctx),
            Operand::Expression(expr) => expr.infer(ctx),
        }
    }
}

impl TypeInferer for Literal {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match &self {
            Literal::Number(_) => Ok(TypeInfer::Type(Some(Type::Name("Int".to_string())))),
            Literal::String(_) => Ok(TypeInfer::Type(Some(Type::Name("String".to_string())))),
            Literal::Bool(_) => Ok(TypeInfer::Type(Some(Type::Name("Bool".to_string())))),
        }
    }
}

impl TypeInferer for Array {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = TypeInfer::Type(None);

        for item in &mut self.items {
            let t = item.infer(ctx)?;

            if let TypeInfer::Type(None) = last {
                last = t.clone();
            }

            if last != t {
                // TODO: type error
                return Err(Error::ParseError(ParseError::new_empty()));
            }
        }

        self.t = Some(Type::Array(
            Box::new(last.get_ret().unwrap()),
            self.items.len(),
        ));

        Ok(TypeInfer::Type(self.t.clone()))
        // Ok(TypeInfer::Type(last.get_ret()))
    }
}
