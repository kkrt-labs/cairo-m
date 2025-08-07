//! # Tests for Analysis Module
//!
//! This module contains tests for dominance analysis and other SSA-related analyses.

use super::dominance::{compute_dominance_frontiers, compute_dominator_tree};
use crate::{BasicBlockId, MirFunction, Terminator, Value};
use rustc_hash::FxHashSet;

/// Helper to create a simple linear CFG: Entry -> B1 -> B2 -> Exit
fn create_linear_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_linear".to_string());

    // Create 4 blocks
    for _ in 0..4 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    // Entry (0) -> B1 (1)
    function.basic_blocks[BasicBlockId::from_raw(0)].terminator = Terminator::Jump {
        target: BasicBlockId::from_raw(1),
    };

    // B1 (1) -> B2 (2)
    function.basic_blocks[BasicBlockId::from_raw(1)].terminator = Terminator::Jump {
        target: BasicBlockId::from_raw(2),
    };

    // B2 (2) -> Exit (3)
    function.basic_blocks[BasicBlockId::from_raw(2)].terminator = Terminator::Jump {
        target: BasicBlockId::from_raw(3),
    };

    // Exit (3) returns
    function.basic_blocks[BasicBlockId::from_raw(3)].terminator =
        Terminator::Return { values: vec![] };

    function
}

/// Helper to create an if-else diamond CFG:
///     Entry
///     /  \
///   Then  Else
///     \  /
///     Merge
fn create_if_else_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_if_else".to_string());

    // Create 4 blocks: Entry, Then, Else, Merge
    for _ in 0..4 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let then_block = BasicBlockId::from_raw(1);
    let else_block = BasicBlockId::from_raw(2);
    let merge = BasicBlockId::from_raw(3);

    // Entry branches to Then or Else
    let cond = function.new_value_id();
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::operand(cond),
        then_target: then_block,
        else_target: else_block,
    };

    // Then -> Merge
    function.basic_blocks[then_block].terminator = Terminator::Jump { target: merge };

    // Else -> Merge
    function.basic_blocks[else_block].terminator = Terminator::Jump { target: merge };

    // Merge returns
    function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

    function
}

/// Helper to create a simple loop CFG:
///     Entry
///       |
///     Header <--
///       |      |
///     Body ----
///       |
///     Exit
fn create_loop_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_loop".to_string());

    // Create 4 blocks: Entry, Header, Body, Exit
    for _ in 0..4 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let header = BasicBlockId::from_raw(1);
    let body = BasicBlockId::from_raw(2);
    let exit = BasicBlockId::from_raw(3);

    // Entry -> Header
    function.basic_blocks[entry].terminator = Terminator::Jump { target: header };

    // Header branches to Body or Exit
    let cond = function.new_value_id();
    function.basic_blocks[header].terminator = Terminator::If {
        condition: Value::operand(cond),
        then_target: body,
        else_target: exit,
    };

    // Body -> Header (loop back)
    function.basic_blocks[body].terminator = Terminator::Jump { target: header };

    // Exit returns
    function.basic_blocks[exit].terminator = Terminator::Return { values: vec![] };

    function
}

/// Helper to create a nested if CFG:
///        Entry
///        /  \
///      B1    B2
///     / \     |
///   B3  B4   /
///     \ |  /
///      Merge
fn create_nested_if_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_nested_if".to_string());

    // Create 6 blocks
    for _ in 0..6 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let b3 = BasicBlockId::from_raw(3);
    let b4 = BasicBlockId::from_raw(4);
    let merge = BasicBlockId::from_raw(5);

    // Entry branches to B1 or B2
    let cond1 = function.new_value_id();
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::operand(cond1),
        then_target: b1,
        else_target: b2,
    };

    // B1 branches to B3 or B4
    let cond2 = function.new_value_id();
    function.basic_blocks[b1].terminator = Terminator::If {
        condition: Value::operand(cond2),
        then_target: b3,
        else_target: b4,
    };

    // B2 -> Merge
    function.basic_blocks[b2].terminator = Terminator::Jump { target: merge };

    // B3 -> Merge
    function.basic_blocks[b3].terminator = Terminator::Jump { target: merge };

    // B4 -> Merge
    function.basic_blocks[b4].terminator = Terminator::Jump { target: merge };

    // Merge returns
    function.basic_blocks[merge].terminator = Terminator::Return { values: vec![] };

    function
}

#[test]
fn test_dominator_tree_linear() {
    let function = create_linear_cfg();
    let dom_tree = compute_dominator_tree(&function);

    // In a linear CFG, each block is dominated by its predecessor
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(1)],
        BasicBlockId::from_raw(0)
    );
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(2)],
        BasicBlockId::from_raw(1)
    );
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(3)],
        BasicBlockId::from_raw(2)
    );
}

#[test]
fn test_dominator_tree_if_else() {
    let function = create_if_else_cfg();
    let dom_tree = compute_dominator_tree(&function);

    // Entry dominates all blocks
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(1)],
        BasicBlockId::from_raw(0)
    ); // Then
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(2)],
        BasicBlockId::from_raw(0)
    ); // Else
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(3)],
        BasicBlockId::from_raw(0)
    ); // Merge
}

#[test]
fn test_dominator_tree_loop() {
    let function = create_loop_cfg();
    let dom_tree = compute_dominator_tree(&function);

    // Entry dominates Header
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(1)],
        BasicBlockId::from_raw(0)
    );

    // Header dominates Body and Exit
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(2)],
        BasicBlockId::from_raw(1)
    );
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(3)],
        BasicBlockId::from_raw(1)
    );
}

#[test]
fn test_dominator_tree_nested_if() {
    let function = create_nested_if_cfg();
    let dom_tree = compute_dominator_tree(&function);

    // Entry dominates B1 and B2
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(1)],
        BasicBlockId::from_raw(0)
    );
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(2)],
        BasicBlockId::from_raw(0)
    );

    // B1 dominates B3 and B4
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(3)],
        BasicBlockId::from_raw(1)
    );
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(4)],
        BasicBlockId::from_raw(1)
    );

    // Entry dominates Merge
    assert_eq!(
        dom_tree[&BasicBlockId::from_raw(5)],
        BasicBlockId::from_raw(0)
    );
}

#[test]
fn test_dominance_frontiers_linear() {
    let function = create_linear_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // In a linear CFG, no block has a dominance frontier
    for i in 0..4 {
        assert!(frontiers[&BasicBlockId::from_raw(i)].is_empty());
    }
}

#[test]
fn test_dominance_frontiers_if_else() {
    let function = create_if_else_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Entry has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(0)].is_empty());

    // Then and Else blocks have Merge in their frontier
    // (They dominate predecessors of Merge but not Merge itself)
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(1)],
        FxHashSet::from_iter([BasicBlockId::from_raw(3)])
    );
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(2)],
        FxHashSet::from_iter([BasicBlockId::from_raw(3)])
    );

    // Merge has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(3)].is_empty());
}

#[test]
fn test_dominance_frontiers_loop() {
    let function = create_loop_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Entry has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(0)].is_empty());

    // Header has itself in its frontier (because Body dominates a predecessor of Header)
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(1)],
        FxHashSet::from_iter([BasicBlockId::from_raw(1)])
    );

    // Body has Header in its frontier (loop back edge)
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(2)],
        FxHashSet::from_iter([BasicBlockId::from_raw(1)])
    );

    // Exit has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(3)].is_empty());
}

#[test]
fn test_dominance_frontiers_nested_if() {
    let function = create_nested_if_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Entry has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(0)].is_empty());

    // B1 has Merge in its frontier (because its children B3/B4 dominate predecessors of Merge)
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(1)],
        FxHashSet::from_iter([BasicBlockId::from_raw(5)])
    );

    // B2, B3, and B4 all have Merge in their frontier
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(2)],
        FxHashSet::from_iter([BasicBlockId::from_raw(5)])
    );
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(3)],
        FxHashSet::from_iter([BasicBlockId::from_raw(5)])
    );
    assert_eq!(
        frontiers[&BasicBlockId::from_raw(4)],
        FxHashSet::from_iter([BasicBlockId::from_raw(5)])
    );

    // Merge has no frontier
    assert!(frontiers[&BasicBlockId::from_raw(5)].is_empty());
}

#[test]
fn test_multiple_returns() {
    let mut function = MirFunction::new("test_multiple_returns".to_string());

    // Create 3 blocks: Entry, Return1, Return2
    for _ in 0..3 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let ret1 = BasicBlockId::from_raw(1);
    let ret2 = BasicBlockId::from_raw(2);

    // Entry branches to Return1 or Return2
    let cond = function.new_value_id();
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::operand(cond),
        then_target: ret1,
        else_target: ret2,
    };

    // Both blocks return
    function.basic_blocks[ret1].terminator = Terminator::Return { values: vec![] };
    function.basic_blocks[ret2].terminator = Terminator::Return { values: vec![] };

    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Entry dominates both returns
    assert_eq!(dom_tree[&ret1], entry);
    assert_eq!(dom_tree[&ret2], entry);

    // No block has a dominance frontier (no merge points)
    assert!(frontiers[&entry].is_empty());
    assert!(frontiers[&ret1].is_empty());
    assert!(frontiers[&ret2].is_empty());
}
