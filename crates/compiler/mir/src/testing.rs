//! # Testing Utilities for MIR
//!
//! This module provides testing utilities and helpers for working with MIR
//! in unit tests and integration tests.

use crate::instruction::CalleeSignature;
use crate::{
    BasicBlockId, BinaryOp, FunctionId, Instruction, MirFunction, MirModule, MirType, Terminator,
    Value, ValueId,
};

/// Builder for creating test MIR modules
pub struct TestMirBuilder {
    module: MirModule,
}

impl TestMirBuilder {
    /// Creates a new test MIR builder
    pub fn new() -> Self {
        Self {
            module: MirModule::new(),
        }
    }

    /// Adds a function to the module and returns a function builder
    pub fn function(&mut self, name: &str) -> TestFunctionBuilder<'_> {
        let function = MirFunction::new(name.to_string());
        TestFunctionBuilder {
            function,
            module: &mut self.module,
        }
    }

    /// Builds the final MIR module
    pub fn build(self) -> MirModule {
        self.module
    }
}

impl Default for TestMirBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test MIR functions
pub struct TestFunctionBuilder<'a> {
    function: MirFunction,
    module: &'a mut MirModule,
}

impl<'a> TestFunctionBuilder<'a> {
    /// Gets a block builder for the entry block
    pub fn block(&mut self) -> TestBlockBuilder<'_> {
        let block_id = self.function.entry_block;
        TestBlockBuilder {
            function: &mut self.function,
            current_block: block_id,
        }
    }

    /// Adds a new basic block and returns a block builder for it
    pub fn new_block(&mut self) -> TestBlockBuilder<'_> {
        let block_id = self.function.add_basic_block();
        TestBlockBuilder {
            function: &mut self.function,
            current_block: block_id,
        }
    }

    /// Sets the entry block
    pub fn entry_block(&mut self, block_id: BasicBlockId) -> &mut Self {
        self.function.entry_block = block_id;
        self
    }

    /// Adds a parameter and returns its ValueId
    pub fn parameter(&mut self) -> ValueId {
        let value_id = self.function.new_value_id();
        self.function.parameters.push(value_id);
        value_id
    }

    /// Finishes building the function and adds it to the module
    pub fn build(self) -> FunctionId {
        self.module.add_function(self.function)
    }
}

/// Builder for creating test basic blocks
pub struct TestBlockBuilder<'a> {
    function: &'a mut MirFunction,
    current_block: BasicBlockId,
}

impl<'a> TestBlockBuilder<'a> {
    /// Adds an assignment instruction
    pub fn assign(&mut self, source: Value) -> ValueId {
        let dest = self.function.new_value_id();
        let instruction = Instruction::assign(dest, source, MirType::felt());
        self.function
            .get_basic_block_mut(self.current_block)
            .unwrap()
            .push_instruction(instruction);
        dest
    }

    /// Adds a binary operation instruction
    pub fn binary_op(&mut self, op: BinaryOp, left: Value, right: Value) -> ValueId {
        let dest = self.function.new_value_id();
        let instruction = Instruction::binary_op(op, dest, left, right);
        self.function
            .get_basic_block_mut(self.current_block)
            .unwrap()
            .push_instruction(instruction);
        dest
    }

    /// Adds a function call instruction
    pub fn call(&mut self, callee: FunctionId, args: Vec<Value>) -> ValueId {
        let dest = self.function.new_value_id();
        // For testing, create a simple signature
        let signature = CalleeSignature {
            param_types: args.iter().map(|_| MirType::Felt).collect(),
            return_types: vec![MirType::Felt],
        };
        let instruction = Instruction::call(vec![dest], callee, args, signature);
        self.function
            .get_basic_block_mut(self.current_block)
            .unwrap()
            .push_instruction(instruction);
        dest
    }

    /// Adds a void function call instruction
    pub fn void_call(&mut self, callee: FunctionId, args: Vec<Value>) {
        // For testing, create a simple signature based on the arguments
        let signature = CalleeSignature {
            param_types: args.iter().map(|_| MirType::Felt).collect(),
            return_types: vec![], // Void call has no returns
        };
        let instruction = Instruction::call(vec![], callee, args, signature);
        self.function
            .get_basic_block_mut(self.current_block)
            .unwrap()
            .push_instruction(instruction);
    }

    /// Sets the terminator for this block
    pub fn terminate(&mut self, terminator: Terminator) {
        self.function
            .get_basic_block_mut(self.current_block)
            .unwrap()
            .set_terminator(terminator);
    }

    /// Sets a jump terminator
    pub fn jump(&mut self, target: BasicBlockId) {
        self.terminate(Terminator::jump(target));
    }

    /// Sets a conditional branch terminator
    pub fn branch(
        &mut self,
        condition: Value,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) {
        self.terminate(Terminator::branch(condition, then_target, else_target));
    }

    /// Sets a return terminator with a value
    pub fn return_value(&mut self, value: Value) {
        // Also set up the function's return_values field if returning an operand
        if let Value::Operand(id) = value {
            if !self.function.return_values.contains(&id) {
                self.function.return_values.push(id);
            }
        }
        self.terminate(Terminator::return_value(value));
    }

    /// Sets a void return terminator
    pub fn return_void(&mut self) {
        self.terminate(Terminator::return_void());
    }

    /// Returns the current block ID
    pub fn block_id(&self) -> BasicBlockId {
        self.current_block
    }
}

/// Convenience functions for creating test values
pub mod values {
    use super::*;

    /// Creates an integer literal value
    pub fn int(value: u32) -> Value {
        Value::integer(value)
    }

    /// Creates a boolean literal value
    pub fn bool(value: bool) -> Value {
        Value::boolean(value)
    }

    /// Creates a unit value
    pub fn unit() -> Value {
        Value::unit()
    }

    /// Creates an operand value
    pub fn operand(id: ValueId) -> Value {
        Value::operand(id)
    }

    /// Creates an error value
    pub fn error() -> Value {
        Value::error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrettyPrint;

    #[test]
    fn test_module_builder() {
        let mut builder = TestMirBuilder::new();

        // Create a simple function: fn add(a, b) { return a + b; }
        let mut func_builder = builder.function("add");
        let mut block_builder = func_builder.block();
        let result = block_builder.binary_op(
            BinaryOp::Add,
            values::operand(ValueId::new(0)),
            values::operand(ValueId::new(1)),
        );
        block_builder.return_value(values::operand(result));
        let func_id = func_builder.build();

        let module = builder.build();

        // Verify the module structure
        assert_eq!(module.function_count(), 1);
        assert_eq!(module.lookup_function("add"), Some(func_id));

        let function = module.get_function(func_id).unwrap();
        assert_eq!(function.name, "add");
        assert_eq!(function.block_count(), 1);

        // Validate the module
        assert!(module.validate().is_ok());
    }

    #[test]
    fn test_function_builder() {
        let mut function = MirFunction::new("test".to_string());

        // Add some parameters
        let param1 = function.new_value_id();
        let param2 = function.new_value_id();
        function.parameters = vec![param1, param2];

        // Use the existing entry block
        let block_id = function.entry_block;

        // Add an instruction
        let result = function.new_value_id();
        let instruction = Instruction::binary_op(
            BinaryOp::Add,
            result,
            Value::operand(param1),
            Value::operand(param2),
        );

        function
            .get_basic_block_mut(block_id)
            .unwrap()
            .push_instruction(instruction);

        // Set up return_values field before setting terminator
        function.return_values = vec![result];

        function
            .get_basic_block_mut(block_id)
            .unwrap()
            .set_terminator(Terminator::return_value(Value::operand(result)));

        // Validate the function
        assert!(function.validate().is_ok());
        assert_eq!(function.block_count(), 1);
        assert_eq!(function.parameters.len(), 2);
    }

    #[test]
    fn test_pretty_printing() {
        let mut builder = TestMirBuilder::new();

        let mut func_builder = builder.function("simple");
        let mut block_builder = func_builder.block();
        let _result = block_builder.assign(values::int(42));
        block_builder.return_void();
        let _func_id = func_builder.build();

        let module = builder.build();
        let pretty = module.pretty_print(0);

        // Just verify it doesn't panic and produces some output
        assert!(!pretty.is_empty());
        assert!(pretty.contains("fn simple"));
        assert!(pretty.contains("42"));
    }

    #[test]
    fn test_value_creation() {
        let int_val = values::int(42);
        assert!(int_val.is_literal());
        assert_eq!(int_val.as_const_integer(), Some(42));

        let bool_val = values::bool(true);
        assert!(bool_val.is_literal());
        assert_eq!(bool_val.as_const_boolean(), Some(true));

        let unit_val = values::unit();
        assert!(unit_val.is_literal());

        let operand_val = values::operand(ValueId::new(5));
        assert!(operand_val.is_operand());
        assert_eq!(operand_val.as_operand(), Some(ValueId::new(5)));

        let error_val = values::error();
        assert!(error_val.is_error());
    }
}
