use super::ast::*;
use super::context::*;
use super::error::*;
use super::type_checker::*;

pub struct Generator {
    ast: SourceFile,
    ctx: Context,
}

impl Generator {
    pub fn new(ast: SourceFile, ctx: Context) -> Generator {
        Generator { ast, ctx }
    }

    pub fn generate(&mut self) -> SourceFile {
        let main_scope = self.ctx.scopes.scopes.first().unwrap();

        for (_, func) in &main_scope.items {
            if let TypeInfer::FuncType(f) = func {
                if !f.is_solved() {
                    self.ast.top_levels = self
                        .ast
                        .top_levels
                        .iter()
                        .filter(|top| {
                            if let TopLevel::Function(fu) = top {
                                fu.name != f.name
                            } else {
                                true
                            }
                        })
                        .cloned()
                        .collect();

                    let mut ctx_save = self.ctx.clone();

                    for (_, call) in &self.ctx.calls[&f.name] {
                        let mut new_f = f.clone();

                        new_f.apply_types(f.ret.clone(), call.clone());

                        new_f.infer(&mut ctx_save).unwrap();

                        // new_f.generate(&mut ctx_save);

                        self.ast.top_levels.insert(0, TopLevel::Function(new_f));
                    }
                }
            }
        }

        self.ast.generate(&mut self.ctx).unwrap();

        self.ast.clone()
    }
}

trait Generate {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error>;
}

impl Generate for SourceFile {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        for top in &mut self.top_levels {
            top.generate(ctx)?;
        }

        Ok(())
    }
}

impl Generate for TopLevel {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            TopLevel::Class(class) => class.generate(ctx),
            TopLevel::Function(fun) => fun.generate(ctx),
            TopLevel::Prototype(fun) => fun.generate(ctx),
            TopLevel::Mod(_) => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl Generate for Class {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        for method in &mut self.methods {
            method.generate(ctx)?;
        }

        Ok(())
    }
}

impl Generate for Prototype {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

impl Generate for FunctionDecl {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        ctx.scopes.push();

        let res = self.body.generate(ctx);

        ctx.scopes.pop();

        res
    }
}

impl Generate for ArgumentDecl {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

impl Generate for Body {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        for stmt in &mut self.stmts {
            stmt.generate(ctx)?;
        }

        Ok(())
    }
}

impl Generate for Statement {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            Statement::If(if_) => if_.generate(ctx),
            Statement::For(for_) => for_.generate(ctx),
            Statement::Expression(expr) => expr.generate(ctx),
            Statement::Assignation(assign) => assign.generate(ctx),
        }
    }
}

impl Generate for If {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.body.generate(ctx)
    }
}

impl Generate for For {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            For::In(in_) => in_.generate(ctx),
            For::While(while_) => while_.generate(ctx),
        }
    }
}

impl Generate for ForIn {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.body.generate(ctx)
    }
}

impl Generate for While {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.body.generate(ctx)
    }
}

impl Generate for Expression {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            Expression::BinopExpr(unary, op, expr) => {
                let left = unary.generate(ctx)?;
                let right = expr.generate(ctx)?;

                Ok(())
            }
            Expression::UnaryExpr(unary) => unary.generate(ctx),
        }
    }
}

impl Generate for Assignation {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.infer(ctx)?;

        self.value.generate(ctx)
    }
}

impl Generate for UnaryExpr {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.generate(ctx),
            UnaryExpr::UnaryExpr(op, unary) => unary.generate(ctx),
        }
    }
}

impl Generate for PrimaryExpr {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            PrimaryExpr::PrimaryExpr(ref mut operand, vec) => {
                let mut s = String::new();
                let mut res = if let Operand::Identifier(ref mut id) = operand {
                    id
                } else {
                    &mut s
                };

                for second in vec {
                    match second {
                        SecondaryExpr::Selector((name, _, _)) => {
                            res = name;
                        },
                        SecondaryExpr::Arguments(args) => {
                            // if let Operand::Identifier(ref mut id) = operand {
                                // let mut res = (*id).to_string();

                                for arg in args {
                                    let t = arg.infer(ctx).unwrap();

                                    arg.generate(ctx)?;

                                    // res = res + &t.get_ret().unwrap().get_name();
                                }

                                let funcs = ctx.scopes.scopes.first().unwrap().items.clone();

                                let that = funcs.get(res).unwrap();

                                let solved = if let TypeInfer::FuncType(f) = that {
                                    f.is_solved()
                                } else {
                                    true
                                };

                                // Check if commenting this broke the whole 
                                // if ctx.externs.get(res).is_none() && !solved {
                                //     *id = res;
                                // }
                            // }
                        }
                        _ => (),
                    };
                }

                Ok(())
            }
        }
    }
}

impl Generate for SecondaryExpr {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            // SecondaryExpr::Arguments(args) => {
            //     //
            //     // for arg in args {}
            // }
            _ => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl Generate for Argument {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.arg.generate(ctx)
    }
}
