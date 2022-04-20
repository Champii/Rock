use crate::ast::tree::*;

pub struct ReturnInserter<'a> {
    pub body: &'a Body,
}

// TODO: Implement `Visitor` trait for `ReturnInserter`
impl<'a> ReturnInserter<'a> {
    pub fn run(&mut self) -> Body {
        let mut body = self.body.clone();

        self.visit_body(&mut body);

        body
    }

    fn visit_body(&mut self, body: &mut Body) {
        if let Some(stmt) = body.stmts.iter_mut().last() {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &mut Statement) {
        let mut is_assign = None;

        match *stmt {
            Statement::Expression(ref mut e) => {
                *e = Box::new(Expression::Return(e.clone()));
            }
            Statement::If(ref mut i) => {
                self.visit_if(i);
            }
            Statement::Assign(ref mut a) => {
                is_assign = Some(a.value.clone());
            }
            Statement::For(ref mut _fa) => {
                unimplemented!("Cannot have loop in return position");
            }
        }

        // FIXME: do this only if a.name is Identifier
        // or else return the ident
        if let Some(value) = is_assign {
            *stmt = Statement::Expression(Box::new(Expression::Return(Box::new(value))));
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
