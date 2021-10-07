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
        // let mut is_assign = None;

        if let Some(stmt) = body.stmts.iter_mut().last() {
            // if let StatementKind::Assign(ref mut a) = *stmt.kind.clone() {
            //     is_assign = Some(a.clone());
            // }

            self.visit_statement(stmt);
        }

        // if let Some(a) = is_assign {
        //     body.stmts.push(Statement {
        //         kind: Box::new(StatementKind::Expression(Box::new(Expression {
        //             kind: ExpressionKind::Return(Box::new(a.name.as_expression())),
        //         }))),
        //     });
        // }
    }

    fn visit_statement(&mut self, stmt: &mut Statement) {
        let mut is_assign = None;

        match *stmt.kind {
            StatementKind::Expression(ref mut e) => {
                e.kind = ExpressionKind::Return(e.clone());
            }
            StatementKind::If(ref mut i) => {
                self.visit_if(i);
            }
            StatementKind::Assign(ref mut a) => {
                is_assign = Some(a.value.clone());
            }
            StatementKind::For(ref mut _fa) => {
                unimplemented!("Cannot have loop in return position");
            }
        }

        // FIXME: do this only if a.name is Identifier
        // or else return the ident
        if let Some(value) = is_assign {
            stmt.kind = Box::new(StatementKind::Expression(Box::new(Expression {
                kind: ExpressionKind::Return(Box::new(value)),
            })));
        }
    }

    fn visit_if(&mut self, r#if: &mut If) {
        self.visit_body(&mut r#if.body);

        if let Some(ref mut r#else) = r#if.else_.as_mut() {
            self.visit_else(r#else);
        } else {
            unimplemented!("Else is not found");
        }
    }

    fn visit_else(&mut self, r#else: &mut Else) {
        match r#else {
            Else::If(i) => self.visit_if(i),
            Else::Body(b) => self.visit_body(b),
        }
    }
}
