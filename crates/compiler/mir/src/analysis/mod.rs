//! # Analysis Module
//!
//! This module contains various analyses that can be performed on MIR,
//! including dominance analysis for SSA construction.

pub mod dominance;

#[cfg(test)]
mod tests;

pub use dominance::{
    compute_dominance_frontiers, compute_dominator_tree, DominanceFrontiers, DominatorTree,
};
