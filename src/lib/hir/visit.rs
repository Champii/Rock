use concat_idents::concat_idents;

use crate::hir::*;
use crate::walk_list;

macro_rules! generate_visitor_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {}

            fn visit_primitive<T>(&mut self, _val: T)
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
    FunctionDecl, function_decl
    ArgumentDecl, argument_decl
    IdentifierPath, identifier_path
    Identifier, identifier
    Body, body
    Statement, statement
    Expression, expression
    Literal, literal
    NativeOperator, native_operator
);

pub fn walk_root<'a, V: Visitor<'a>>(visitor: &mut V, root: &'a Root) {
    walk_map!(visitor, visit_top_level, &root.top_levels);
    walk_map!(visitor, visit_body, &root.bodies);
}

pub fn walk_top_level<'a, V: Visitor<'a>>(visitor: &mut V, top_level: &'a TopLevel) {
    match &top_level.kind {
        TopLevelKind::Function(f) => visitor.visit_function_decl(&f),
    };
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

pub fn walk_body<'a, V: Visitor<'a>>(visitor: &mut V, body: &'a Body) {
    visitor.visit_statement(&body.stmt);
}

pub fn walk_statement<'a, V: Visitor<'a>>(visitor: &mut V, statement: &'a Statement) {
    match statement.kind.as_ref() {
        StatementKind::Expression(expr) => visitor.visit_expression(&expr),
    }
}

pub fn walk_expression<'a, V: Visitor<'a>>(visitor: &mut V, expr: &'a Expression) {
    match &*expr.kind {
        ExpressionKind::Lit(lit) => visitor.visit_literal(&lit),
        ExpressionKind::Identifier(id) => visitor.visit_identifier_path(&id),
        ExpressionKind::FunctionCall(op, args) => {
            visitor.visit_expression(&op);
            walk_list!(visitor, visit_expression, args);
        }
        ExpressionKind::NativeOperation(op, left, right) => {
            visitor.visit_native_operator(&op);
            visitor.visit_identifier(&left);
            visitor.visit_identifier(&right);
        }
    }
}

pub fn walk_literal<'a, V: Visitor<'a>>(visitor: &mut V, literal: &'a Literal) {
    match &literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
    }
}

pub fn walk_native_operator<'a, V: Visitor<'a>>(_visitor: &mut V, _operator: &'a NativeOperator) {
    // Nothing to do
}
