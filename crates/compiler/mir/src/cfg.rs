//! # Control Flow Graph Utilities
//!
//! This module provides common utilities for working with control flow graphs,
//! including critical edge detection and splitting, predecessor/successor queries,
//! and other CFG transformations.

use crate::{BasicBlock, BasicBlockId, MirFunction, Terminator};
use rustc_hash::FxHashMap;

/// Get all successor blocks of a given block
pub(crate) fn get_successors(function: &MirFunction, block_id: BasicBlockId) -> Vec<BasicBlockId> {
    if let Some(block) = function.basic_blocks.get(block_id) {
        block.terminator.target_blocks()
    } else {
        vec![]
    }
}

/// Get all predecessor blocks of a given block
pub(crate) fn get_predecessors(
    function: &MirFunction,
    target_id: BasicBlockId,
) -> Vec<BasicBlockId> {
    let block = function
        .basic_blocks
        .get(target_id)
        .unwrap_or_else(|| panic!("Block {:?} not found", target_id));
    block.preds.clone()
}

/// Check if an edge is critical
///
/// A critical edge is an edge from a block with multiple successors to a block
/// with multiple predecessors. These edges need to be split to ensure correct
/// phi node elimination and other transformations.
pub(crate) fn is_critical_edge(
    function: &MirFunction,
    pred_id: BasicBlockId,
    succ_id: BasicBlockId,
) -> bool {
    // Check how many successors the predecessor has
    let pred_successors = get_successors(function, pred_id);
    if pred_successors.len() > 1 {
        // Check how many predecessors the successor has
        let succ_predecessors = get_predecessors(function, succ_id);
        if succ_predecessors.len() > 1 {
            return true;
        }
    }
    false
}

/// Split a critical edge by inserting a new block between predecessor and successor
///
/// Returns the ID of the newly created edge block.
///
/// ## Example
/// Before:
/// ```text
///   Pred (has multiple successors)
///    |  \
///    |   Other
///    |  /
///   Succ (has multiple predecessors)
/// ```
///
/// After:
/// ```text
///   Pred
///    |  \
///  Edge  Other
///    |  /
///   Succ
/// ```
pub(crate) fn split_critical_edge(
    function: &mut MirFunction,
    pred_id: BasicBlockId,
    succ_id: BasicBlockId,
) -> BasicBlockId {
    // Create a new edge block that simply jumps to the successor
    let edge_block = BasicBlock {
        name: Some(format!("edge_{:?}_{:?}", pred_id, succ_id)),
        instructions: Vec::new(),
        terminator: Terminator::Jump { target: succ_id },
        preds: Vec::new(),
        sealed: false,
        filled: false,
    };
    let edge_block_id = function.basic_blocks.push(edge_block);

    // Update edges using new infrastructure
    function.replace_edge(pred_id, succ_id, edge_block_id);
    function.connect(edge_block_id, succ_id);

    // Update the predecessor's terminator to point to the edge block
    // instead of directly to the successor
    if let Some(pred_block) = function.basic_blocks.get_mut(pred_id) {
        pred_block.terminator.replace_target(succ_id, edge_block_id);
    }

    edge_block_id
}

/// Split all critical edges in a function
///
/// Returns a map from (predecessor, successor) pairs to the newly created edge blocks.
pub(crate) fn split_all_critical_edges(
    function: &mut MirFunction,
) -> FxHashMap<(BasicBlockId, BasicBlockId), BasicBlockId> {
    let mut edge_splits = FxHashMap::default();
    let mut critical_edges = Vec::new();

    // First, identify all critical edges
    for (pred_id, pred_block) in function.basic_blocks.iter_enumerated() {
        for succ_id in pred_block.terminator.target_blocks() {
            if is_critical_edge(function, pred_id, succ_id) {
                critical_edges.push((pred_id, succ_id));
            }
        }
    }

    // Then split them (doing this in two phases avoids iterator invalidation)
    for (pred_id, succ_id) in critical_edges {
        let edge_block_id = split_critical_edge(function, pred_id, succ_id);
        edge_splits.insert((pred_id, succ_id), edge_block_id);
    }

    edge_splits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MirFunction, Terminator, Value};

    fn create_diamond_cfg() -> MirFunction {
        let mut function = MirFunction::new("test_diamond".to_string());

        // Create 4 blocks: Entry, Left, Right, Merge
        for _ in 0..4 {
            function.basic_blocks.push(BasicBlock::new());
        }

        let entry = BasicBlockId::from_raw(0);
        let left = BasicBlockId::from_raw(1);
        let right = BasicBlockId::from_raw(2);
        let merge = BasicBlockId::from_raw(3);

        // Entry branches to Left or Right
        let cond = function.new_value_id();
        function.basic_blocks[entry].terminator = Terminator::If {
            condition: Value::operand(cond),
            then_target: left,
            else_target: right,
        };
        // Set up predecessor relationships for entry's successors
        function.connect(entry, left);
        function.connect(entry, right);

        // Left -> Merge
        function.basic_blocks[left].terminator = Terminator::Jump { target: merge };
        function.connect(left, merge);

        // Right -> Merge
        function.basic_blocks[right].terminator = Terminator::Jump { target: merge };
        function.connect(right, merge);

        // Merge returns
        function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

        function
    }

    #[test]
    fn test_get_successors() {
        let function = create_diamond_cfg();

        let entry_succs = get_successors(&function, BasicBlockId::from_raw(0));
        assert_eq!(entry_succs.len(), 2);
        assert!(entry_succs.contains(&BasicBlockId::from_raw(1)));
        assert!(entry_succs.contains(&BasicBlockId::from_raw(2)));

        let left_succs = get_successors(&function, BasicBlockId::from_raw(1));
        assert_eq!(left_succs, vec![BasicBlockId::from_raw(3)]);

        let merge_succs = get_successors(&function, BasicBlockId::from_raw(3));
        assert_eq!(merge_succs.len(), 0);
    }

    #[test]
    fn test_get_predecessors() {
        let function = create_diamond_cfg();

        let entry_preds = get_predecessors(&function, BasicBlockId::from_raw(0));
        assert_eq!(entry_preds.len(), 0);

        let left_preds = get_predecessors(&function, BasicBlockId::from_raw(1));
        assert_eq!(left_preds, vec![BasicBlockId::from_raw(0)]);

        let merge_preds = get_predecessors(&function, BasicBlockId::from_raw(3));
        assert_eq!(merge_preds.len(), 2);
        assert!(merge_preds.contains(&BasicBlockId::from_raw(1)));
        assert!(merge_preds.contains(&BasicBlockId::from_raw(2)));
    }

    #[test]
    fn test_critical_edge_detection() {
        let function = create_diamond_cfg();

        // No critical edges in a simple diamond
        assert!(!is_critical_edge(
            &function,
            BasicBlockId::from_raw(0),
            BasicBlockId::from_raw(1)
        ));
        assert!(!is_critical_edge(
            &function,
            BasicBlockId::from_raw(0),
            BasicBlockId::from_raw(2)
        ));
        assert!(!is_critical_edge(
            &function,
            BasicBlockId::from_raw(1),
            BasicBlockId::from_raw(3)
        ));
        assert!(!is_critical_edge(
            &function,
            BasicBlockId::from_raw(2),
            BasicBlockId::from_raw(3)
        ));
    }

    #[test]
    fn test_critical_edge_splitting() {
        let mut function = MirFunction::new("test_critical".to_string());

        // Create a CFG with critical edges
        for _ in 0..4 {
            function.basic_blocks.push(BasicBlock::new());
        }

        let entry = BasicBlockId::from_raw(0);
        let b1 = BasicBlockId::from_raw(1);
        let b2 = BasicBlockId::from_raw(2);
        let merge = BasicBlockId::from_raw(3);

        // Entry branches to B1 or Merge (critical edge: Entry->Merge)
        let cond1 = function.new_value_id();
        function.basic_blocks[entry].terminator = Terminator::If {
            condition: Value::operand(cond1),
            then_target: b1,
            else_target: merge,
        };
        function.connect(entry, b1);
        function.connect(entry, merge);

        // B1 branches to B2 or Merge (critical edge: B1->Merge)
        let cond2 = function.new_value_id();
        function.basic_blocks[b1].terminator = Terminator::If {
            condition: Value::operand(cond2),
            then_target: b2,
            else_target: merge,
        };
        function.connect(b1, b2);
        function.connect(b1, merge);

        // B2 -> Merge
        function.basic_blocks[b2].terminator = Terminator::Jump { target: merge };
        function.connect(b2, merge);

        // Merge returns
        function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

        // Entry->Merge and B1->Merge are critical edges
        assert!(is_critical_edge(&function, entry, merge));
        assert!(is_critical_edge(&function, b1, merge));

        // Split the critical edge from Entry to Merge
        let edge_block = split_critical_edge(&mut function, entry, merge);

        // Verify the edge was split correctly
        assert!(!is_critical_edge(&function, entry, edge_block));
        assert_eq!(get_successors(&function, edge_block), vec![merge]);

        // The Entry->Merge edge should now go through the edge block
        let entry_succs = get_successors(&function, entry);
        assert!(entry_succs.contains(&edge_block));
        assert!(!entry_succs.contains(&merge));
    }
}
