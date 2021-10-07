use std::collections::HashMap;

use crate::ast::{Expression, ExpressionKind, Identifier, Operand, Statement, UnaryExpr};

pub struct Loop {
    pre_loop: Vec<Statement>,
    predicat: Expression,
    pre_body: Vec<Statement>,
    body: Body,
    post_body: Vec<Statement>,
}

#[derive(Debug)]
pub struct LoopDesugar {}

impl LoopDesugar {
    pub fn new() -> Self {
        LoopDesugar {}
    }

    pub fn desugar(&mut self, for_loop: &For) -> Loop {
        match for_loop {
            For::In(for_in) => self.desugar_for_in(&for_in),
            For::While(while_loop) => self.desugar_while(&while_loop),
        }
    }

    // for item in arr
    //   do item
    //
    // --- becomes
    //
    // let i = 0
    // let arr_len = ~Len arr
    // for i < arr_len
    //   let item = arr[i] // pre_body
    //   do item
    //   i = i + 1 // post_body



    fn desugar_for_in(&mut self, for_in: &ForIn) -> Loop {
        let i_var = Statement::new_assign()

        Loop {
            pre_loop: vec![],
            predicat: (),
            pre_body: vec![],
            body: for_in.body.clone(),
            post_body: vec![],
        }
    }

    fn desugar_while(&mut self, while_loop: &While) -> Loop {
        Loop {
            pre_loop: vec![],
            predicat: while_loop.predicat.clone(),
            pre_body: vec![],
            body: while_loop.body.clone(),
            post_body: vec![],
        }
    }
}
