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
    
        let mut i = 0;
        let mut items = &mut main_scope.get_ordered().clone();


        for mut func in items {
        // for top in self.ast.top_levels {
            // let (_, func) = items.get(top);

            if let TypeInfer::FuncType(ref mut f) = func {
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

                if !f.is_solved() {
                    let mut ctx_save = self.ctx.clone();

                    println!("GENERATE {} {:?}", f.name, self.ctx.calls);

                    if let Some(calls) = self.ctx.calls.get(&f.name) {
                        for (_, call) in calls {
                            let mut new_f = f.clone();

                            new_f.apply_types(f.ret.clone(), call.clone());
                            new_f.infer(&mut ctx_save).unwrap();
                            // new_f.generate(&mut ctx_save).unwrap();
                            new_f.apply_name(f.ret.clone(), call.clone());

                            self.ast.top_levels.insert(i, TopLevel::Function(new_f.clone()));

                            self.ctx.scopes.add(new_f.name.clone(), TypeInfer::FuncType(new_f));
                        } 
                    } else {
                        f.infer(&mut self.ctx).unwrap();

                        // f.generate(&mut self.ctx);
                        f.apply_name_self();

                        // f.infer(&mut self.ctx).unwrap();

                        self.ctx.scopes.add(f.name.clone(), TypeInfer::FuncType(f.clone()));
                        self.ast.top_levels.insert(i, TopLevel::Function(f.clone()));
                    }

                    // self.ctx.scopes.remove(f.name.clone());
                } else {
                    f.infer(&mut self.ctx);
                    // f.generate(&mut self.ctx);
                    // f.infer(&mut self.ctx);
                    f.apply_name_self();
                    self.ctx.scopes.add(f.name.clone(), TypeInfer::FuncType(f.clone()));
                    self.ast.top_levels.insert(i, TopLevel::Function(f.clone()));
                }
            }
            i += 1;
        }


        println!("GEN AST BEFORE {:#?}", self.ast);

        self.ast.generate(&mut self.ctx).unwrap();
        // self.ast.infer(&mut self.ctx).unwrap();

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
        // for method in &mut self.methods {
        //     method.generate(ctx)?;
        // }

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

                let mut last_method = None;
                let mut already_mangled = false;

                for second in vec {
                    match second {
                        SecondaryExpr::Selector(sel) => {
                            println!("SELECTOR ?!?! {:?}", sel.class_name);
                            last_method = sel.class_name.clone();

                            if sel.full_name != sel.name {
                                already_mangled = true;
                            }

                            res = &mut sel.full_name;
                        },
                        SecondaryExpr::Arguments(args) => {
                            // if let Operand::Identifier(ref mut id) = operand {
                                // let mut res = (*id).to_string();
                                let mut name = res.clone();

                                if already_mangled {
                                    continue;
                                }

                                println!("LAST_METHOD? {:#?}", last_method);

                                if let Some(classname) = last_method.clone() {
                                    name = classname.get_name() + "_" + &name;
                                }

                                // let orig_name = res.clone();
                                let orig_name = name.clone();
                                println!("GEN ARG BEFORE?! {:?}", args);

                                let mut ctx_save = ctx.clone();
                                for arg in args {
                                    let t = arg.infer(&mut ctx_save).unwrap();
                                
                                    arg.generate(&mut ctx_save)?;

                                    name = name.to_owned() + &t.get_ret().unwrap().get_name();

                                    println!("GEN ARG ?! {:?}", name);
                                }

                                let mut funcs = &mut ctx.scopes.scopes.first_mut().unwrap().items;

                                println!("FUNCSSSS {} {:#?}", name, funcs);

                                let that = funcs.get_mut(&name).unwrap();

                                let solved = if let TypeInfer::FuncType(ref mut f) = that {
                                    // name = name + &f.ret.clone().unwrap().get_name();
                                    if f.ret.is_none() {
                                        // f.ret = 
                                        // f.ret = f.infer(&mut ctx_save).unwrap().get_type();
                                    }
                                    true
                                } else {
                                    true
                                };

                                if ctx.externs.get(res).is_none() && !already_mangled {
                                    *res = name;
                                }

                                println!("GEN ARGS TYPE {}", res);
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
