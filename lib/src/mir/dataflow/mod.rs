//! Dataflow analysis framework for MIR
//!
//! Based on rustc's dataflow design with lattice-based analyses.

pub mod analyses;
pub mod bitset;
pub mod engine;
pub mod lattice;

pub use bitset::{BitSet, LocalSet};
pub use engine::{run_fixpoint, Analysis, Results};
pub use lattice::Lattice;
