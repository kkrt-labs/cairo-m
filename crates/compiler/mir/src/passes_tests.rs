use super::*;
use crate::{Instruction, Terminator};

#[test]
fn test_dead_code_elimination() {
    let mut function = MirFunction::new("test_function".to_string());

    // Create some basic blocks - one reachable, one unreachable
    let entry_block = function.entry_block;
    let reachable_block = function.add_basic_block();
    let unreachable_block = function.add_basic_block();

    // Set up the control flow: entry -> reachable, unreachable is orphaned
    function
        .get_basic_block_mut(entry_block)
        .unwrap()
        .set_terminator(Terminator::jump(reachable_block));
    function
        .get_basic_block_mut(reachable_block)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Add an instruction to the unreachable block so we can verify it gets cleared
    function
        .get_basic_block_mut(unreachable_block)
        .unwrap()
        .push_instruction(Instruction::debug(
            "This should be removed".to_string(),
            vec![],
        ));
    function
        .get_basic_block_mut(unreachable_block)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Verify the unreachable block exists and has content before DCE
    assert_eq!(function.basic_blocks.len(), 3);
    assert!(
        !function
            .get_basic_block(unreachable_block)
            .unwrap()
            .instructions
            .is_empty()
    );

    // Run dead code elimination
    let mut dce = DeadCodeElimination::new();
    let modified = dce.run(&mut function);

    // Verify the pass made changes
    assert!(modified);

    // Verify the unreachable block was completely removed (compacted away)
    // The function should now have only entry and reachable blocks (2 total)
    assert_eq!(function.basic_blocks.len(), 2);

    // The entry block should still jump to the (now renumbered) reachable block
    let entry = function.get_basic_block(function.entry_block).unwrap();
    assert!(matches!(entry.terminator, Terminator::Jump { .. }));
}

#[test]
fn test_pass_manager() {
    let mut function = MirFunction::new("test_function".to_string());

    // Set up a function with unreachable code
    let entry_block = function.entry_block;
    let unreachable_block = function.add_basic_block();

    function
        .get_basic_block_mut(entry_block)
        .unwrap()
        .set_terminator(Terminator::return_void());
    function
        .get_basic_block_mut(unreachable_block)
        .unwrap()
        .push_instruction(Instruction::debug("Unreachable".to_string(), vec![]));
    function
        .get_basic_block_mut(unreachable_block)
        .unwrap()
        .set_terminator(Terminator::return_void());

    // Run standard optimization pipeline
    let mut pass_manager = PassManager::standard_pipeline();
    let modified = pass_manager.run(&mut function);

    // Should be modified by DCE
    assert!(modified);

    // Verify unreachable block was completely removed
    // The function should now only have the entry block
    assert_eq!(function.basic_blocks.len(), 1);
}
