#[macro_use]
pub mod ast_print;

pub mod return_placement;
pub mod tree;
pub mod visit;

pub use tree::*;
pub use visit::*;

// TODO: Make it the same way as HirId and FnBodyId ?
pub type NodeId = u64;
