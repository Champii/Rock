use super::ast::*;
use super::context::*;
use super::error::Error;

pub struct TypeChecker {
    ctx: Context,
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

        let main_scope = self.ctx.scopes.scopes.first().unwrap();

        for (_, func) in &main_scope.items {
            if let TypeInfer::FuncType(f) = func {
                println!("SOLVED {}", f.solved);
                if !f.solved {
                    self.ast.top_levels = self
                        .ast
                        .top_levels
                        .iter()
                        .filter(|top| {
                            if let TopLevel::Function(fu) = top {
                                fu.name != f.func.name
                            } else {
                                true
                            }
                        })
                        .cloned()
                        .collect();

                    println!("CALLS {:?}", self.ctx.calls);
                    let mut ctx_save = self.ctx.clone();

                    for (_, calls) in &self.ctx.calls {
                        for call in calls {
                            let mut new_f = f.func.clone();

                            new_f.apply_types(f.ret.clone(), call.clone());

                            new_f.infer(&mut ctx_save);

                            self.ast.top_levels.insert(0, TopLevel::Function(new_f));
                        }
                    }
                }
            }
        }

        self.ast.clone()
    }
}

pub trait TypeInferer {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error>;
}

impl TypeInferer for SourceFile {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = Err(Error::new_empty());

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
            TopLevel::Mod(_) => Err(Error::new_empty()),
        }
    }
}

impl TypeInferer for Prototype {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.scopes.add(
            self.name.clone().unwrap(),
            TypeInfer::FuncType(FuncType::new_from_proto(self.clone())),
        );

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

        if self.t.is_none() {
            self.t = last.get_ret();
        }

        for arg in &mut self.arguments {
            arg.t = ctx.scopes.get(arg.name.clone()).unwrap().get_ret();
        }

        ctx.scopes.pop();

        ctx.scopes.add(
            self.name.clone(),
            TypeInfer::FuncType(FuncType::new(self.clone())),
        );

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
        let mut last = Err(Error::new_empty());

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
            Statement::Expression(expr) => expr.infer(ctx),
            Statement::Assignation(assign) => assign.infer(ctx),
        }
    }
}

impl TypeInferer for If {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        self.body.infer(ctx)
        //TODO: infer else
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

                unary.infer(ctx)?;
                expr.infer(ctx)?;

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
            Ok(t)
        } else {
            let t = self.value.infer(ctx)?;

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
                //
                ctx.cur_type = operand.infer(ctx)?;

                for second in vec {
                    match second {
                        SecondaryExpr::Arguments(args) => {
                            if let Operand::Identifier(id) = operand {
                                let mut res = vec![];

                                for arg in args {
                                    res.push(arg.infer(ctx)?);
                                }

                                ctx.calls.entry(id.clone()).or_insert(vec![]).push(res);
                            }
                        }
                        _ => (),
                    };
                    // second.infer(ctx)?;
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
            _ => Err(Error::new_empty()),
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
                let res = ctx.scopes.get(ident.clone()).unwrap();

                if let TypeInfer::Type(None) = res {
                    ctx.scopes.add(ident.clone(), ctx.cur_type.clone());

                    return Ok(ctx.cur_type.clone());
                } else {
                    Ok(res)
                }
            }
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
