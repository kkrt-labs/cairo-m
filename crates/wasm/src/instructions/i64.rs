use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Value, ValueId};
use wasmparser::Operator as Op;

impl DagToMir {
    /// Handle I64 operations using u32 tuple representation
    /// I64 is represented as (low_u32, high_u32)
    pub(crate) fn handle_i64_operations(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        match wasm_op {
            // Bitwise operations - operate independently on low and high parts
            Op::I64And => {
                self.handle_i64_bitwise_op(node_idx, BinaryOp::U32BitwiseAnd, inputs, context)
            }
            Op::I64Or => {
                self.handle_i64_bitwise_op(node_idx, BinaryOp::U32BitwiseOr, inputs, context)
            }
            Op::I64Xor => {
                self.handle_i64_bitwise_op(node_idx, BinaryOp::U32BitwiseXor, inputs, context)
            }

            // Arithmetic operations
            Op::I64Add => {
                self.handle_i64_arithmetic_op(node_idx, BinaryOp::U32Add, inputs, context)
            }
            Op::I64Sub => {
                self.handle_i64_arithmetic_op(node_idx, BinaryOp::U32Sub, inputs, context)
            }

            // Constants
            Op::I64Const { value } => {
                let low_part = (*value as u64) as u32;
                let high_part = ((*value as u64) >> 32) as u32;

                // Create low part
                let low_result = context.mir_function.new_typed_value_id(MirType::U32);
                let low_instruction =
                    Instruction::assign(low_result, Value::integer(low_part), MirType::U32);
                context
                    .get_current_block()?
                    .push_instruction(low_instruction);

                // Create high part
                let high_result = context.mir_function.new_typed_value_id(MirType::U32);
                let high_instruction =
                    Instruction::assign(high_result, Value::integer(high_part), MirType::U32);
                context
                    .get_current_block()?
                    .push_instruction(high_instruction);

                // Create tuple to represent the i64
                let tuple_result = context
                    .mir_function
                    .new_typed_value_id(MirType::Tuple(vec![MirType::U32, MirType::U32]));
                let tuple_instruction = Instruction::make_tuple(
                    tuple_result,
                    vec![Value::operand(low_result), Value::operand(high_result)],
                );
                context
                    .get_current_block()?
                    .push_instruction(tuple_instruction);

                Ok(Some(tuple_result))
            }

            _ => Err(DagToMirError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: context.mir_function.name.clone(),
                node_idx,
                suggestion: "This I64 operation is not yet implemented".to_string(),
            }),
        }
    }

    /// Handle I64 bitwise operations by operating on low and high parts separately
    fn handle_i64_bitwise_op(
        &self,
        _node_idx: usize,
        op: BinaryOp,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        // Extract low and high parts from first operand (inputs[0])
        let left_low = context.mir_function.new_typed_value_id(MirType::U32);
        let left_low_extract =
            Instruction::extract_tuple_element(left_low, inputs[0], 0, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(left_low_extract);

        let left_high = context.mir_function.new_typed_value_id(MirType::U32);
        let left_high_extract =
            Instruction::extract_tuple_element(left_high, inputs[0], 1, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(left_high_extract);

        // Extract low and high parts from second operand (inputs[1])
        let right_low = context.mir_function.new_typed_value_id(MirType::U32);
        let right_low_extract =
            Instruction::extract_tuple_element(right_low, inputs[1], 0, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(right_low_extract);

        let right_high = context.mir_function.new_typed_value_id(MirType::U32);
        let right_high_extract =
            Instruction::extract_tuple_element(right_high, inputs[1], 1, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(right_high_extract);

        // Perform bitwise operation on low parts
        let result_low = context.mir_function.new_typed_value_id(MirType::U32);
        let low_op_instruction = Instruction::binary_op(
            op,
            result_low,
            Value::operand(left_low),
            Value::operand(right_low),
        );
        context
            .get_current_block()?
            .push_instruction(low_op_instruction);

        // Perform bitwise operation on high parts
        let result_high = context.mir_function.new_typed_value_id(MirType::U32);
        let high_op_instruction = Instruction::binary_op(
            op,
            result_high,
            Value::operand(left_high),
            Value::operand(right_high),
        );
        context
            .get_current_block()?
            .push_instruction(high_op_instruction);

        // Create result tuple
        let result_tuple = context
            .mir_function
            .new_typed_value_id(MirType::Tuple(vec![MirType::U32, MirType::U32]));
        let tuple_instruction = Instruction::make_tuple(
            result_tuple,
            vec![Value::operand(result_low), Value::operand(result_high)],
        );
        context
            .get_current_block()?
            .push_instruction(tuple_instruction);

        Ok(Some(result_tuple))
    }

    /// Handle I64 arithmetic operations (add/sub) with proper carry handling
    fn handle_i64_arithmetic_op(
        &self,
        _node_idx: usize,
        op: BinaryOp,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        // Extract low and high parts from first operand (inputs[0])
        let left_low = context.mir_function.new_typed_value_id(MirType::U32);
        let left_low_extract =
            Instruction::extract_tuple_element(left_low, inputs[0], 0, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(left_low_extract);

        let left_high = context.mir_function.new_typed_value_id(MirType::U32);
        let left_high_extract =
            Instruction::extract_tuple_element(left_high, inputs[0], 1, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(left_high_extract);

        // Extract low and high parts from second operand (inputs[1])
        let right_low = context.mir_function.new_typed_value_id(MirType::U32);
        let right_low_extract =
            Instruction::extract_tuple_element(right_low, inputs[1], 0, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(right_low_extract);

        let right_high = context.mir_function.new_typed_value_id(MirType::U32);
        let right_high_extract =
            Instruction::extract_tuple_element(right_high, inputs[1], 1, MirType::U32);
        context
            .get_current_block()?
            .push_instruction(right_high_extract);

        // For addition: result_low = left_low + right_low, carry = overflow
        // For subtraction: result_low = left_low - right_low, borrow = underflow
        let result_low = context.mir_function.new_typed_value_id(MirType::U32);
        let low_op_instruction = Instruction::binary_op(
            op,
            result_low,
            Value::operand(left_low),
            Value::operand(right_low),
        );
        context
            .get_current_block()?
            .push_instruction(low_op_instruction);

        // Calculate carry/borrow by checking if the low operation overflowed/underflowed
        // For addition: carry = (left_low + right_low) < left_low (unsigned overflow)
        // For subtraction: borrow = left_low < right_low (unsigned underflow)
        let carry_borrow = context.mir_function.new_typed_value_id(MirType::U32);
        let carry_instruction = match op {
            BinaryOp::U32Add => {
                // Carry = (left_low + right_low) < left_low
                Instruction::binary_op(
                    BinaryOp::U32Less,
                    carry_borrow,
                    Value::operand(result_low),
                    Value::operand(left_low),
                )
            }
            BinaryOp::U32Sub => {
                // Borrow = left_low < right_low
                Instruction::binary_op(
                    BinaryOp::U32Less,
                    carry_borrow,
                    Value::operand(left_low),
                    Value::operand(right_low),
                )
            }
            _ => {
                return Err(DagToMirError::UnsupportedOperation {
                    op: format!("{:?}", op),
                    function_name: context.mir_function.name.clone(),
                    node_idx: 0,
                    suggestion: "Only U32Add and U32Sub are supported for i64 arithmetic"
                        .to_string(),
                });
            }
        };
        context
            .get_current_block()?
            .push_instruction(carry_instruction);

        // Calculate high part: result_high = left_high op right_high op carry/borrow
        let high_with_carry = context.mir_function.new_typed_value_id(MirType::U32);
        let high_op_instruction = Instruction::binary_op(
            op,
            high_with_carry,
            Value::operand(left_high),
            Value::operand(right_high),
        );
        context
            .get_current_block()?
            .push_instruction(high_op_instruction);

        let result_high = context.mir_function.new_typed_value_id(MirType::U32);
        let final_high_instruction = Instruction::binary_op(
            op,
            result_high,
            Value::operand(high_with_carry),
            Value::operand(carry_borrow),
        );
        context
            .get_current_block()?
            .push_instruction(final_high_instruction);

        // Create result tuple
        let result_tuple = context
            .mir_function
            .new_typed_value_id(MirType::Tuple(vec![MirType::U32, MirType::U32]));
        let tuple_instruction = Instruction::make_tuple(
            result_tuple,
            vec![Value::operand(result_low), Value::operand(result_high)],
        );
        context
            .get_current_block()?
            .push_instruction(tuple_instruction);

        Ok(Some(result_tuple))
    }
}
