use paste::paste;

use crate::{hir::*, ty::*};

macro_rules! generate_visitor_mut_trait {
    ($(
        $name:ident
    )+) => {
        pub trait VisitorMut<'hir>: Sized {
            fn visit_name(&mut self, _name: &mut String) {}

            fn visit_primitive<T>(&mut self, _val: T)
            {}

            paste! {
                $(
                    fn [<visit_ $name:snake>](&mut self, node: &'hir mut $name) {
                        [<walk_ $name:snake>](self, node);
                    }
                )+
            }
        }
    };
}

generate_visitor_mut_trait!(
    Root
    TopLevel
    Trait
    Impl
    Prototype
    FunctionDecl
    StructDecl
    Assign
    AssignLeftSide
    ArgumentDecl
    IdentifierPath
    Identifier
    FnBody
    Body
    Statement
    For
    ForIn
    While
    Expression
    IfChain
    If
    FunctionCall
    StructCtor
    Indice
    Dot
    Literal
    Array
    NativeOperator
    Type
    FuncType
);

pub fn walk_root<'a, V: VisitorMut<'a>>(visitor: &mut V, root: &'a mut Root) {
    walk_list!(visitor, visit_top_level, &mut root.top_levels);

    for r#struct in &mut root.structs.values_mut() {
        visitor.visit_struct_decl(r#struct);
    }

    for (_, r#trait) in &mut root.traits {
        visitor.visit_trait(r#trait);
    }

    for (_, impls) in &mut root.trait_methods {
        walk_map!(visitor, visit_function_decl, impls);
    }

    for (_, impls) in &mut root.struct_methods {
        walk_map!(visitor, visit_function_decl, impls);
    }

    walk_map!(visitor, visit_fn_body, &mut root.bodies);
}

pub fn walk_struct_decl<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructDecl) {
    visitor.visit_identifier(&mut s.name);

    walk_list!(visitor, visit_prototype, &mut s.defs);
}

pub fn walk_struct_ctor<'a, V: VisitorMut<'a>>(visitor: &mut V, s: &'a mut StructCtor) {
    visitor.visit_identifier(&mut s.name);

    walk_map!(visitor, visit_expression, &mut s.defs);
}

pub fn walk_top_level<'a, V: VisitorMut<'a>>(visitor: &mut V, top_level: &'a mut TopLevel) {
    match &mut top_level.kind {
        TopLevelKind::Extern(p) => visitor.visit_prototype(p),
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
        StatementKind::If(expr) => visitor.visit_if_chain(expr),
        StatementKind::For(for_loop) => visitor.visit_for(for_loop),
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
    //
}

pub fn walk_if_chain<'a, V: VisitorMut<'a>>(visitor: &mut V, if_chain: &'a mut IfChain) {
    walk_list!(visitor, visit_if, &mut if_chain.ifs);

    if let Some(body) = &mut if_chain.else_body {
        visitor.visit_body(body);
    }
}

pub fn walk_if<'a, V: VisitorMut<'a>>(visitor: &mut V, r#if: &'a mut If) {
    visitor.visit_expression(&mut r#if.predicat);
    visitor.visit_body(&mut r#if.body);
}

pub fn walk_type<'a, V: VisitorMut<'a>>(_visitor: &mut V, _t: &'a mut Type) {
    // Nothing to do
}

pub fn walk_func_type<'a, V: VisitorMut<'a>>(visitor: &mut V, signature: &'a mut FuncType) {
    walk_list!(visitor, visit_type, &mut signature.arguments);

    visitor.visit_type(&mut signature.ret);
}
