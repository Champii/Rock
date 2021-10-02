use paste::paste;
use std::collections::HashMap;

use crate::ast::visit::*;
use crate::parser::Span;
use crate::{ast::visit::Visitor, ast::*};

#[derive(Debug, Default)]
pub struct SpanCollector {
    list: HashMap<NodeId, Span>,
}

impl SpanCollector {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn take_list(self) -> HashMap<NodeId, Span> {
        self.list
    }

    pub fn insert(&mut self, ident: &Identity) {
        self.list.insert(ident.node_id, ident.span.clone());
    }
}

macro_rules! generate_span_collector {
    ($($expr:ty,)+) => {
        impl<'a> Visitor<'a> for SpanCollector {
            paste! {
                $(
                    fn [<visit_ $expr:snake>](&mut self, node: &'a $expr) {
                        self.insert(&node.identity);

                        [<walk_ $expr:snake>](self, node);
                    }
                )+
            }
        }
    };
}

generate_span_collector!(
    Mod,
    TopLevel,
    Prototype,
    Use,
    FunctionDecl,
    Identifier,
    ArgumentDecl,
    If,
    PrimaryExpr,
    Literal,
    NativeOperator,
);

pub fn collect_spans(root: &Root) -> HashMap<NodeId, Span> {
    let mut span_collector = SpanCollector::new();

    span_collector.visit_root(root);

    span_collector.take_list()
}
