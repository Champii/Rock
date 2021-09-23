use concat_idents::concat_idents;

use crate::{ast::Type, hir::*};
use crate::{ast::TypeSignature, walk_list};

macro_rules! generate_visitor_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {}

            fn visit_primitive<T: std::fmt::Debug + std::fmt::Display>(&mut self, _val: T)
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
    TopLevel, top_level
    Trait, r#trait
    Impl, r#impl
    Assign, assign
    AssignLeftSide, assign_left_side
    Prototype, prototype
    FunctionDecl, function_decl
    ArgumentDecl, argument_decl
    IdentifierPath, identifier_path
    Identifier, identifier
    FnBody, fn_body
    Body, body
    Statement, statement
    Expression, expression
    If, r#if
    Else, r#else
    FunctionCall, function_call
    Indice, indice
    Literal, literal
    Array, array
    NativeOperator, native_operator
    Type, r#type
    TypeSignature, type_signature
);

pub fn walk_root<'a, V: Visitor<'a>>(visitor: &mut V, root: &'a Root) {
    walk_list!(visitor, visit_top_level, &root.top_levels);

    for (_, r#trait) in &root.traits {
        visitor.visit_trait(r#trait);
    }

    for (_, impls) in &root.trait_methods {
        walk_map!(visitor, visit_function_decl, impls);
    }

    walk_map!(visitor, visit_fn_body, &root.bodies);
}

pub fn walk_top_level<'a, V: Visitor<'a>>(visitor: &mut V, top_level: &'a TopLevel) {
    match &top_level.kind {
        TopLevelKind::Prototype(p) => visitor.visit_prototype(&p),
        TopLevelKind::Function(f) => visitor.visit_function_decl(&f),
    };
}

pub fn walk_trait<'a, V: Visitor<'a>>(visitor: &mut V, t: &'a Trait) {
    visitor.visit_type(&t.name);

    walk_list!(visitor, visit_type, &t.types);

    walk_list!(visitor, visit_prototype, &t.defs);
}

#[allow(dead_code)]
pub fn walk_impl<'a, V: Visitor<'a>>(visitor: &mut V, i: &'a Impl) {
    visitor.visit_type(&i.name);

    walk_list!(visitor, visit_type, &i.types);

    walk_list!(visitor, visit_function_decl, &i.defs);
}

pub fn walk_prototype<'a, V: Visitor<'a>>(visitor: &mut V, prototype: &'a Prototype) {
    visitor.visit_identifier(&prototype.name);

    visitor.visit_type_signature(&prototype.signature);
}

pub fn walk_function_decl<'a, V: Visitor<'a>>(visitor: &mut V, function_decl: &'a FunctionDecl) {
    visitor.visit_identifier(&function_decl.name);

    walk_list!(visitor, visit_argument_decl, &function_decl.arguments);
}

pub fn walk_identifier_path<'a, V: Visitor<'a>>(visitor: &mut V, identifier: &'a IdentifierPath) {
    walk_list!(visitor, visit_identifier, &identifier.path);
}

pub fn walk_identifier<'a, V: Visitor<'a>>(visitor: &mut V, identifier: &'a Identifier) {
    visitor.visit_name(identifier.name.clone());
}

pub fn walk_argument_decl<'a, V: Visitor<'a>>(visitor: &mut V, argument: &'a ArgumentDecl) {
    visitor.visit_identifier(&argument.name);
}

pub fn walk_fn_body<'a, V: Visitor<'a>>(visitor: &mut V, fn_body: &'a FnBody) {
    visitor.visit_body(&fn_body.body);
}

pub fn walk_body<'a, V: Visitor<'a>>(visitor: &mut V, body: &'a Body) {
    walk_list!(visitor, visit_statement, &body.stmts);
}

pub fn walk_assign_left_side<'a, V: Visitor<'a>>(visitor: &mut V, assign_left: &'a AssignLeftSide) {
    match assign_left {
        AssignLeftSide::Identifier(id) => visitor.visit_identifier(id),
        AssignLeftSide::Indice(expr) => visitor.visit_indice(expr),
    }
}

pub fn walk_assign<'a, V: Visitor<'a>>(visitor: &mut V, assign: &'a Assign) {
    visitor.visit_assign_left_side(&assign.name);
    visitor.visit_expression(&assign.value);
}

pub fn walk_statement<'a, V: Visitor<'a>>(visitor: &mut V, statement: &'a Statement) {
    match statement.kind.as_ref() {
        StatementKind::Expression(expr) => visitor.visit_expression(&expr),
        StatementKind::Assign(assign) => visitor.visit_assign(&assign),
        StatementKind::If(expr) => visitor.visit_if(&expr),
    }
}

pub fn walk_expression<'a, V: Visitor<'a>>(visitor: &mut V, expr: &'a Expression) {
    match &*expr.kind {
        ExpressionKind::Lit(lit) => visitor.visit_literal(&lit),
        ExpressionKind::Identifier(id) => visitor.visit_identifier_path(&id),
        ExpressionKind::FunctionCall(fc) => visitor.visit_function_call(&fc),
        ExpressionKind::Indice(indice) => visitor.visit_indice(indice),
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_native_operator(&op);
            visitor.visit_identifier(&left);
            visitor.visit_identifier(&right);
        }
        ExpressionKind::Return(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_function_call<'a, V: Visitor<'a>>(visitor: &mut V, fc: &'a FunctionCall) {
    visitor.visit_expression(&fc.op);

    walk_list!(visitor, visit_expression, &fc.args);
}

pub fn walk_indice<'a, V: Visitor<'a>>(visitor: &mut V, indice: &'a Indice) {
    visitor.visit_expression(&indice.op);
    visitor.visit_expression(&indice.value);
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

pub fn walk_native_operator<'a, V: Visitor<'a>>(visitor: &mut V, operator: &'a NativeOperator) {
    visitor.visit_primitive(operator.kind.clone());
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

pub fn walk_type<'a, V: Visitor<'a>>(_visitor: &mut V, _t: &'a Type) {
    // Nothing to do
}

pub fn walk_type_signature<'a, V: Visitor<'a>>(visitor: &mut V, signature: &'a TypeSignature) {
    walk_list!(visitor, visit_type, &signature.args);

    visitor.visit_type(&signature.ret);
}
