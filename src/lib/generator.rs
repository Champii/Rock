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

    fn insert_toplevel_at(&mut self, i: usize, f: FunctionDecl) {
        self.ctx.scopes.add(f.name.clone(), Some(Type::FuncType(Box::new(f.clone()))));

        let i = if i > self.ast.top_levels.len() { 
            self.ast.top_levels.len()
        } else {
            i
        };

        self.ast.top_levels.insert(i, TopLevel::Function(f.clone()));
    }

    pub fn generate(&mut self) -> SourceFile {
        let main_scope = self.ctx.scopes.scopes.first().unwrap();
    
        let mut i = 0;
        let items = &mut main_scope.get_ordered().clone();


        for func in items {
            if let Some(Type::FuncType(ref mut f)) = func {
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

                if f.name == "main" {
                    f.infer(&mut self.ctx).unwrap();

                    self.insert_toplevel_at(i, *f.clone());

                    continue;
                }

                if !f.is_solved() {
                    let ctx_save = self.ctx.clone();

                    if let Some(calls) = ctx_save.calls.get(&f.name) {
                        for (_, call) in calls {
                            let mut new_f = f.clone();

                            new_f.apply_types(f.ret.clone(), call.clone());
                            new_f.infer(&mut self.ctx).unwrap();
                            new_f.apply_name(call.clone());

                            self.insert_toplevel_at(i, *new_f.clone());
                        } 
                    }

                    self.ctx = ctx_save;
                } else {
                    f.infer(&mut self.ctx).unwrap();
                    f.apply_name_self();

                    self.insert_toplevel_at(i, *f.clone());
                }
            }
            i += 1;
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
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

impl Generate for Prototype {
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
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
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
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
        match &mut self.kind {
            StatementKind::If(if_) => if_.generate(ctx),
            StatementKind::For(for_) => for_.generate(ctx),
            StatementKind::Expression(expr) => expr.generate(ctx),
            StatementKind::Assignation(assign) => assign.generate(ctx),
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
            Expression::BinopExpr(unary, _op, expr) => {
                let _left = unary.generate(ctx)?;
                let _right = expr.generate(ctx)?;

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
            UnaryExpr::UnaryExpr(_op, unary) => unary.generate(ctx),
        }
    }
}

impl Generate for PrimaryExpr {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            PrimaryExpr::PrimaryExpr(ref mut operand, vec) => {
                let mut s = String::new();
                let mut res = if let OperandKind::Identifier(ref mut id) = &mut operand.kind {
                    id
                } else {
                    &mut s
                };

                let mut last_method = None;
                let mut already_mangled = false;

                for second in vec {
                    match second {
                        SecondaryExpr::Selector(sel) => {
                            last_method = sel.class_type.clone();

                            if sel.full_name != sel.name {
                                already_mangled = true;
                            }

                            res = &mut sel.full_name;
                        },
                        SecondaryExpr::Arguments(args) => {
                                let mut name = res.clone();

                                if already_mangled {
                                    continue;
                                }

                                if let Some(classname) = last_method.clone() {
                                    name = classname.get_name() + "_" + &name;
                                }

                                let mut ctx_save = ctx.clone();

                                for arg in args {
                                    let t = arg.infer(&mut ctx_save).unwrap();
                                
                                    arg.generate(&mut ctx_save)?;

                                    name = name.to_owned() + &t.unwrap().get_name();
                                }

                                *ctx = ctx_save;

                                if ctx.externs.get(res).is_none() && !already_mangled {
                                    *res = name;
                                }
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
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
        match self {
            _ => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl Generate for Argument {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.arg.generate(ctx)
    }
}
