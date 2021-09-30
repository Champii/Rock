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
        if let Some(stmt) = body.stmts.iter_mut().last() {
            self.visit_statement(stmt)
        }
    }

    fn visit_statement(&mut self, stmt: &mut Statement) {
        match *stmt.kind {
            StatementKind::Expression(ref mut e) => {
                e.kind = ExpressionKind::Return(e.clone());
            }
            StatementKind::If(ref mut i) => {
                self.visit_if(i);
            }
            StatementKind::Assign(ref mut _a) => {
                unimplemented!("Assign as return value");
            }
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
