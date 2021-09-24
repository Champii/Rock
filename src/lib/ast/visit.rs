use concat_idents::concat_idents;

use crate::walk_list;
use crate::{ast::*, walk_map};

macro_rules! generate_visitor_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {}

            fn visit_primitive<T>(&mut self, _val: T)
            where
                T: std::fmt::Debug,
            {}

            $(
                concat_idents!(fn_name = visit_, $method {
                    fn fn_name(&mut self, $method: &'ast $name) {
                        concat_idents!(fn2_name = walk_, $method {
                            fn2_name(self, $method);
                        });
                    }
                });
            )*
        }
    };
}

generate_visitor_trait!(
    Root, root
    Mod, r#mod
    TopLevel, top_level
    Assign, assign
    AssignLeftSide, assign_left_side
    Prototype, prototype
    Use, r#use
    Trait, r#trait
    Impl, r#impl
    FunctionDecl, function_decl
    StructDecl, struct_decl
    Identifier, identifier
    IdentifierPath, identifier_path
    ArgumentDecl, argument_decl
    Body, body
    Statement, statement
    Expression, expression
    If, r#if
    Else, r#else
    UnaryExpr, unary
    Operator, operator
    PrimaryExpr, primary_expr
    SecondaryExpr, secondary_expr
    Operand, operand
    Argument, argument
    Literal, literal
    StructCtor, struct_ctor
    Array, array
    NativeOperator, native_operator
    TypeSignature, type_signature
    Type, r#type
);

pub fn walk_root<'a, V: Visitor<'a>>(visitor: &mut V, root: &'a Root) {
    visitor.visit_mod(&root.r#mod);
}

pub fn walk_mod<'a, V: Visitor<'a>>(visitor: &mut V, _mod: &'a Mod) {
    walk_list!(visitor, visit_top_level, &_mod.top_levels);
}

pub fn walk_top_level<'a, V: Visitor<'a>>(visitor: &mut V, top_level: &'a TopLevel) {
    match &top_level.kind {
        TopLevelKind::Prototype(p) => visitor.visit_prototype(&p),
        TopLevelKind::Use(u) => visitor.visit_use(&u),
        TopLevelKind::Trait(t) => visitor.visit_trait(&t),
        TopLevelKind::Impl(i) => visitor.visit_impl(&i),
        TopLevelKind::Struct(i) => visitor.visit_struct_decl(&i),
        TopLevelKind::Mod(name, m) => {
            visitor.visit_identifier(&name);
            visitor.visit_mod(&m);
        }
        TopLevelKind::Function(f) => visitor.visit_function_decl(&f),
        TopLevelKind::Infix(ident, _) => visitor.visit_identifier(&ident),
    };
}

pub fn walk_struct_decl<'a, V: Visitor<'a>>(visitor: &mut V, s: &'a StructDecl) {
    visitor.visit_type(&s.name);

    walk_list!(visitor, visit_prototype, &s.defs);
}

pub fn walk_trait<'a, V: Visitor<'a>>(visitor: &mut V, t: &'a Trait) {
    visitor.visit_type(&t.name);

    walk_list!(visitor, visit_type, &t.types);

    walk_list!(visitor, visit_prototype, &t.defs);
}

pub fn walk_impl<'a, V: Visitor<'a>>(visitor: &mut V, i: &'a Impl) {
    visitor.visit_type(&i.name);

    walk_list!(visitor, visit_type, &i.types);

    walk_list!(visitor, visit_function_decl, &i.defs);
}

pub fn walk_prototype<'a, V: Visitor<'a>>(visitor: &mut V, prototype: &'a Prototype) {
    visitor.visit_identifier(&prototype.name);

    visitor.visit_type_signature(&prototype.signature);
}

pub fn walk_use<'a, V: Visitor<'a>>(visitor: &mut V, r#use: &'a Use) {
    visitor.visit_identifier_path(&r#use.path);
}

pub fn walk_function_decl<'a, V: Visitor<'a>>(visitor: &mut V, function_decl: &'a FunctionDecl) {
    visitor.visit_identifier(&function_decl.name);

    walk_list!(visitor, visit_argument_decl, &function_decl.arguments);

    visitor.visit_body(&function_decl.body);
}

pub fn walk_identifier_path<'a, V: Visitor<'a>>(
    visitor: &mut V,
    identifier_path: &'a IdentifierPath,
) {
    walk_list!(visitor, visit_identifier, &identifier_path.path);
}

pub fn walk_identifier<'a, V: Visitor<'a>>(visitor: &mut V, identifier: &'a Identifier) {
    visitor.visit_name(identifier.name.clone());
}

pub fn walk_argument_decl<'a, V: Visitor<'a>>(visitor: &mut V, argument: &'a ArgumentDecl) {
    visitor.visit_name(argument.name.clone());
}

pub fn walk_body<'a, V: Visitor<'a>>(visitor: &mut V, body: &'a Body) {
    walk_list!(visitor, visit_statement, &body.stmts);
}

pub fn walk_statement<'a, V: Visitor<'a>>(visitor: &mut V, statement: &'a Statement) {
    match statement.kind.as_ref() {
        StatementKind::Expression(expr) => visitor.visit_expression(&expr),
        StatementKind::Assign(assign) => visitor.visit_assign(&assign),
        StatementKind::If(expr) => visitor.visit_if(&expr),
    }
}

pub fn walk_assign_left_side<'a, V: Visitor<'a>>(visitor: &mut V, assign_left: &'a AssignLeftSide) {
    match assign_left {
        AssignLeftSide::Identifier(id) => visitor.visit_identifier(id),
        AssignLeftSide::Indice(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_assign<'a, V: Visitor<'a>>(visitor: &mut V, assign: &'a Assign) {
    visitor.visit_assign_left_side(&assign.name);
    visitor.visit_expression(&assign.value);
}

pub fn walk_if<'a, V: Visitor<'a>>(visitor: &mut V, r#if: &'a If) {
    visitor.visit_expression(&r#if.predicat);
    visitor.visit_body(&r#if.body);
    if let Some(r#else) = &r#if.else_ {
        visitor.visit_else(&r#else);
    }
}

pub fn walk_else<'a, V: Visitor<'a>>(visitor: &mut V, r#else: &'a Else) {
    match r#else {
        Else::If(expr) => visitor.visit_if(&expr),
        Else::Body(expr) => visitor.visit_body(&expr),
    }
}

pub fn walk_expression<'a, V: Visitor<'a>>(visitor: &mut V, expr: &'a Expression) {
    match &expr.kind {
        ExpressionKind::BinopExpr(unary, operator, expr) => {
            visitor.visit_unary(&unary);
            visitor.visit_operator(&operator);
            visitor.visit_expression(&*expr);
        }
        ExpressionKind::UnaryExpr(unary) => visitor.visit_unary(&unary),
        ExpressionKind::StructCtor(ctor) => visitor.visit_struct_ctor(&ctor),
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_identifier(&left);
            visitor.visit_identifier(&right);
            visitor.visit_native_operator(&op);
        }
        ExpressionKind::Return(expr) => {
            visitor.visit_expression(&expr);
        }
    }
}

pub fn walk_struct_ctor<'a, V: Visitor<'a>>(visitor: &mut V, s: &'a StructCtor) {
    visitor.visit_type(&s.name);

    walk_map!(visitor, visit_expression, &s.defs);
}

pub fn walk_unary<'a, V: Visitor<'a>>(visitor: &mut V, unary: &'a UnaryExpr) {
    match unary {
        UnaryExpr::PrimaryExpr(primary) => visitor.visit_primary_expr(primary),
        UnaryExpr::UnaryExpr(op, unary) => {
            visitor.visit_operator(op);
            visitor.visit_unary(&*unary);
        }
    }
}

pub fn walk_primary_expr<'a, V: Visitor<'a>>(visitor: &mut V, primary: &'a PrimaryExpr) {
    visitor.visit_operand(&primary.op);

    if let Some(secondaries) = &primary.secondaries {
        walk_list!(visitor, visit_secondary_expr, secondaries);
    }
}

pub fn walk_secondary_expr<'a, V: Visitor<'a>>(visitor: &mut V, secondary: &'a SecondaryExpr) {
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

pub fn walk_operator<'a, V: Visitor<'a>>(visitor: &mut V, operator: &'a Operator) {
    visitor.visit_identifier(&operator.0)
}

pub fn walk_operand<'a, V: Visitor<'a>>(visitor: &mut V, operand: &'a Operand) {
    match &operand.kind {
        OperandKind::Literal(l) => visitor.visit_literal(&l),
        OperandKind::Identifier(i) => visitor.visit_identifier_path(&i),
        OperandKind::Expression(e) => visitor.visit_expression(&*e),
    }
}

pub fn walk_argument<'a, V: Visitor<'a>>(visitor: &mut V, argument: &'a Argument) {
    visitor.visit_unary(&argument.arg);
}

pub fn walk_literal<'a, V: Visitor<'a>>(visitor: &mut V, literal: &'a Literal) {
    match &literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::Float(f) => visitor.visit_primitive(f),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
        LiteralKind::Array(arr) => visitor.visit_array(arr),
    }
}

pub fn walk_array<'a, V: Visitor<'a>>(visitor: &mut V, arr: &'a Array) {
    walk_list!(visitor, visit_expression, &arr.values);
}

pub fn walk_native_operator<'a, V: Visitor<'a>>(_visitor: &mut V, _operator: &'a NativeOperator) {
    // Nothing to do
}

pub fn walk_type_signature<'a, V: Visitor<'a>>(visitor: &mut V, signature: &'a TypeSignature) {
    walk_list!(visitor, visit_type, &signature.args);

    visitor.visit_type(&signature.ret);
}

pub fn walk_type<'a, V: Visitor<'a>>(_visitor: &mut V, _t: &'a Type) {
    // Nothing to do
}
