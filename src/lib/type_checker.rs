use std::collections::HashMap;

use super::ast::*;
use super::context::*;
use super::error::*;
use super::token::*;

pub struct TypeChecker {
    pub ctx: Context,
    pub ast: SourceFile,
}

impl TypeChecker {
    pub fn new(ast: SourceFile) -> TypeChecker {
        TypeChecker {
            ctx: Context::new(),
            ast,
        }
    }

    pub fn infer(&mut self) -> Result<TypeInfer, Error> {
        self.ast.infer(&mut self.ctx)
    }
}


pub trait TypeInferer {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error>;
}

impl TypeInferer for SourceFile {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = Err(Error::new_empty());

        let mut top_level_methods = vec![];

        for top in &mut self.top_levels {
            last = Ok(top.infer(ctx)?);
            match top {
                TopLevel::Class(class) => {
                    for method in &class.methods {
                        top_level_methods.push(method.clone());
                    }
                },
                _ => (),
            }
        }

        for method in top_level_methods {
            self.top_levels.push(TopLevel::Function(method));
        }

        last
    }
}

impl TypeInferer for TopLevel {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            TopLevel::Class(class) => class.infer(ctx),
            TopLevel::Function(fun) => fun.infer(ctx),
            TopLevel::Prototype(fun) => fun.infer(ctx),
            TopLevel::Mod(_) => Err(Error::new_empty()),
        }
    }
}

impl TypeInferer for Class {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        // let t = TypeInfer::Type(Some(Type::Name(self.name.clone())));
        let t = Some(Type::Class(self.name.clone()));

        ctx.scopes.add(self.name.clone(), t.clone());

        for attr in &mut self.attributes {
            attr.infer(ctx)?;
        }

        ctx.classes.insert(self.name.clone(), self.clone());

        for method in &mut self.methods {
            method.infer(ctx)?;
        }

        ctx.classes.insert(self.name.clone(), self.clone());

        Ok(t)
    }
}

impl TypeInferer for Attribute {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        if let Some(mut default) = self.default.clone() {
            let t = default.infer(ctx)?;

            if let Some(t2) = self.t.clone() {
                if t2.get_name() != t.clone().unwrap().get_name() {
                    return Err(Error::new_empty());
                }
            }

            self.t = t.clone();

            Ok(t)
        } else if let Some(_) = self.t.clone() {
            Ok(self.t.clone())
        } else {
            Err(Error::new_empty())
        }
    }
}

impl TypeInferer for Prototype {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.externs
            .insert(self.name.clone().unwrap(), self.name.clone().unwrap());

        ctx.scopes
            .add(self.name.clone().unwrap(), Some(Type::Proto(Box::new(self.clone()))));

        Ok(Some(Type::Primitive(PrimitiveType::Void)))
    }
}

impl TypeInferer for FunctionDecl {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {

        ctx.scopes.push();

        let mut types = vec![];

        for arg in &mut self.arguments {
            let t = arg.infer(ctx)?;

            arg.t = t.clone();

            types.push(arg.t.clone());
        }

        let last = self.body.infer(ctx)?;

        if self.ret.is_none() {
            self.ret = last.clone();
        }

        let mut i = 0;

        for arg in &mut self.arguments {
            arg.t = types[i].clone();

            i += 1;
        }

        ctx.scopes.pop();

        ctx.scopes.add(self.name.clone(), Some(Type::FuncType(Box::new(self.clone()))));

        Ok(last)
    }
}

impl TypeInferer for ArgumentDecl {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        ctx.scopes
            .add(self.name.clone(), self.t.clone());

        Ok(self.t.clone())
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
        let t = match &mut self.kind {
            StatementKind::If(if_) => if_.infer(ctx),
            StatementKind::For(for_) => for_.infer(ctx),
            StatementKind::Expression(expr) => expr.infer(ctx),
            StatementKind::Assignation(assign) => assign.infer(ctx),
        };

        self.t = t?;

        Ok(self.t.clone())
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
        match &mut self.kind {
            ExpressionKind::BinopExpr(unary, op, expr) => {
                let t = match op {
                    Operator::Add => Some(Type::Primitive(PrimitiveType::Int)),
                    Operator::EqualEqual => Some(Type::Primitive(PrimitiveType::Bool)),
                    _ => Some(Type::Primitive(PrimitiveType::Int)),
                };

                ctx.cur_type = t.clone();

                let _left = unary.infer(ctx)?;
                let _right = expr.infer(ctx)?;

                // if left != right {
                //     return Err(TypeError::new());
                // }

                // check left == right

                ctx.cur_type = None;

                Ok(t)
            }
            ExpressionKind::UnaryExpr(unary) => unary.infer(ctx),
        }
    }
}

impl TypeInferer for Assignation {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        // simple identifier
        if !self.name.has_secondaries() {
            let name = self.name.get_identifier().unwrap();
            let res = ctx.scopes.get(name.clone());

            if let Some(t) = res {
                let value_t = self.value.infer(ctx)?;

                if value_t.clone().unwrap().get_name() != t.clone().unwrap().get_name() {
                    return Err(Error::new_type_error(ctx.input.clone(), self.value.token.clone(), t.clone(), value_t.clone()));
                }

                self.t = t.clone();

                Ok(t)
            } else {
                let t = self.value.infer(ctx)?;

                self.t = t.clone();

                ctx.scopes.add(name.clone(), t.clone());

                Ok(t)
            }
        } else {
            self.name.infer(ctx)?;

            let t = self.value.infer(ctx)?;

            self.t = t.clone();

            Ok(t)
        }
    }
}

impl TypeInferer for UnaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.infer(ctx),
            UnaryExpr::UnaryExpr(_op, unary) => unary.infer(ctx),
        }
    }
}

impl TypeInferer for PrimaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, ref mut vec) => {
                ctx.cur_type = operand.infer(ctx)?;

                if vec.len() == 0 {
                    return Ok(ctx.cur_type.clone());
                }
                let mut prec = vec![];

                for second in vec {
                    match second {
                        SecondaryExpr::Arguments(ref mut args) => {
                            let mut res = vec![];

                            if let Some(Type::FuncType(f)) = &ctx.cur_type {
                                let mut name = f.name.clone();
                                let ret = f.ret.clone();
                                
                                if args.len() < f.arguments.len() {
                                    if let Some(classname) = &f.class_name {
                                        let this = Argument {
                                            token: Token::default(), 
                                            t: Some(Type::Class(classname.clone())), 
                                            arg: Expression {
                                                kind: ExpressionKind::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr::PrimaryExpr(operand.clone(), prec.clone()))),
                                                t: Some(Type::Class(classname.clone())),
                                                token: Token::default(),
                                            },
                                        };

                                        args.insert(0, this);
                                    }
                                }

                                let orig_name = name.clone();

                                for arg in args {
                                    let t = arg.infer(ctx)?;

                                    res.push(t.clone());

                                    name = name + &t.unwrap().get_name();
                                }

                                ctx.cur_type = ret;

                                ctx.calls
                                    .entry(orig_name.clone())
                                    .or_insert(HashMap::new())
                                    .insert(name, res);
                            } else if let Some(Type::Proto(_)) = &ctx.cur_type {

                            } else {
                                println!("AST {:?}", self);
                                panic!("WOUAT ?!");
                            }
                        }
                        // }

                        SecondaryExpr::Index(_args) => {
                            if let Some(t) = ctx.cur_type.clone() {
                                let t = t.clone();

                                if let Type::Primitive(PrimitiveType::Array(a, _n)) = t.clone() {
                                    ctx.cur_type = Some(*a);
                                } else if let Type::Primitive(PrimitiveType::String(_n)) = t.clone() {
                                    ctx.cur_type = Some(Type::Primitive(PrimitiveType::Int8));
                                } else {
                                    return Err(Error::new_not_indexable_error(t.get_name()))
                                }

                                // TODO
                                // if let Type::Primitive(_p) = t {
                                //     ctx.cur_type =
                                //         Some(Type::Primitive(PrimitiveType::Int8));
                                // }
                            }
                        }

                        SecondaryExpr::Selector(ref mut sel) => {
                            if let Some(t) = ctx.cur_type.clone() {
                                let classname = t.get_name();
                                let class = ctx.classes.get(&classname);

                                if class.is_none() {
                                    panic!("Unknown class {}", classname);
                                }

                                let class = class.unwrap();

                                let method_name = class.name.clone() + "_" + &sel.name.clone();

                                let f = class.get_method(method_name.clone());

                                if let Some(f) = f {
                                    let scope_f = ctx.scopes.get(f.name.clone()).clone().unwrap();
                                    
                                    ctx.cur_type = Some(Type::FuncType(Box::new(f)));

                                    if let Some(Type::FuncType(_)) = scope_f.clone() {
                                        ctx.cur_type = scope_f.clone();
                                    }

                                    sel.class_type = Some(t.clone()); // classname

                                    continue;
                                }

                                let attr = class.get_attribute(sel.name.clone());

                                if let None = attr {
                                    panic!("Unknown property {}", sel.name);
                                }

                                let (attr, i) = attr.unwrap();

                                sel.class_offset = i as u8; // attribute index

                                sel.class_type = Some(t.clone()); // classname

                                ctx.cur_type = attr.t.clone();
                            }
                        }
                    };

                    prec.push(second.clone());
                }

                Ok(ctx.cur_type.clone())
            }
        }
    }
}

impl TypeInferer for SecondaryExpr {
    fn infer(&mut self, _ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            _ => Err(Error::new_empty()),
        }
    }
}

impl TypeInferer for Argument {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let t = self.arg.infer(ctx);

        self.t = t?;

        Ok(self.t.clone())
    }
}

impl TypeInferer for Operand {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let t = match &mut self.kind {
            OperandKind::Literal(lit) => lit.infer(ctx),
            OperandKind::Identifier(ident) => {
                let res = ctx.scopes.get(ident.clone()).unwrap();

                if let None = res {
                    ctx.scopes.add(ident.clone(), ctx.cur_type.clone());

                    return Ok(ctx.cur_type.clone());
                } else {
                    Ok(res)
                }
            }
            OperandKind::ClassInstance(ci) => ci.infer(ctx),
            OperandKind::Array(arr) => arr.infer(ctx),
            OperandKind::Expression(expr) => expr.infer(ctx),
        };

        self.t = t?;

        Ok(self.t.clone())
    }
}

impl TypeInferer for ClassInstance {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        for class_attr in &self.class.attributes {
            if let Some(attr) = self.attributes.get(&class_attr.name) {
                let mut attr = attr.clone();
                let attr_t = attr.infer(ctx)?;

                if class_attr.t.clone().unwrap().get_name() != attr_t.clone().unwrap().get_name() {
                    return Err(Error::new_type_error(ctx.input.clone(), attr.token.clone(), class_attr.t.clone(),  attr_t.clone()));
                }
            } else if class_attr.default.is_none() {
                return Err(Error::new_undefined_error(self.class.name.clone() + "::" + &class_attr.name.clone()));
            }
        }

        Ok(ctx.scopes.get(self.name.clone()).unwrap())
    }
}

impl TypeInferer for Literal {
    fn infer(&mut self, _ctx: &mut Context) -> Result<TypeInfer, Error> {
        match &self {
            Literal::Number(_) => Ok(Some(Type::Primitive(PrimitiveType::Int))),
            Literal::String(s) => Ok(Some(Type::Primitive(PrimitiveType::String(s.len())))),
            Literal::Bool(_) => Ok(Some(Type::Primitive(PrimitiveType::Bool))),
        }
    }
}

impl TypeInferer for Array {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let mut last = None;

        for item in &mut self.items {
            let t = item.infer(ctx)?;

            if let None = last {
                last = t.clone();
            }

            if last.clone().unwrap().get_name() != t.clone().unwrap().get_name() {
                // TODO: type error
                return Err(Error::new_empty());
            }
        }

        self.t = Some(Type::Primitive(PrimitiveType::Array(
            Box::new(last.unwrap()),
            self.items.len(),
        )));

        Ok(self.t.clone())
    }
}
