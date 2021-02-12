use concat_idents::concat_idents;

use crate::ast::*;

macro_rules! generate_visitor_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        pub trait Visitor<'ast>: Sized {
            fn visit_name(&mut self, _name: String) {
                // Nothing
            }

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
    FunctionDecl, function_decl
    Identifier, identifier
    IdentifierPath, identifier_path
    ArgumentDecl, argument_decl
    Body, body
    Statement, statement
    Expression, expression
    If, r#if
    UnaryExpr, unary
    Operator, operator
    PrimaryExpr, primary
    SecondaryExpr, secondary
    Operand, operand
    Argument, argument
    Literal, literal
);

pub fn walk_root<'a, V: Visitor<'a>>(visitor: &mut V, root: &'a Root) {
    visitor.visit_mod(&root.r#mod);
}

pub fn walk_mod<'a, V: Visitor<'a>>(visitor: &mut V, _mod: &'a Mod) {
    walk_list!(visitor, visit_top_level, &_mod.top_levels);
}

pub fn walk_top_level<'a, V: Visitor<'a>>(visitor: &mut V, top_level: &'a TopLevel) {
    match &top_level.kind {
        TopLevelKind::Mod(name, m) => {
            visitor.visit_identifier(&name);
            visitor.visit_mod(&m);
        }
        TopLevelKind::Function(f) => visitor.visit_function_decl(&f),
    };
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
    visitor.visit_statement(&body.stmt);
}

pub fn walk_statement<'a, V: Visitor<'a>>(visitor: &mut V, statement: &'a Statement) {
    match statement.kind.as_ref() {
        StatementKind::If(i) => visitor.visit_if(&i),
        StatementKind::Expression(expr) => visitor.visit_expression(&expr),
    }
}

pub fn walk_if<'a, V: Visitor<'a>>(_visitor: &mut V, _if: &'a If) {
    // TODO
    // visitor.visit_(body.stmt);
}

pub fn walk_expression<'a, V: Visitor<'a>>(visitor: &mut V, expr: &'a Expression) {
    match &expr.kind {
        ExpressionKind::BinopExpr(unary, operator, expr) => {
            visitor.visit_unary(&unary);
            visitor.visit_operator(&operator);
            visitor.visit_expression(&*expr);
        }
        ExpressionKind::UnaryExpr(unary) => visitor.visit_unary(&unary),
    }
}

pub fn walk_unary<'a, V: Visitor<'a>>(visitor: &mut V, unary: &'a UnaryExpr) {
    match unary {
        UnaryExpr::PrimaryExpr(primary) => visitor.visit_primary(primary),
        UnaryExpr::UnaryExpr(op, unary) => {
            visitor.visit_operator(op);
            visitor.visit_unary(&*unary);
        }
    }
}

pub fn walk_primary<'a, V: Visitor<'a>>(visitor: &mut V, primary: &'a PrimaryExpr) {
    match primary {
        PrimaryExpr::PrimaryExpr(operand, secondaries) => {
            visitor.visit_operand(operand);
            walk_list!(visitor, visit_secondary, secondaries);
        }
    }
}

pub fn walk_secondary<'a, V: Visitor<'a>>(visitor: &mut V, secondary: &'a SecondaryExpr) {
    match secondary {
        SecondaryExpr::Arguments(args) => {
            walk_list!(visitor, visit_argument, args);
        }
    }
}

pub fn walk_operator<'a, V: Visitor<'a>>(_visitor: &mut V, _operator: &'a Operator) {
    // Nothing to do
}

pub fn walk_operand<'a, V: Visitor<'a>>(visitor: &mut V, operand: &'a Operand) {
    match &operand.kind {
        OperandKind::Literal(l) => visitor.visit_literal(&l),
        OperandKind::Identifier(i) => visitor.visit_identifier_path(&i),
        OperandKind::Expression(e) => visitor.visit_expression(&*e),
    }
}

pub fn walk_argument<'a, V: Visitor<'a>>(visitor: &mut V, argument: &'a Argument) {
    visitor.visit_expression(&argument.arg);
}

pub fn walk_literal<'a, V: Visitor<'a>>(visitor: &mut V, literal: &'a Literal) {
    match &literal.kind {
        LiteralKind::Number(n) => visitor.visit_primitive(n),
        LiteralKind::String(s) => visitor.visit_primitive(s),
        LiteralKind::Bool(b) => visitor.visit_primitive(b),
    }
}
