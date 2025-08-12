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

use crate::{cfg, BasicBlockId, MirFunction};
use rustc_hash::{FxHashMap, FxHashSet};

/// A dominator tree represented as a mapping from each block to its immediate dominator
pub type DominatorTree = FxHashMap<BasicBlockId, BasicBlockId>;

/// Dominance frontiers represented as a mapping from each block to its frontier set
pub type DominanceFrontiers = FxHashMap<BasicBlockId, FxHashSet<BasicBlockId>>;

/// Computes the dominator tree for a function using the Cooper-Harvey-Kennedy algorithm
///
/// This is an efficient iterative algorithm that computes immediate dominators directly
/// in O(n²) time worst case, but typically O(n log n) in practice.
///
/// ## Algorithm
/// 1. Compute blocks in reverse postorder (RPO)
/// 2. Initialize entry block's idom to itself
/// 3. Iterate until convergence, updating idoms using the intersect function
pub fn compute_dominator_tree(function: &MirFunction) -> DominatorTree {
    let entry = function.entry_block;

    // Compute reverse postorder traversal
    let rpo = compute_reverse_postorder(function);
    let mut rpo_number = FxHashMap::default();
    for (i, &block) in rpo.iter().enumerate() {
        rpo_number.insert(block, i);
    }

    // Initialize immediate dominators
    let mut idom = FxHashMap::default();
    idom.insert(entry, entry); // Entry is its own idom

    // Build predecessor map
    let predecessors = cfg::build_predecessor_map(function);

    // Iterate until convergence
    let mut changed = true;
    while changed {
        changed = false;

        // Process blocks in RPO (skip entry)
        for &block in rpo.iter().skip(1) {
            // Find first processed predecessor
            let preds = predecessors.get(&block);
            if preds.is_none() || preds.unwrap().is_empty() {
                continue;
            }

            let preds = preds.unwrap();
            let mut new_idom = None;

            // Find first predecessor that has an idom
            for &pred in preds {
                if idom.contains_key(&pred) {
                    new_idom = Some(pred);
                    break;
                }
            }

            if let Some(mut current_idom) = new_idom {
                // Intersect with remaining predecessors that have idoms
                for &pred in preds {
                    if idom.contains_key(&pred) && pred != current_idom {
                        current_idom = intersect(pred, current_idom, &idom, &rpo_number);
                    }
                }

                // Update if changed
                if idom.get(&block) != Some(&current_idom) {
                    idom.insert(block, current_idom);
                    changed = true;
                }
            }
        }
    }

    // Remove self-loop for entry
    if idom.get(&entry) == Some(&entry) {
        idom.remove(&entry);
    }

    idom
}

/// Cooper's intersect function for finding common dominator
fn intersect(
    mut b1: BasicBlockId,
    mut b2: BasicBlockId,
    idom: &DominatorTree,
    rpo_number: &FxHashMap<BasicBlockId, usize>,
) -> BasicBlockId {
    while b1 != b2 {
        while rpo_number[&b1] > rpo_number[&b2] {
            b1 = idom[&b1];
        }
        while rpo_number[&b2] > rpo_number[&b1] {
            b2 = idom[&b2];
        }
    }
    b1
}

/// Computes blocks in reverse postorder
fn compute_reverse_postorder(function: &MirFunction) -> Vec<BasicBlockId> {
    let mut visited = FxHashSet::default();
    let mut postorder = Vec::new();

    fn dfs(
        block: BasicBlockId,
        function: &MirFunction,
        visited: &mut FxHashSet<BasicBlockId>,
        postorder: &mut Vec<BasicBlockId>,
    ) {
        if !visited.insert(block) {
            return;
        }

        for successor in function.basic_blocks[block].terminator.target_blocks() {
            dfs(successor, function, visited, postorder);
        }

        postorder.push(block);
    }

    dfs(function.entry_block, function, &mut visited, &mut postorder);
    postorder.reverse();
    postorder
}

/// Computes dominance frontiers using the standard algorithm
///
/// The dominance frontier DF(X) of a node X is the set of nodes Y where:
/// - X dominates at least one predecessor of Y
/// - X does not strictly dominate Y
///
/// ## Algorithm
/// For each block B with ≥2 predecessors:
///   For each predecessor P:
///     Walk up from P until we reach idom(B)
///     Add B to DF of each block on the path
pub fn compute_dominance_frontiers(
    function: &MirFunction,
    dom_tree: &DominatorTree,
) -> DominanceFrontiers {
    let mut frontiers: DominanceFrontiers = FxHashMap::default();

    // Initialize empty frontiers for all blocks
    for (block_id, _) in function.basic_blocks.iter_enumerated() {
        frontiers.insert(block_id, FxHashSet::default());
    }

    // Build predecessor map
    let predecessors = cfg::build_predecessor_map(function);

    // For each block with at least 2 predecessors (join points)
    for (block_id, _) in function.basic_blocks.iter_enumerated() {
        if let Some(preds) = predecessors.get(&block_id) {
            if preds.len() >= 2 {
                // For each predecessor
                for &pred in preds {
                    let mut runner = pred;

                    // Walk up dominator tree from pred until we reach idom(block)
                    // Add block to DF of each node on the path (excluding idom(block))
                    let block_idom = dom_tree.get(&block_id);

                    while Some(&runner) != block_idom {
                        frontiers.entry(runner).or_default().insert(block_id);

                        // Move up the dominator tree
                        if let Some(&idom) = dom_tree.get(&runner) {
                            runner = idom;
                        } else {
                            // Reached entry block (no idom)
                            frontiers.entry(runner).or_default().insert(block_id);
                            break;
                        }
                    }
                }
            }
        }
    }

    frontiers
}

// Note: build_predecessor_map and get_successors have been removed
// These functions are now imported from the cfg module
