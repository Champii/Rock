#[macro_use]
pub mod ast_print;

pub mod tree;
pub mod visit;
pub mod visit_mut;

pub use tree::*;

// TODO: Make it the same way as HirId and FnBodyId ?
pub type NodeId = u64;
