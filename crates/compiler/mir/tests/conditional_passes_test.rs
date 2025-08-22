//! Tests for conditional optimization pass execution

use cairo_m_compiler_mir::passes::{function_uses_memory, MirPass};
use cairo_m_compiler_mir::*;

#[test]
fn test_function_uses_memory_detection() {
    // Test with memory operations
    let mut mem_function = MirFunction::new("uses_memory".to_string());
    let alloca = mem_function.new_value_id();
    let block_id = mem_function.add_basic_block();
    {
        let block = mem_function.get_basic_block_mut(block_id).unwrap();
        block
            .instructions
            .push(Instruction::frame_alloc(alloca, MirType::felt()));
        block.set_terminator(Terminator::Return { values: vec![] });
    }
    assert!(function_uses_memory(&mem_function));

    // Test without memory operations (only aggregates)
    let mut agg_function = MirFunction::new("uses_aggregates".to_string());
    let tuple_val = agg_function.new_value_id();
    let block_id = agg_function.add_basic_block();
    {
        let block = agg_function.get_basic_block_mut(block_id).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(tuple_val)],
        });
    }
    assert!(!function_uses_memory(&agg_function));
}

#[test]
fn test_conditional_pass_skipping() {
    // Create a pass that tracks if it was run
    struct TrackingPass {
        pub was_run: std::cell::RefCell<bool>,
    }

    impl MirPass for TrackingPass {
        fn run(&mut self, _function: &mut MirFunction) -> bool {
            *self.was_run.borrow_mut() = true;
            false // No modifications
        }

        fn name(&self) -> &'static str {
            "TrackingPass"
        }
    }

    // Test with aggregate-only function
    let mut agg_function = MirFunction::new("aggregates_only".to_string());
    let tuple_val = agg_function.new_value_id();
    let block_id = agg_function.add_basic_block();
    {
        let block = agg_function.get_basic_block_mut(block_id).unwrap();
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::integer(1), Value::integer(2)],
        ));
        block.set_terminator(Terminator::Return {
            values: vec![Value::operand(tuple_val)],
        });
    }

    // Create pass manager with conditional pass
    let tracking_pass = TrackingPass {
        was_run: std::cell::RefCell::new(false),
    };

    // This is a simplified test - in real code, PassManager owns the passes
    // Here we just verify the logic
    assert!(!function_uses_memory(&agg_function));

    // Conditional pass should NOT run for aggregate-only function
    if function_uses_memory(&agg_function) {
        // Would run the pass here
        *tracking_pass.was_run.borrow_mut() = true;
    }
    assert!(!*tracking_pass.was_run.borrow());

    // Test with memory-using function
    let mut mem_function = MirFunction::new("uses_memory".to_string());
    let alloca = mem_function.new_value_id();
    let block_id = mem_function.add_basic_block();
    {
        let block = mem_function.get_basic_block_mut(block_id).unwrap();
        block
            .instructions
            .push(Instruction::frame_alloc(alloca, MirType::felt()));
        block.set_terminator(Terminator::Return { values: vec![] });
    }

    // Conditional pass SHOULD run for memory-using function
    if function_uses_memory(&mem_function) {
        // Would run the pass here
        *tracking_pass.was_run.borrow_mut() = true;
    }
    assert!(*tracking_pass.was_run.borrow());
}

#[test]
fn test_mixed_operations_detection() {
    // Test function with both aggregates and memory
    let mut mixed_function = MirFunction::new("mixed".to_string());

    // Create value IDs first
    let tuple_val = mixed_function.new_value_id();
    let alloca = mixed_function.new_value_id();

    let block_id = mixed_function.add_basic_block();
    {
        let block = mixed_function.get_basic_block_mut(block_id).unwrap();

        // Add aggregate operation
        block.instructions.push(Instruction::make_tuple(
            tuple_val,
            vec![Value::integer(1), Value::integer(2)],
        ));

        // Add memory operation
        block
            .instructions
            .push(Instruction::frame_alloc(alloca, MirType::felt()));

        block.set_terminator(Terminator::Return { values: vec![] });
    }

    // Should detect memory usage even with aggregates present
    assert!(function_uses_memory(&mixed_function));
}

#[test]
fn test_array_operations_trigger_memory_passes() {
    // Arrays should still trigger memory optimization passes
    let mut array_function = MirFunction::new("array_ops".to_string());

    // Create value IDs first
    let array_alloca = array_function.new_value_id();
    let elem_ptr = array_function.new_value_id();

    let block_id = array_function.add_basic_block();
    {
        let block = array_function.get_basic_block_mut(block_id).unwrap();

        // Array allocation uses FrameAlloc
        block.instructions.push(Instruction::frame_alloc(
            array_alloca,
            MirType::Array {
                element_type: Box::new(MirType::felt()),
                size: Some(10),
            },
        ));

        // Array access uses GetElementPtr + Load/Store
        block.instructions.push(Instruction::get_element_ptr(
            elem_ptr,
            Value::operand(array_alloca),
            Value::integer(0),
        ));

        block.set_terminator(Terminator::Return { values: vec![] });
    }

    // Arrays require memory passes
    assert!(function_uses_memory(&array_function));
}

#[test]
fn test_pointer_operations_trigger_memory_passes() {
    // Explicit pointer operations should trigger memory passes
    let mut ptr_function = MirFunction::new("pointer_ops".to_string());

    // Create value IDs first
    let value = ptr_function.new_value_id();
    let addr = ptr_function.new_value_id();

    let block_id = ptr_function.add_basic_block();
    {
        let block = ptr_function.get_basic_block_mut(block_id).unwrap();

        // Taking address
        block
            .instructions
            .push(Instruction::address_of(addr, value));

        block.set_terminator(Terminator::Return { values: vec![] });
    }

    // Address operations require memory passes
    assert!(function_uses_memory(&ptr_function));
}
