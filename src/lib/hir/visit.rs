use paste::paste;

use crate::{hir::*, ty::*};

macro_rules! generate_visitor_trait {
    ($(
        $name:ident
    )+) => {
        pub trait Visitor<'hir>: Sized {
            fn visit_name(&mut self, _name: &str) {}

            fn visit_primitive<T: std::fmt::Debug + std::fmt::Display>(&mut self, _val: T)
            {}

            paste! {
                $(
                    fn [<visit_ $name:snake>](&mut self, node: &'hir $name) {
                        [<walk_ $name:snake>](self, node);
                    }
                )+
            }
        }
    };
}

generate_visitor_trait!(
    Root
    TopLevel
    Trait
    Impl
    Assign
    AssignLeftSide
    Prototype
    FunctionDecl
    StructDecl
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

pub fn walk_root<'a, V: Visitor<'a>>(visitor: &mut V, root: &'a Root) {
    walk_list!(visitor, visit_top_level, &root.top_levels);

    for r#struct in root.structs.values() {
        visitor.visit_struct_decl(r#struct);
    }

    for r#trait in root.traits.values() {
        visitor.visit_trait(r#trait);
    }

    for impls in root.trait_methods.values() {
        walk_map!(visitor, visit_function_decl, impls);
    }

    for impls in root.struct_methods.values() {
        walk_map!(visitor, visit_function_decl, impls);
    }

    walk_map!(visitor, visit_fn_body, &root.bodies);
}

pub fn walk_top_level<'a, V: Visitor<'a>>(visitor: &mut V, top_level: &'a TopLevel) {
    match &top_level.kind {
        TopLevelKind::Prototype(p) => visitor.visit_prototype(p),
        TopLevelKind::Function(f) => visitor.visit_function_decl(f),
    };
}

pub fn walk_struct_decl<'a, V: Visitor<'a>>(visitor: &mut V, s: &'a StructDecl) {
    visitor.visit_identifier(&s.name);

    walk_list!(visitor, visit_prototype, &s.defs);
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

    visitor.visit_func_type(&prototype.signature);
}

pub fn walk_function_decl<'a, V: Visitor<'a>>(visitor: &mut V, function_decl: &'a FunctionDecl) {
    visitor.visit_identifier(&function_decl.name);

    walk_list!(visitor, visit_argument_decl, &function_decl.arguments);
}

pub fn walk_identifier_path<'a, V: Visitor<'a>>(visitor: &mut V, identifier: &'a IdentifierPath) {
    walk_list!(visitor, visit_identifier, &identifier.path);
}

pub fn walk_identifier<'a, V: Visitor<'a>>(visitor: &mut V, identifier: &'a Identifier) {
    visitor.visit_name(&identifier.name);
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
        AssignLeftSide::Dot(expr) => visitor.visit_dot(expr),
    }
}

pub fn walk_assign<'a, V: Visitor<'a>>(visitor: &mut V, assign: &'a Assign) {
    visitor.visit_assign_left_side(&assign.name);
    visitor.visit_expression(&assign.value);
}

pub fn walk_statement<'a, V: Visitor<'a>>(visitor: &mut V, statement: &'a Statement) {
    match statement.kind.as_ref() {
        StatementKind::Expression(expr) => visitor.visit_expression(expr),
        StatementKind::Assign(assign) => visitor.visit_assign(assign),
        StatementKind::If(expr) => visitor.visit_if_chain(expr),
        StatementKind::For(for_loop) => visitor.visit_for(for_loop),
    }
}

pub fn walk_for<'a, V: Visitor<'a>>(visitor: &mut V, for_loop: &'a For) {
    match for_loop {
        For::In(for_in) => visitor.visit_for_in(for_in),
        For::While(while_loop) => visitor.visit_while(while_loop),
    }
}

pub fn walk_for_in<'a, V: Visitor<'a>>(visitor: &mut V, for_in: &'a ForIn) {
    visitor.visit_identifier(&for_in.value);
    visitor.visit_expression(&for_in.expr);
    visitor.visit_body(&for_in.body);
}

pub fn walk_while<'a, V: Visitor<'a>>(visitor: &mut V, while_loop: &'a While) {
    visitor.visit_expression(&while_loop.predicat);
    visitor.visit_body(&while_loop.body);
}

pub fn walk_expression<'a, V: Visitor<'a>>(visitor: &mut V, expr: &'a Expression) {
    match &*expr.kind {
        ExpressionKind::Lit(lit) => visitor.visit_literal(lit),
        ExpressionKind::Identifier(id) => visitor.visit_identifier_path(id),
        ExpressionKind::FunctionCall(fc) => visitor.visit_function_call(fc),
        ExpressionKind::StructCtor(s) => visitor.visit_struct_ctor(s),
        ExpressionKind::Indice(indice) => visitor.visit_indice(indice),
        ExpressionKind::Dot(dot) => visitor.visit_dot(dot),
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_native_operator(op);
            visitor.visit_identifier(left);
            visitor.visit_identifier(right);
        }
        ExpressionKind::Return(expr) => visitor.visit_expression(expr),
    }
}

pub fn walk_struct_ctor<'a, V: Visitor<'a>>(visitor: &mut V, s: &'a StructCtor) {
    visitor.visit_identifier(&s.name);

    walk_map!(visitor, visit_expression, &s.defs);
}

pub fn walk_function_call<'a, V: Visitor<'a>>(visitor: &mut V, fc: &'a FunctionCall) {
    visitor.visit_expression(&fc.op);

    walk_list!(visitor, visit_expression, &fc.args);
}

pub fn walk_dot<'a, V: Visitor<'a>>(visitor: &mut V, dot: &'a Dot) {
    visitor.visit_expression(&dot.op);
    visitor.visit_identifier(&dot.value);
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
        LiteralKind::Char(c) => visitor.visit_primitive(c),
    }
}

pub fn walk_array<'a, V: Visitor<'a>>(visitor: &mut V, arr: &'a Array) {
    walk_list!(visitor, visit_expression, &arr.values);
}

pub fn walk_native_operator<'a, V: Visitor<'a>>(visitor: &mut V, operator: &'a NativeOperator) {
    visitor.visit_primitive(operator.kind.clone());
}

pub fn walk_if_chain<'a, V: Visitor<'a>>(visitor: &mut V, if_chain: &'a IfChain) {
    walk_list!(visitor, visit_if, &if_chain.ifs);

    if let Some(body) = &if_chain.else_body {
        visitor.visit_body(body);
    }
}

pub fn walk_if<'a, V: Visitor<'a>>(visitor: &mut V, r#if: &'a If) {
    visitor.visit_expression(&r#if.predicat);
    visitor.visit_body(&r#if.body);
}

pub fn walk_type<'a, V: Visitor<'a>>(_visitor: &mut V, _t: &'a Type) {
    // Nothing to do
}

pub fn walk_func_type<'a, V: Visitor<'a>>(visitor: &mut V, signature: &'a FuncType) {
    walk_list!(visitor, visit_type, &signature.arguments);

    visitor.visit_type(&signature.ret);
}
