#[macro_use]
pub mod ast_print;

mod identity;
pub mod resolve;
pub mod span_collector;
mod tree;
pub mod visit;

pub use identity::*;
pub use resolve::resolve;
pub use tree::*;
pub use visit::*;

// TODO: Make it the same way as HirId and FnBodyId ?
pub type NodeId = u64;
