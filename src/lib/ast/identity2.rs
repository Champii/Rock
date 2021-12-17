use crate::parser::span2::Span;

use crate::ast::NodeId;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Identity {
    pub node_id: NodeId,
    pub span: Span,
}

impl Identity {
    pub fn new(node_id: NodeId, span: Span) -> Self {
        Self { span, node_id }
    }
}
