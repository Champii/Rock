use concat_idents::concat_idents;

use crate::{ast::Type, hir::*};
use crate::{ast::TypeSignature, walk_list};

macro_rules! generate_visitor_mut_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        pub trait VisitorMut<'ast>: Sized {
            fn visit_name(&mut self, _name: &mut String) {}

            fn visit_primitive<T>(&mut self, _val: T)
            {}

            $(
                concat_idents!(fn_name = visit_, $method {
                    fn fn_name(&mut self, $method: &'ast mut $name) {
                        concat_idents!(fn2_name = walk_, $method {
                            fn2_name(self, $method);
                        });
                    }
                });
            )*
        }
    };
}

generate_visitor_mut_trait!(
    Root, root
    TopLevel, top_level
    Trait, r#trait
    Impl, r#impl
    Prototype, prototype
    FunctionDecl, function_decl
    Assign, assign
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
    Literal, literal
    NativeOperator, native_operator
    Type, r#type
    TypeSignature, type_signature
);

pub fn walk_root<'a, V: VisitorMut<'a>>(visitor: &mut V, root: &'a mut Root) {
    walk_map!(visitor, visit_top_level, &mut root.top_levels);

    for (_, r#trait) in &mut root.traits {
        visitor.visit_trait(r#trait);
    }

    for (_, impls) in &mut root.trait_methods {
        walk_map!(visitor, visit_function_decl, impls);
    }

    walk_map!(visitor, visit_fn_body, &mut root.bodies);
}

pub fn walk_top_level<'a, V: VisitorMut<'a>>(visitor: &mut V, top_level: &'a mut TopLevel) {
    match &mut top_level.kind {
        TopLevelKind::Prototype(p) => visitor.visit_prototype(p),
        TopLevelKind::Function(f) => visitor.visit_function_decl(f),
    };
}

pub fn walk_trait<'a, V: VisitorMut<'a>>(visitor: &mut V, t: &'a mut Trait) {
    visitor.visit_type(&mut t.name);

    walk_list!(visitor, visit_type, &mut t.types);

    walk_list!(visitor, visit_prototype, &mut t.defs);
}

#[allow(dead_code)]
pub fn walk_impl<'a, V: VisitorMut<'a>>(visitor: &mut V, i: &'a mut Impl) {
    visitor.visit_type(&mut i.name);

    walk_list!(visitor, visit_type, &mut i.types);

    walk_list!(visitor, visit_function_decl, &mut i.defs);
}

pub fn walk_prototype<'a, V: VisitorMut<'a>>(visitor: &mut V, prototype: &'a mut Prototype) {
    visitor.visit_identifier(&mut prototype.name);

    visitor.visit_type_signature(&mut prototype.signature);
}

pub fn walk_function_decl<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    function_decl: &'a mut FunctionDecl,
) {
    visitor.visit_identifier(&mut function_decl.name);

    walk_list!(visitor, visit_argument_decl, &mut function_decl.arguments);
}

pub fn walk_identifier_path<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    identifier: &'a mut IdentifierPath,
) {
    walk_list!(visitor, visit_identifier, &mut identifier.path);
}

pub fn walk_identifier<'a, V: VisitorMut<'a>>(visitor: &mut V, identifier: &'a mut Identifier) {
    visitor.visit_name(&mut identifier.name);
}

pub fn walk_argument_decl<'a, V: VisitorMut<'a>>(visitor: &mut V, argument: &'a mut ArgumentDecl) {
    visitor.visit_identifier(&mut argument.name);
}

pub fn walk_fn_body<'a, V: VisitorMut<'a>>(visitor: &mut V, fn_body: &'a mut FnBody) {
    visitor.visit_body(&mut fn_body.body);
}

pub fn walk_body<'a, V: VisitorMut<'a>>(visitor: &mut V, body: &'a mut Body) {
    walk_list!(visitor, visit_statement, &mut body.stmts);
}

pub fn walk_statement<'a, V: VisitorMut<'a>>(visitor: &mut V, statement: &'a mut Statement) {
    match &mut *statement.kind {
        StatementKind::Expression(expr) => visitor.visit_expression(expr),
        StatementKind::Assign(assign) => visitor.visit_assign(assign),
        StatementKind::If(expr) => visitor.visit_if(expr),
    }
}

pub fn walk_assign<'a, V: VisitorMut<'a>>(visitor: &mut V, assign: &'a mut Assign) {
    visitor.visit_identifier(&mut assign.name);
    visitor.visit_expression(&mut assign.value);
}
pub fn walk_expression<'a, V: VisitorMut<'a>>(visitor: &mut V, expr: &'a mut Expression) {
    match &mut *expr.kind {
        ExpressionKind::Lit(lit) => visitor.visit_literal(lit),
        ExpressionKind::Identifier(id) => visitor.visit_identifier_path(id),
        ExpressionKind::FunctionCall(fc) => visitor.visit_function_call(fc),
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_native_operator(op);
            visitor.visit_identifier(left);
            visitor.visit_identifier(right);
        }
        ExpressionKind::Return(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_function_call<'a, V: VisitorMut<'a>>(visitor: &mut V, fc: &'a mut FunctionCall) {
    visitor.visit_expression(&mut fc.op);

    walk_list!(visitor, visit_expression, &mut fc.args);
}

pub fn walk_literal<'a, V: VisitorMut<'a>>(visitor: &mut V, literal: &'a mut Literal) {
    match &mut literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::Float(f) => visitor.visit_primitive(f),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
    }
}

pub fn walk_native_operator<'a, V: VisitorMut<'a>>(
    _visitor: &mut V,
    _operator: &'a mut NativeOperator,
) {
    //
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

pub fn walk_type<'a, V: VisitorMut<'a>>(_visitor: &mut V, _t: &'a mut Type) {
    // Nothing to do
}

pub fn walk_type_signature<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    signature: &'a mut TypeSignature,
) {
    walk_list!(visitor, visit_type, &mut signature.args);

    visitor.visit_type(&mut signature.ret);
}
