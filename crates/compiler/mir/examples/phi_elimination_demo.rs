//! Example demonstrating phi-node elimination in MIR
//!
//! Run with: cargo run --example phi_elimination_demo

use cairo_m_compiler_mir::{
    Instruction, InstructionKind, MirFunction, MirType, PrettyPrint, Terminator, Value,
};

fn main() {
    // Create a simple function with conditional logic that requires phi nodes
    let mut function = create_max_function();

    println!("=== Original MIR with Phi Nodes ===\n");
    println!("{}", function.pretty_print(0));

    println!("\n=== Running Phi Elimination Pass ===\n");

    // Run just the phi elimination pass
    let mut phi_elim = cairo_m_compiler_mir::passes::phi_elimination::PhiElimination::new();
    let modified = cairo_m_compiler_mir::MirPass::run(&mut phi_elim, &mut function);

    if modified {
        println!("✓ Phi nodes successfully eliminated");
    } else {
        println!("No phi nodes to eliminate");
    }

    println!("\n=== MIR After Phi Elimination ===\n");
    println!("{}", function.pretty_print(0));

    // Count instructions
    let total_instructions: usize = function
        .basic_blocks
        .iter()
        .map(|block| block.instructions.len())
        .sum();

    println!("\n=== Statistics ===");
    println!("Blocks: {}", function.basic_blocks.len());
    println!("Instructions: {}", total_instructions);
    println!("Phi nodes remaining: {}", count_phi_nodes(&function));
}

/// Create a function that computes max(a, b) using phi nodes
fn create_max_function() -> MirFunction {
    let mut function = MirFunction::new("max".to_string());

    // Parameters: a and b
    let a = function.new_value_id();
    let b = function.new_value_id();
    function.parameters.push(a);
    function.parameters.push(b);

    // Create blocks
    let entry = function.add_basic_block();
    let then_block = function.add_basic_block();
    let else_block = function.add_basic_block();
    let merge = function.add_basic_block();

    function.entry_block = entry;

    // Entry: compare a > b
    let cmp = function.new_value_id();
    function.basic_blocks[entry]
        .instructions
        .push(Instruction::binary_op(
            cairo_m_compiler_mir::BinaryOp::Greater,
            cmp,
            Value::Operand(a),
            Value::Operand(b),
        ));
    function.basic_blocks[entry].terminator = Terminator::If {
        condition: Value::Operand(cmp),
        then_target: then_block,
        else_target: else_block,
    };
    function.connect(entry, then_block);
    function.connect(entry, else_block);

    // Then block: a is greater, select a
    let max_a = function.new_value_id();
    function.basic_blocks[then_block]
        .instructions
        .push(Instruction::assign(max_a, Value::Operand(a), MirType::Felt));
    function.basic_blocks[then_block].terminator = Terminator::Jump { target: merge };
    function.connect(then_block, merge);

    // Else block: b is greater or equal, select b
    let max_b = function.new_value_id();
    function.basic_blocks[else_block]
        .instructions
        .push(Instruction::assign(max_b, Value::Operand(b), MirType::Felt));
    function.basic_blocks[else_block].terminator = Terminator::Jump { target: merge };
    function.connect(else_block, merge);

    // Merge block: phi node to select the maximum
    let result = function.new_value_id();
    function.basic_blocks[merge]
        .instructions
        .push(Instruction::phi(
            result,
            MirType::Felt,
            vec![
                (then_block, Value::Operand(max_a)),
                (else_block, Value::Operand(max_b)),
            ],
        ));
    function.basic_blocks[merge].terminator = Terminator::Return {
        values: vec![Value::Operand(result)],
    };

    function
}

/// Count phi nodes in a function
fn count_phi_nodes(function: &MirFunction) -> usize {
    function
        .basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter(|instr| matches!(instr.kind, InstructionKind::Phi { .. }))
        .count()
}
