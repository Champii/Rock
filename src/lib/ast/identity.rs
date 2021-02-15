use std::sync::atomic::{AtomicU64, Ordering};

use crate::parser::{Span, TokenId};

use crate::infer::NodeId;
use crate::infer::TypeId;

static GLOBAL_NEXT_NODE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Identity {
    pub node_id: NodeId,
    pub token_id: TokenId,
    pub span: Span,
    pub type_id: TypeId,
    pub scope_depth: u8,
}

impl Identity {
    pub fn new(token_id: TokenId, span: Span) -> Self {
        Self {
            span,
            node_id: Self::next_node_id(),
            token_id,
            type_id: 0,
            scope_depth: 0,
        }
    }

    pub fn new_placeholder() -> Self {
        Self {
            span: Span::new_placeholder(),
            node_id: 0,
            token_id: 0,
            type_id: 0,
            scope_depth: 0,
        }
    }

    pub fn next_node_id() -> NodeId {
        GLOBAL_NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst)
    }
}
