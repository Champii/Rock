use crate::ast::*;

pub struct ReturnInserter<'a> {
    pub body: &'a Body,
}

impl<'a> ReturnInserter<'a> {
    pub fn run(&mut self) -> Body {
        let mut body = self.body.clone();

        self.visit_body(&mut body);

        body.clone()
    }

    fn visit_body(&mut self, body: &mut Body) {
        body.stmts
            .iter_mut()
            .last()
            .map(|stmt| self.visit_statement(stmt));
    }

    fn visit_statement(&mut self, stmt: &mut Statement) {
        match *stmt.kind {
            StatementKind::Expression(ref mut e) => {
                e.kind = ExpressionKind::Return(Box::new(e.clone()));
            }
            StatementKind::If(ref mut i) => {
                self.visit_if(i);
            }
        }
    }
    fn visit_if(&mut self, r#if: &mut If) {
        self.visit_body(&mut r#if.body);

        if let Some(ref mut r#else) = r#if.else_.as_mut() {
            self.visit_else(r#else);
        } else {
            // r#if.else_ = Some(Else::Body(Statement {
            //     kind: Box::new(StatementKind::Expression(Expression::new_literal(
            //         Literal {
            //             kind: LiteralKind::Number(0),
            //         },
            //     ))),
            // }));
        }
    }

    fn visit_else(&mut self, r#else: &mut Else) {
        match r#else {
            Else::If(i) => self.visit_if(i),
            Else::Body(b) => self.visit_body(b),
        }
    }
}
