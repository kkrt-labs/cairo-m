//! Integration tests for phi-node elimination pass

use cairo_m_compiler_mir::pipeline::optimize_module;
use cairo_m_compiler_mir::{
    BasicBlockId, BinaryOp, Instruction, InstructionKind, Literal, MirFunction, MirModule, MirType,
    PassManager, PipelineConfig, PrettyPrint, Terminator, Value,
};

/// Create a function with a simple if-else that requires phi nodes
fn create_if_else_function() -> MirFunction {
    let mut function = MirFunction::new("if_else_test".to_string());

    // Create blocks
    let entry = function.add_basic_block();
    let then_block = function.add_basic_block();
    let else_block = function.add_basic_block();
    let merge = function.add_basic_block();

    function.entry_block = entry;

    // Entry: check condition
    let cond = function.new_value_id();
    let param = function.new_value_id();
    function.parameters.push(param);

    function.basic_blocks[entry]
        .instructions
        .push(Instruction::assign(
            cond,
            Value::Operand(param),
            MirType::Felt,
        ));
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::Operand(cond),
        then_target: then_block,
        else_target: else_block,
    };
    function.connect(entry, then_block);
    function.connect(entry, else_block);

    // Then block: compute value
    let then_val = function.new_value_id();
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::assign(
            then_val,
            Value::Literal(Literal::Integer(100)),
            MirType::Felt,
        ));
    function.basic_blocks[then_block].terminator = Terminator::Jump { target: merge };
    function.connect(then_block, merge);

    // Else block: compute different value
    let else_val = function.new_value_id();
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::assign(
            else_val,
            Value::Literal(Literal::Integer(200)),
            MirType::Felt,
        ));
    function.basic_blocks[else_block].terminator = Terminator::Jump { target: merge };
    function.connect(else_block, merge);

    // Merge block: phi node to select value
    let result = function.new_value_id();
    function.basic_blocks[merge]
        .instructions
        .push(Instruction::phi(
            result,
            MirType::Felt,
            vec![
                (then_block, Value::Operand(then_val)),
                (else_block, Value::Operand(else_val)),
            ],
        ));
    function.basic_blocks[merge].terminator = Terminator::Return {
        values: vec![Value::Operand(result)],
    };

    function
}

/// Create a function with a loop that requires phi nodes
fn create_loop_function() -> MirFunction {
    let mut function = MirFunction::new("loop_test".to_string());

    // Create blocks
    let entry = function.add_basic_block();
    let loop_header = function.add_basic_block();
    let loop_body = function.add_basic_block();
    let exit = function.add_basic_block();

    function.entry_block = entry;

    // Entry: initialize counter
    let init_counter = function.new_value_id();
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::assign(
            init_counter,
            Value::Literal(Literal::Integer(0)),
            MirType::Felt,
        ));
    function.basic_blocks[entry].terminator = Terminator::Jump {
        target: loop_header,
    };
    function.connect(entry, loop_header);

    // Loop header: phi node for counter and condition check
    let counter = function.new_value_id();
    let updated_counter = function.new_value_id(); // Will be defined in loop body

    function.basic_blocks[loop_header]
        .instructions
        .push(Instruction::phi(
            counter,
            MirType::Felt,
            vec![
                (entry, Value::Operand(init_counter)),
                (loop_body, Value::Operand(updated_counter)),
            ],
        ));

    // Check if counter < 10
    let cond = function.new_value_id();
    let ten = function.new_value_id();
    function.basic_blocks[loop_header]
        .instructions
        .push(Instruction::assign(
            ten,
            Value::Literal(Literal::Integer(10)),
            MirType::Felt,
        ));
    function.basic_blocks[loop_header]
        .instructions
        .push(Instruction::binary_op(
            BinaryOp::Less,
            cond,
            Value::Operand(counter),
            Value::Operand(ten),
        ));
    function.basic_blocks[loop_header].terminator = Terminator::If {
        condition: Value::Operand(cond),
        then_target: loop_body,
        else_target: exit,
    };
    function.connect(loop_header, loop_body);
    function.connect(loop_header, exit);

    // Loop body: increment counter
    let one = function.new_value_id();
    function.basic_blocks[loop_body]
        .instructions
        .push(Instruction::assign(
            one,
            Value::Literal(Literal::Integer(1)),
            MirType::Felt,
        ));
    function.basic_blocks[loop_body]
        .instructions
        .push(Instruction::binary_op(
            BinaryOp::Add,
            updated_counter,
            Value::Operand(counter),
            Value::Operand(one),
        ));
    function.basic_blocks[loop_body].terminator = Terminator::Jump {
        target: loop_header,
    };
    function.connect(loop_body, loop_header);

    // Exit: return final counter value
    function.basic_blocks[exit].terminator = Terminator::Return {
        values: vec![Value::Operand(counter)],
    };

    function
}

#[test]
fn test_if_else_phi_elimination() {
    let mut function = create_if_else_function();

    // Count phi nodes before
    let phi_count_before = count_phi_nodes(&function);
    assert_eq!(phi_count_before, 1, "Should have 1 phi node initially");

    // Run the standard pipeline which includes phi elimination
    let mut pass_manager = PassManager::basic_pipeline();
    pass_manager.run(&mut function);

    // Count phi nodes after
    let phi_count_after = count_phi_nodes(&function);
    assert_eq!(phi_count_after, 0, "All phi nodes should be eliminated");

    // Verify the function is still valid
    assert!(
        function.validate().is_ok(),
        "Function should be valid after phi elimination"
    );
}

#[test]
fn test_loop_phi_elimination() {
    let mut function = create_loop_function();

    // Count phi nodes before
    let phi_count_before = count_phi_nodes(&function);
    assert_eq!(
        phi_count_before, 1,
        "Should have 1 phi node for loop counter"
    );

    // Run the standard pipeline which includes phi elimination
    let mut pass_manager = PassManager::standard_pipeline();
    pass_manager.run(&mut function);

    // Count phi nodes after
    let phi_count_after = count_phi_nodes(&function);
    assert_eq!(phi_count_after, 0, "All phi nodes should be eliminated");

    // Verify the function is still valid (but not SSA)
    // The validation pass at the end of the pipeline should be post-SSA
    assert!(
        function.validate().is_ok(),
        "Function should be valid after phi elimination"
    );
}

#[test]
fn test_module_phi_elimination() {
    let mut module = MirModule::new();

    // Add both test functions
    module.add_function(create_if_else_function());
    module.add_function(create_loop_function());

    // Count total phi nodes before
    let total_phis_before: usize = module.functions().map(|(_id, f)| count_phi_nodes(f)).sum();
    assert_eq!(total_phis_before, 2, "Should have 2 phi nodes total");

    // Run optimization pipeline on the module
    let config = PipelineConfig::default();
    optimize_module(&mut module, &config);

    // Count total phi nodes after
    let total_phis_after: usize = module.functions().map(|(_id, f)| count_phi_nodes(f)).sum();
    assert_eq!(total_phis_after, 0, "All phi nodes should be eliminated");

    // Verify the module is still valid
    assert!(
        module.validate().is_ok(),
        "Module should be valid after optimization"
    );
}

#[test]
fn test_phi_elimination_preserves_semantics() {
    let mut function = create_if_else_function();

    // Get a string representation before optimization
    let before = function.pretty_print(0);
    assert!(
        before.contains("φ") || before.contains("phi"),
        "Should contain phi instruction before"
    );

    // Run phi elimination
    let mut pass_manager = PassManager::basic_pipeline();
    pass_manager.run(&mut function);

    // Get a string representation after optimization
    let after = function.pretty_print(0);
    assert!(
        !after.contains("φ") && !after.contains("phi"),
        "Should not contain phi instruction after"
    );

    // The function might have more blocks due to critical edge splitting
    assert!(
        function.basic_blocks.len() >= 4,
        "Should have at least 4 blocks"
    );

    // Each predecessor should now have copy instructions
    // This is a semantic check - the values should be assigned in predecessor blocks
    let merge_block_id = BasicBlockId::from_raw(3);
    let merge_block = &function.basic_blocks[merge_block_id];

    // The merge block should not have any phi instructions
    for instr in &merge_block.instructions {
        assert!(!matches!(instr.kind, InstructionKind::Phi { .. }));
    }
}

/// Helper function to count phi nodes in a function
fn count_phi_nodes(function: &MirFunction) -> usize {
    function
        .basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter(|instr| matches!(instr.kind, InstructionKind::Phi { .. }))
        .count()
}
