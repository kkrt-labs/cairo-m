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

/// Helper to create an irreducible CFG (multiple entry points to a loop)
///     Entry
///     /  \
///   B1    B2
///     \  / \
///      B3--B4
fn create_irreducible_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_irreducible".to_string());

    // Create 5 blocks
    for _ in 0..5 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let b3 = BasicBlockId::from_raw(3);
    let b4 = BasicBlockId::from_raw(4);

    // Entry branches to B1 or B2
    let cond1 = function.new_value_id();
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::operand(cond1),
        then_target: b1,
        else_target: b2,
    };

    // B1 -> B3
    function.basic_blocks[b1].terminator = Terminator::Jump { target: b3 };

    // B2 branches to B3 or B4
    let cond2 = function.new_value_id();
    function.basic_blocks[b2].terminator = Terminator::If {
        condition: Value::operand(cond2),
        then_target: b3,
        else_target: b4,
    };

    // B3 -> B4 (creates a cycle with B4)
    function.basic_blocks[b3].terminator = Terminator::Jump { target: b4 };

    // B4 -> B3 (back edge creating irreducible loop)
    function.basic_blocks[b4].terminator = Terminator::Jump { target: b3 };

    function
}

#[test]
fn test_irreducible_cfg() {
    let function = create_irreducible_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    let entry = BasicBlockId::from_raw(0);
    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let b3 = BasicBlockId::from_raw(3);
    let b4 = BasicBlockId::from_raw(4);

    // Entry dominates B1 and B2
    assert_eq!(dom_tree[&b1], entry);
    assert_eq!(dom_tree[&b2], entry);

    // Entry dominates B3 and B4 (not B1 or B2 since there are multiple paths)
    assert_eq!(dom_tree[&b3], entry);
    assert_eq!(dom_tree[&b4], entry);

    // B3 and B4 form an irreducible loop - they are in each other's DF
    assert!(frontiers[&b3].contains(&b4));
    assert!(frontiers[&b4].contains(&b3));

    // B3 is a join point (from B1 and B2)
    assert!(frontiers[&b1].contains(&b3) || frontiers[&b2].contains(&b3));
}

/// Helper to create a complex nested loop CFG
///     Entry
///       |
///     Outer <----
///     /  \      |
///   Inner Exit  |
///     |\ /      |
///     | X       |
///     |/ \      |
///   Back--+-----
fn create_nested_loops_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_nested_loops".to_string());

    // Create 5 blocks: Entry, Outer, Inner, Back, Exit
    for _ in 0..5 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let outer = BasicBlockId::from_raw(1);
    let inner = BasicBlockId::from_raw(2);
    let back = BasicBlockId::from_raw(3);
    let exit = BasicBlockId::from_raw(4);

    // Entry -> Outer
    function.basic_blocks[entry].terminator = Terminator::Jump { target: outer };

    // Outer branches to Inner or Exit
    let cond1 = function.new_value_id();
    function.basic_blocks[outer].terminator = Terminator::If {
        condition: Value::operand(cond1),
        then_target: inner,
        else_target: exit,
    };

    // Inner branches to Back or Exit
    let cond2 = function.new_value_id();
    function.basic_blocks[inner].terminator = Terminator::If {
        condition: Value::operand(cond2),
        then_target: back,
        else_target: exit,
    };

    // Back branches to Inner (inner loop) or Outer (outer loop)
    let cond3 = function.new_value_id();
    function.basic_blocks[back].terminator = Terminator::If {
        condition: Value::operand(cond3),
        then_target: inner,
        else_target: outer,
    };

    // Exit returns
    function.basic_blocks[exit].terminator = Terminator::Return { values: vec![] };

    function
}

#[test]
fn test_nested_loops() {
    let function = create_nested_loops_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    let entry = BasicBlockId::from_raw(0);
    let outer = BasicBlockId::from_raw(1);
    let inner = BasicBlockId::from_raw(2);
    let back = BasicBlockId::from_raw(3);
    let exit = BasicBlockId::from_raw(4);

    // Verify dominator tree
    assert_eq!(dom_tree[&outer], entry);
    assert_eq!(dom_tree[&inner], outer);
    assert_eq!(dom_tree[&back], inner);
    assert_eq!(dom_tree[&exit], outer);

    // Outer is in its own DF (loop header for outer loop)
    assert!(frontiers[&outer].contains(&outer));

    // Inner is in its own DF (loop header for inner loop)
    assert!(frontiers[&inner].contains(&inner));

    // Back node should have both loop headers in its DF
    assert!(frontiers[&back].contains(&inner));
    assert!(frontiers[&back].contains(&outer));

    // Exit is a join point from Outer and Inner
    assert!(frontiers[&inner].contains(&exit));
}

/// Helper to create a diamond with critical edge
///     Entry
///       |
///      B1
///     /  \
///   B2    B3
///   |  X  |
///   | / \ |
///   B4   B5
///    \ /
///    Exit
fn create_critical_edge_cfg() -> MirFunction {
    let mut function = MirFunction::new("test_critical_edge".to_string());

    // Create 7 blocks
    for _ in 0..7 {
        function.basic_blocks.push(crate::BasicBlock::new());
    }

    let entry = BasicBlockId::from_raw(0);
    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let b3 = BasicBlockId::from_raw(3);
    let b4 = BasicBlockId::from_raw(4);
    let b5 = BasicBlockId::from_raw(5);
    let exit = BasicBlockId::from_raw(6);

    // Entry -> B1
    function.basic_blocks[entry].terminator = Terminator::Jump { target: b1 };

    // B1 branches to B2 or B3
    let cond1 = function.new_value_id();
    function.basic_blocks[b1].terminator = Terminator::If {
        condition: Value::operand(cond1),
        then_target: b2,
        else_target: b3,
    };

    // B2 branches to B4 or B5 (critical edges to join points)
    let cond2 = function.new_value_id();
    function.basic_blocks[b2].terminator = Terminator::If {
        condition: Value::operand(cond2),
        then_target: b4,
        else_target: b5,
    };

    // B3 branches to B4 or B5 (critical edges to join points)
    let cond3 = function.new_value_id();
    function.basic_blocks[b3].terminator = Terminator::If {
        condition: Value::operand(cond3),
        then_target: b4,
        else_target: b5,
    };

    // B4 -> Exit
    function.basic_blocks[b4].terminator = Terminator::Jump { target: exit };

    // B5 -> Exit
    function.basic_blocks[b5].terminator = Terminator::Jump { target: exit };

    // Exit returns
    function.basic_blocks[exit].terminator = Terminator::Return { values: vec![] };

    function
}

#[test]
fn test_critical_edges() {
    let function = create_critical_edge_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    let b1 = BasicBlockId::from_raw(1);
    let b2 = BasicBlockId::from_raw(2);
    let b3 = BasicBlockId::from_raw(3);
    let b4 = BasicBlockId::from_raw(4);
    let b5 = BasicBlockId::from_raw(5);
    let exit = BasicBlockId::from_raw(6);

    // B4 and B5 are join points with critical edges
    // They should be in the DF of B2 and B3
    assert!(frontiers[&b2].contains(&b4));
    assert!(frontiers[&b2].contains(&b5));
    assert!(frontiers[&b3].contains(&b4));
    assert!(frontiers[&b3].contains(&b5));

    // Exit is a join point from B4 and B5
    assert!(frontiers[&b4].contains(&exit));
    assert!(frontiers[&b5].contains(&exit));

    // Verify dominator relationships
    assert_eq!(dom_tree[&b4], b1);
    assert_eq!(dom_tree[&b5], b1);
    assert_eq!(dom_tree[&exit], b1);
}

#[test]
fn test_phi_placement_correctness() {
    // Test that phi nodes would be placed correctly for a variable defined in multiple branches
    // This mimics what mem2reg does with the DF computation
    let function = create_if_else_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Simulate placing phi nodes for a variable defined in blocks 1 and 2
    let mut phi_blocks = FxHashSet::default();
    let def_blocks = vec![BasicBlockId::from_raw(1), BasicBlockId::from_raw(2)];

    // Standard algorithm: for each def block, add its DF to phi blocks
    for &def_block in &def_blocks {
        for &df_block in &frontiers[&def_block] {
            phi_blocks.insert(df_block);
        }
    }

    // Phi should be placed at the merge block (block 3)
    assert_eq!(phi_blocks.len(), 1);
    assert!(phi_blocks.contains(&BasicBlockId::from_raw(3)));
}

#[test]
fn test_loop_phi_placement() {
    // Test phi placement for loop variables
    let function = create_loop_cfg();
    let dom_tree = compute_dominator_tree(&function);
    let frontiers = compute_dominance_frontiers(&function, &dom_tree);

    // Simulate placing phi nodes for a variable modified in the loop body
    let mut phi_blocks = FxHashSet::default();
    let def_blocks = vec![BasicBlockId::from_raw(2)]; // Body modifies variable

    for &def_block in &def_blocks {
        for &df_block in &frontiers[&def_block] {
            phi_blocks.insert(df_block);
        }
    }

    // Phi should be placed at the loop header (block 1)
    assert!(phi_blocks.contains(&BasicBlockId::from_raw(1)));
}
