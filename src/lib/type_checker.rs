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
            TopLevel::Mod(_) => Err(Error::ParseError(ParseError::new_empty())),
        }
    }
}

impl TypeInferer for Class {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        let t = TypeInfer::Type(Some(Type::Name(self.name.clone())));

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
                if t2 != t.get_ret().unwrap() {
                    return Err(Error::ParseError(ParseError::new_empty()));
                }
            }

            self.t = t.get_ret();

            Ok(t)
        } else if let Some(t) = self.t.clone() {
            Ok(TypeInfer::Type(Some(t)))
        } else {
            Err(Error::ParseError(ParseError::new_empty()))
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

        let mut types = vec![];

        for arg in &mut self.arguments {
            let t = arg.infer(ctx)?;
            arg.t = t.get_type();

            types.push(arg.t.clone());
        }

        let last = self.body.infer(ctx)?;

        println!("LAST {:?}", last);

        if self.ret.is_none() {
            println!("SET RET {:?}", last.get_ret());
            self.ret = last.get_ret();
        }

        println!("FUNDECL ARGS {:?}", self.arguments);

        let mut i = 0;

        for arg in &mut self.arguments {
            arg.t = types[i].clone();
            // arg.t = ctx.scopes.get(arg.name.clone()).unwrap().get_ret();
            i += 1;
        }

        ctx.scopes.pop();

        ctx.scopes.add(self.name.clone(), TypeInfer::FuncType(self.clone()));

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
            PrimaryExpr::PrimaryExpr(operand, ref mut vec) => {
                ctx.cur_type = operand.infer(ctx)?;

                if vec.len() == 0 {
                    return Ok(ctx.cur_type.clone());
                }
                let mut prec = vec![];

                for second in vec {
                    match second {
                        SecondaryExpr::Arguments(ref mut args) => {
                            // let mut args = args.clone();
                            let mut name = String::new();

                            let mut res = vec![];

                            if let Some(f) = ctx.cur_type.get_fn_type() {

                                name = f.name.clone();
                                let ret = TypeInfer::Type(f.ret.clone());
                                
                                if args.len() < f.arguments.len() {
                                    if let Some(_) = &f.class_name {
                                        // name = classdef.name.clone() + "_" + &f.name.clone();
                                        let this = Argument {arg: Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr::PrimaryExpr(operand.clone(), prec.clone())))};

                                        args.insert(0, this);
                                    }
                                }

                                let orig_name = name.clone();

                                for mut arg in args {
                                    let t = arg.infer(ctx)?;

                                    res.push(t.clone());

                                    name = name + &t.get_ret().unwrap().get_name();
                                }

                                println!("CUR_TYPE RET {:?}", ret);

                                ctx.cur_type = ret;

                                ctx.calls
                                    .entry(orig_name.clone())
                                    .or_insert(HashMap::new())
                                    .insert(name, res);
                            } else {
                                panic!("WOUAT ?!");
                            }
                        }
                        // }

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

                        SecondaryExpr::Selector(ref mut sel) => {
                            if let TypeInfer::Type(name) = ctx.cur_type.clone() {
                                let classname = name.clone().unwrap().get_name();
                                let class = ctx.classes.get(&classname);

                                if class.is_none() {
                                    panic!("Unknown class {}", classname);
                                }

                                let class = class.unwrap();

                                let method_name = class.name.clone() + "_" + &sel.name.clone();

                                let f = class.get_method(method_name.clone());

                                if let Some(f) = f {
                                    // sel.0 = method_name.clone();
                                    let mut scope_f = ctx.scopes.get(f.name.clone()).clone().unwrap();
                                    println!("SCOPE F {:?}", scope_f);
                                    
                                    ctx.cur_type = TypeInfer::FuncType(f);

                                    if let TypeInfer::FuncType(scope_f2) = scope_f.clone() {
                                        ctx.cur_type = scope_f.clone();
                                    }

                                    sel.class_name = name.clone(); // classname

                                    continue;
                                }

                                let attr = class.get_attribute(sel.name.clone());

                                if let None = attr {
                                    panic!("Unknown property {}", sel.name);
                                }

                                let (attr, i) = attr.unwrap();

                                sel.class_offset = i as u8; // attribute index

                                sel.class_name = name.clone(); // classname

                                ctx.cur_type = TypeInfer::Type(attr.t);
                            }
                        }
                        _ => (),
                    };

                    prec.push(second.clone());
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
            Operand::ClassInstance(ci) => ci.infer(ctx),
            Operand::Array(arr) => arr.infer(ctx),
            Operand::Expression(expr) => expr.infer(ctx),
        }
    }
}

impl TypeInferer for ClassInstance {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        // TODO: check types of fields
        Ok(ctx.scopes.get(self.name.clone()).unwrap())
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
