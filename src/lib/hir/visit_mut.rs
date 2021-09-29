use concat_idents::concat_idents;

use crate::{ast::FuncType, walk_list};
use crate::{ast::Type, hir::*};

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
    StructDecl, struct_decl
    Assign, assign
    AssignLeftSide, assign_left_side
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
    StructCtor, struct_ctor
    Indice, indice
    Dot, dot
    Literal, literal
    Array, array
    NativeOperator, native_operator
    Type, r#type
    FuncType, func_type
);

pub fn walk_root<'a, V: VisitorMut<'a>>(visitor: &mut V, root: &'a mut Root) {
    walk_list!(visitor, visit_top_level, &mut root.top_levels);

    for (_, r#struct) in &mut root.structs {
        visitor.visit_struct_decl(r#struct);
    }

    // for (_, r#trait) in &mut root.traits {
    //     visitor.visit_trait(r#trait);
    // }

    // for (_, impls) in &mut root.trait_methods {
    //     walk_map!(visitor, visit_function_decl, impls);
    // }

    walk_map!(visitor, visit_fn_body, &mut root.bodies);
}

pub fn walk_struct_decl<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructDecl) {
    visitor.visit_type(&mut s.name);

    walk_list!(visitor, visit_prototype, &mut s.defs);
}

pub fn walk_struct_ctor<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructCtor) {
    visitor.visit_type(&mut s.name);

    walk_map!(visitor, visit_expression, &mut s.defs);
}

pub fn walk_top_level<'a, V: VisitorMut<'a>>(visitor: &mut V, top_level: &'a mut TopLevel) {
    match &mut top_level.kind {
        TopLevelKind::Prototype(p) => visitor.visit_prototype(p),
        TopLevelKind::Function(f) => visitor.visit_function_decl(f),
    };
}

#[allow(dead_code)]
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

    visitor.visit_func_type(&mut prototype.signature);
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

pub fn walk_assign_left_side<'a, V: VisitorMut<'a>>(
    visitor: &mut V,
    assign_left: &'a mut AssignLeftSide,
) {
    match assign_left {
        AssignLeftSide::Identifier(id) => visitor.visit_identifier(id),
        AssignLeftSide::Indice(expr) => visitor.visit_indice(expr),
        AssignLeftSide::Dot(expr) => visitor.visit_dot(expr),
    }
}

pub fn walk_assign<'a, V: VisitorMut<'a>>(visitor: &mut V, assign: &'a mut Assign) {
    visitor.visit_assign_left_side(&mut assign.name);
    visitor.visit_expression(&mut assign.value);
}

pub fn walk_expression<'a, V: VisitorMut<'a>>(visitor: &mut V, expr: &'a mut Expression) {
    match &mut *expr.kind {
        ExpressionKind::Lit(lit) => visitor.visit_literal(lit),
        ExpressionKind::Identifier(id) => visitor.visit_identifier_path(id),
        ExpressionKind::FunctionCall(fc) => visitor.visit_function_call(fc),
        ExpressionKind::StructCtor(s) => visitor.visit_struct_ctor(s),
        ExpressionKind::Indice(indice) => visitor.visit_indice(indice),
        ExpressionKind::Dot(dot) => visitor.visit_dot(dot),
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_identifier(left);
            visitor.visit_identifier(right);
            visitor.visit_native_operator(op);
        }
        ExpressionKind::Return(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_function_call<'a, V: VisitorMut<'a>>(visitor: &mut V, fc: &'a mut FunctionCall) {
    visitor.visit_expression(&mut fc.op);

    walk_list!(visitor, visit_expression, &mut fc.args);
}

pub fn walk_indice<'a, V: VisitorMut<'a>>(visitor: &mut V, indice: &'a mut Indice) {
    visitor.visit_expression(&mut indice.op);
    visitor.visit_expression(&mut indice.value);
}

pub fn walk_dot<'a, V: VisitorMut<'a>>(visitor: &mut V, dot: &'a mut Dot) {
    visitor.visit_expression(&mut dot.op);
    visitor.visit_identifier(&mut dot.value);
}

pub fn walk_literal<'a, V: VisitorMut<'a>>(visitor: &mut V, literal: &'a mut Literal) {
    match &mut literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::Float(f) => visitor.visit_primitive(f),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
        LiteralKind::Array(arr) => visitor.visit_array(arr),
    }
}

pub fn walk_array<'a, V: VisitorMut<'a>>(visitor: &mut V, arr: &'a mut Array) {
    walk_list!(visitor, visit_expression, &mut arr.values);
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

pub fn walk_func_type<'a, V: VisitorMut<'a>>(visitor: &mut V, signature: &'a mut FuncType) {
    walk_list!(visitor, visit_type, &mut signature.arguments);

    visitor.visit_type(&mut signature.ret);
}
