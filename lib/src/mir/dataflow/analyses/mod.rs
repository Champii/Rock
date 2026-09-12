//! Dataflow analyses for borrow checking.

pub mod init;
pub mod loans;

pub use init::{InitError, InitMap, InitState, InitializationAnalysis, MoveInfo};
pub use loans::{LoanAnalysis, LoanData, LoanKind, LoanState, LoanTable};
