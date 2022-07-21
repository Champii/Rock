use paste::paste;

use crate::ast::tree::*;
use crate::ty::*;

macro_rules! generate_visitor_mut_trait {
    ($(
        $name:ty
    )+) => {
        pub trait VisitorMut<'a>: Sized {
            fn visit_name(&mut self, _name: &mut String) {}

            fn visit_primitive<T>(&mut self, _val: T)
            where
                T: std::fmt::Debug,
            {}

            paste! {
                $(
                    fn [<visit_ $name:snake>](&mut self, node: &'a mut $name) {
                        [<walk_ $name:snake>](self, node);
                    }
                )+
            }
        }
    };
}

generate_visitor_mut_trait!(
    Root
    Mod
    TopLevel
    Assign
    AssignLeftSide
    Prototype
    Use
    Trait
    Impl
    FunctionDecl
    StructDecl
    Identifier
    IdentifierPath
    Body
    Statement
    For
    ForIn
    While
    Expression
    If
    Else
    UnaryExpr
    Operator
    PrimaryExpr
    SecondaryExpr
    Operand
    Argument
    Literal
    StructCtor
    Array
    NativeOperator
    FuncType
    Type
);

pub fn walk_root<'a, V: VisitorMut<'a>>(visitor: &mut V, root: &'a mut Root) {
    visitor.visit_mod(&mut root.r#mod);
}

pub fn walk_mod<'a, V: VisitorMut<'a>>(visitor: &mut V, _mod: &'a mut Mod) {
    walk_list!(visitor, visit_top_level, &mut _mod.top_levels);
}

pub fn walk_top_level<'a, V: VisitorMut<'a>>(visitor: &mut V, top_level: &'a mut TopLevel) {
    match top_level {
        TopLevel::Extern(p) => visitor.visit_prototype(p),
        TopLevel::FnSignature(p) => visitor.visit_prototype(p),
        TopLevel::Use(u) => visitor.visit_use(u),
        TopLevel::Trait(t) => visitor.visit_trait(t),
        TopLevel::Impl(i) => visitor.visit_impl(i),
        TopLevel::Struct(i) => visitor.visit_struct_decl(i),
        TopLevel::Mod(name, m) => {
            visitor.visit_identifier(name);
            visitor.visit_mod(m);
        }
        TopLevel::Function(f) => visitor.visit_function_decl(f),
        TopLevel::Infix(ident, _) => visitor.visit_operator(ident),
    };
}

pub fn walk_struct_decl<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructDecl) {
    visitor.visit_identifier(&mut s.name);

    walk_list!(visitor, visit_prototype, &mut s.defs);
}

pub fn walk_trait<'a, V: VisitorMut<'a>>(visitor: &mut V, t: &'a mut Trait) {
    visitor.visit_type(&mut t.name);

    walk_list!(visitor, visit_type, &mut t.types);

    walk_list!(visitor, visit_prototype, &mut t.defs);
}

pub fn walk_impl<'a, V: VisitorMut<'a>>(visitor: &mut V, i: &'a mut Impl) {
    visitor.visit_type(&mut i.name);

    walk_list!(visitor, visit_type, &mut i.types);

    walk_list!(visitor, visit_function_decl, &mut i.defs);
}

pub fn walk_prototype<'a, V: VisitorMut<'a>>(visitor: &mut V, prototype: &'a mut Prototype) {
    visitor.visit_identifier(&mut prototype.name);

    visitor.visit_func_type(&mut prototype.signature);
}

pub fn walk_use<'a, V: VisitorMut<'a>>(visitor: &mut V, r#use: &'a mut Use) {
    visitor.visit_identifier_path(&mut r#use.path);
}

pub fn walk_function_decl<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    function_decl: &'a mut FunctionDecl,
) {
    visitor.visit_identifier(&mut function_decl.name);

    walk_list!(visitor, visit_identifier, &mut function_decl.arguments);

    visitor.visit_body(&mut function_decl.body);
}

pub fn walk_identifier_path<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    identifier_path: &'a mut IdentifierPath,
) {
    walk_list!(visitor, visit_identifier, &mut identifier_path.path);
}

pub fn walk_identifier<'a, V: VisitorMut<'a>>(visitor: &mut V, identifier: &'a mut Identifier) {
    visitor.visit_name(&mut identifier.name);
}

pub fn walk_body<'a, V: VisitorMut<'a>>(visitor: &mut V, body: &'a mut Body) {
    walk_list!(visitor, visit_statement, &mut body.stmts);
}

pub fn walk_statement<'a, V: VisitorMut<'a>>(visitor: &mut V, statement: &'a mut Statement) {
    match statement {
        Statement::Expression(expr) => visitor.visit_expression(expr),
        Statement::Assign(assign) => visitor.visit_assign(assign),
        Statement::If(expr) => visitor.visit_if(expr),
        Statement::For(for_loop) => visitor.visit_for(for_loop),
    }
}

pub fn walk_for<'a, V: VisitorMut<'a>>(visitor: &mut V, for_loop: &'a mut For) {
    match for_loop {
        For::In(for_in) => visitor.visit_for_in(for_in),
        For::While(while_loop) => visitor.visit_while(while_loop),
    }
}

pub fn walk_for_in<'a, V: VisitorMut<'a>>(visitor: &mut V, for_in: &'a mut ForIn) {
    visitor.visit_identifier(&mut for_in.value);
    visitor.visit_expression(&mut for_in.expr);
    visitor.visit_body(&mut for_in.body);
}

pub fn walk_while<'a, V: VisitorMut<'a>>(visitor: &mut V, while_loop: &'a mut While) {
    visitor.visit_expression(&mut while_loop.predicat);
    visitor.visit_body(&mut while_loop.body);
}

pub fn walk_assign_left_side<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    assign_left: &'a mut AssignLeftSide,
) {
    match assign_left {
        AssignLeftSide::Identifier(id) => visitor.visit_expression(id),
        AssignLeftSide::Indice(expr) => visitor.visit_expression(expr),
        AssignLeftSide::Dot(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_assign<'a, V: VisitorMut<'a>>(visitor: &mut V, assign: &'a mut Assign) {
    visitor.visit_assign_left_side(&mut assign.name);
    visitor.visit_expression(&mut assign.value);
}

pub fn walk_if<'a, V: VisitorMut<'a>>(visitor: &mut V, r#if: &'a mut If) {
    visitor.visit_expression(&mut r#if.predicat);
    visitor.visit_body(&mut r#if.body);
    if let Some(r#else) = &mut r#if.else_ {
        visitor.visit_else(r#else);
    }
}

pub fn walk_else<'a, V: VisitorMut<'a>>(visitor: &mut V, r#else: &'a mut Else) {
    match r#else {
        Else::If(expr) => visitor.visit_if(expr),
        Else::Body(expr) => visitor.visit_body(expr),
    }
}

pub fn walk_expression<'a, V: VisitorMut<'a>>(visitor: &mut V, expr: &'a mut Expression) {
    match expr {
        Expression::BinopExpr(unary, operator, expr) => {
            visitor.visit_unary_expr(unary);
            visitor.visit_operator(operator);
            visitor.visit_expression(&mut *expr);
        }
        Expression::UnaryExpr(unary) => visitor.visit_unary_expr(unary),
        Expression::StructCtor(ctor) => visitor.visit_struct_ctor(ctor),
        Expression::NativeOperation(op, left, right) => {
            visitor.visit_identifier(left);
            visitor.visit_identifier(right);
            visitor.visit_native_operator(op);
        }
        Expression::Return(expr) => {
            visitor.visit_expression(expr);
        }
    }
}

pub fn walk_struct_ctor<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructCtor) {
    visitor.visit_identifier(&mut s.name);

    walk_map!(visitor, visit_expression, &mut s.defs);
}

pub fn walk_unary_expr<'a, V: VisitorMut<'a>>(visitor: &mut V, unary: &'a mut UnaryExpr) {
    match unary {
        UnaryExpr::PrimaryExpr(primary) => visitor.visit_primary_expr(primary),
        UnaryExpr::UnaryExpr(op, unary) => {
            visitor.visit_operator(op);
            visitor.visit_unary_expr(&mut *unary);
        }
    }
}

pub fn walk_primary_expr<'a, V: VisitorMut<'a>>(visitor: &mut V, primary: &'a mut PrimaryExpr) {
    visitor.visit_operand(&mut primary.op);

    if let Some(secondaries) = &mut primary.secondaries {
        walk_list!(visitor, visit_secondary_expr, secondaries);
    }
}

pub fn walk_secondary_expr<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    secondary: &'a mut SecondaryExpr,
) {
    match secondary {
        SecondaryExpr::Arguments(args) => {
            walk_list!(visitor, visit_argument, args);
        }
        SecondaryExpr::Indice(expr) => {
            visitor.visit_expression(expr);
        }
        SecondaryExpr::Dot(expr) => {
            visitor.visit_identifier(expr);
        }
    }
}

pub fn walk_operator<'a, V: VisitorMut<'a>>(visitor: &mut V, operator: &'a mut Operator) {
    visitor.visit_identifier(&mut operator.0)
}

pub fn walk_operand<'a, V: VisitorMut<'a>>(visitor: &mut V, operand: &'a mut Operand) {
    match operand {
        Operand::Literal(l) => visitor.visit_literal(l),
        Operand::Identifier(i) => visitor.visit_identifier_path(i),
        Operand::Expression(e) => visitor.visit_expression(&mut *e),
    }
}

pub fn walk_argument<'a, V: VisitorMut<'a>>(visitor: &mut V, argument: &'a mut Argument) {
    visitor.visit_unary_expr(&mut argument.arg);
}

pub fn walk_literal<'a, V: VisitorMut<'a>>(visitor: &mut V, literal: &'a mut Literal) {
    match &mut literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::Float(f) => visitor.visit_primitive(f),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
        LiteralKind::Array(arr) => visitor.visit_array(arr),
        LiteralKind::Char(c) => visitor.visit_primitive(c),
    }
}

pub fn walk_array<'a, V: VisitorMut<'a>>(visitor: &mut V, arr: &'a mut Array) {
    walk_list!(visitor, visit_expression, &mut arr.values);
}

pub fn walk_native_operator<'a, V: VisitorMut<'a>>(
    _visitor: &mut V,
    _operator: &'a mut NativeOperator,
) {
    // Nothing to do
}

pub fn walk_func_type<'a, V: VisitorMut<'a>>(visitor: &mut V, signature: &'a mut FuncType) {
    walk_list!(visitor, visit_type, &mut signature.arguments);

    visitor.visit_type(&mut signature.ret);
}

pub fn walk_type<'a, V: VisitorMut<'a>>(_visitor: &mut V, _t: &'a mut Type) {
    // Nothing to do
}
