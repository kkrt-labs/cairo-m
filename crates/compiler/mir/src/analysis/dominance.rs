//! # Dominance Analysis
//!
//! This module implements algorithms to compute the dominator tree and dominance frontiers
//! for a control flow graph. These are fundamental analyses required for SSA construction.
//!
//! ## Dominator Tree
//! A node X dominates a node Y if every path from the entry node to Y must pass through X.
//! The immediate dominator of a node is its closest dominator (excluding itself).
//!
//! ## Dominance Frontiers
//! The dominance frontier of a node X is the set of nodes Y such that:
//! - X dominates a predecessor of Y, but
//! - X does not strictly dominate Y

use crate::{BasicBlockId, MirFunction};
use rustc_hash::{FxHashMap, FxHashSet};

/// A dominator tree represented as a mapping from each block to its immediate dominator
pub type DominatorTree = FxHashMap<BasicBlockId, BasicBlockId>;

/// Dominance frontiers represented as a mapping from each block to its frontier set
pub type DominanceFrontiers = FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>>;

/// Computes the dominator tree for a function using the Cooper-Harvey-Kennedy algorithm
///
/// This is an efficient iterative algorithm that computes dominators in O(n²) time
/// in the worst case, but performs much better in practice.
///
/// ## Algorithm
/// 1. Initialize the entry block to dominate itself
/// 2. Initialize all other blocks to be dominated by all blocks
/// 3. Iteratively refine dominators by intersecting predecessor dominators
/// 4. Repeat until a fixed point is reached
pub fn compute_dominator_tree(function: &MirFunction) -> DominatorTree {
    let entry = function.entry_block;
    let mut doms: FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>> = FxHashMap::default();

    // Build predecessor map for efficient traversal
    let predecessors = build_predecessor_map(function);

    // Initialize: entry dominates itself, all others are dominated by all blocks
    let all_blocks: FxHashSet<_> = (0..function.basic_blocks.len())
        .map(BasicBlockId::from_raw)
        .collect();

    for block_id in &all_blocks {
        if *block_id == entry {
            // Entry block only dominates itself
            let mut entry_doms = FxHashSet::default();
            entry_doms.insert(entry);
            doms.insert(entry, entry_doms);
        } else {
            // All other blocks initially dominated by all blocks
            doms.insert(*block_id, all_blocks.clone());
        }
    }

    // Iterate until fixed point
    let mut changed = true;
    while changed {
        changed = false;

        for block_id in &all_blocks {
            if *block_id == entry {
                continue;
            }

            // New dominators = {block} ∪ (∩ dominators of predecessors)
            let mut new_doms = FxHashSet::default();
            new_doms.insert(*block_id);

            if let Some(preds) = predecessors.get(block_id) {
                if !preds.is_empty() {
                    // Start with dominators of first predecessor
                    let mut intersection = doms[&preds[0]].clone();

                    // Intersect with dominators of remaining predecessors
                    for pred in &preds[1..] {
                        let pred_doms = &doms[pred];
                        intersection.retain(|d| pred_doms.contains(d));
                    }

                    // Add intersection to new dominators
                    new_doms.extend(intersection);
                }
            }

            // Check if dominators changed
            if new_doms != doms[block_id] {
                doms.insert(*block_id, new_doms);
                changed = true;
            }
        }
    }

    // Convert to immediate dominator tree
    compute_immediate_dominators(doms, &all_blocks)
}

/// Computes immediate dominators from the full dominator sets
fn compute_immediate_dominators(
    doms: FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>>,
    all_blocks: &FxHashSet<BasicBlockId>,
) -> DominatorTree {
    let mut idom = FxHashMap::default();

    for block_id in all_blocks {
        let block_doms = &doms[block_id];

        // Find immediate dominator: the dominator that doesn't dominate any other dominator
        for candidate in block_doms {
            if *candidate == *block_id {
                continue; // Skip self
            }

            let mut is_immediate = true;
            for other in block_doms {
                if *other == *block_id || *other == *candidate {
                    continue;
                }

                // If candidate dominates other, it's not immediate
                if doms[other].contains(candidate) {
                    is_immediate = false;
                    break;
                }
            }

            if is_immediate {
                idom.insert(*block_id, *candidate);
                break;
            }
        }
    }

    idom
}

/// Computes dominance frontiers using the dominator tree
///
/// The dominance frontier DF(X) of a node X is computed as:
/// For each node Y that X dominates:
///   For each successor Z of Y:
///     If X does not strictly dominate Z, add Z to DF(X)
pub fn compute_dominance_frontiers(
    function: &MirFunction,
    dom_tree: &DominatorTree,
) -> DominanceFrontiers {
    let mut frontiers: DominanceFrontiers = FxHashMap::default();

    // Initialize empty frontiers for all blocks
    for block_id in 0..function.basic_blocks.len() {
        frontiers.insert(BasicBlockId::from_raw(block_id), FxHashSet::default());
    }

    // Build the set of nodes dominated by each node
    let dominates = build_dominates_map(dom_tree);

    // For each block X
    for x in 0..function.basic_blocks.len() {
        let x_id = BasicBlockId::from_raw(x);

        // For each block Y that X dominates (including X itself)
        if let Some(dominated) = dominates.get(&x_id) {
            for y_id in dominated {
                let y_block = &function.basic_blocks[*y_id];

                // For each successor Z of Y
                for z_id in get_successors(&y_block.terminator) {
                    // If X does not strictly dominate Z (i.e., Z is not dominated by X or X == Z)
                    if !strictly_dominates(x_id, z_id, dom_tree) {
                        frontiers.get_mut(&x_id).unwrap().insert(z_id);
                    }
                }
            }
        }
    }

    frontiers
}

/// Builds a map from each block to the set of blocks it dominates
fn build_dominates_map(
    dom_tree: &DominatorTree,
) -> FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>> {
    let mut dominates: FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>> = FxHashMap::default();

    // Every block dominates itself
    for block in dom_tree.keys() {
        dominates.entry(*block).or_default().insert(*block);
    }

    // Add transitive dominance relationships
    for (block, idom) in dom_tree {
        let mut current = *idom;

        // Walk up the dominator tree
        while dom_tree.contains_key(&current) {
            dominates.entry(current).or_default().insert(*block);

            if let Some(next) = dom_tree.get(&current) {
                if *next == current {
                    break; // Reached entry block
                }
                current = *next;
            } else {
                break;
            }
        }
    }

    // Entry block dominates itself and all blocks with it as ancestor
    let entry = BasicBlockId::from_raw(0);
    dominates.entry(entry).or_default().insert(entry);
    for block in dom_tree.keys() {
        if *block != entry {
            dominates.entry(entry).or_default().insert(*block);
        }
    }

    dominates
}

/// Checks if X strictly dominates Y (X dominates Y and X != Y)
fn strictly_dominates(x: BasicBlockId, y: BasicBlockId, dom_tree: &DominatorTree) -> bool {
    if x == y {
        return false;
    }

    // Walk up from Y to see if we reach X
    let mut current = y;
    while let Some(idom) = dom_tree.get(&current) {
        if *idom == x {
            return true;
        }
        if *idom == current {
            break; // Reached entry
        }
        current = *idom;
    }

    // X is the entry block and Y is not
    x == BasicBlockId::from_raw(0) && y != BasicBlockId::from_raw(0)
}

/// Builds a map from each block to its predecessors
fn build_predecessor_map(function: &MirFunction) -> FxHashMap<BasicBlockId, Vec<BasicBlockId>> {
    let mut predecessors: FxHashMap<BasicBlockId, Vec<BasicBlockId>> = FxHashMap::default();

    for (block_id, block) in function.basic_blocks.iter_enumerated() {
        for successor in get_successors(&block.terminator) {
            predecessors.entry(successor).or_default().push(block_id);
        }
    }

    predecessors
}

/// Gets the successor blocks from a terminator
fn get_successors(terminator: &crate::Terminator) -> Vec<BasicBlockId> {
    use crate::Terminator;

    match terminator {
        Terminator::Jump { target } => vec![*target],
        Terminator::If {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
        Terminator::BranchCmp {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
        Terminator::Return { .. } | Terminator::Unreachable => vec![],
    }
}
