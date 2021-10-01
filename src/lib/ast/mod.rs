#[macro_use]
pub mod ast_print;

mod func_type;
mod identity;
mod primitive_type;
pub mod resolve;
pub mod span_collector;
mod struct_type;
mod tree;
mod r#type;
pub mod visit;

pub use func_type::*;
pub use identity::*;
pub use primitive_type::*;
pub use r#type::*;
pub use resolve::resolve;
pub use struct_type::*;
pub use tree::*;
pub use visit::*;
