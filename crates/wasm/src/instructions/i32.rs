use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Value, ValueId};
use wasmparser::Operator as Op;

impl DagToMir {
    /// Convert a WASM binary operations to the immediately corresponding Mir operation
    fn convert_wasm_binop_to_mir(
        &self,
        wasm_op: &Op,
        left: Value,
        right: Value,
        dest_type: MirType,
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        let result_id = context.mir_function.new_typed_value_id(dest_type);
        let mir_op = match wasm_op {
            Op::I32Add => BinaryOp::U32Add,
            Op::I32Sub => BinaryOp::U32Sub,
            Op::I32Mul => BinaryOp::U32Mul,
            Op::I32DivU => BinaryOp::U32Div,
            Op::I32Eq => BinaryOp::U32Eq,
            Op::I32Ne => BinaryOp::U32Neq,
            Op::I32GtU => BinaryOp::U32Greater,
            Op::I32GeU => BinaryOp::U32GreaterEqual,
            Op::I32LtU => BinaryOp::U32Less,
            Op::I32LeU => BinaryOp::U32LessEqual,
            Op::I32And => BinaryOp::U32BitwiseAnd,
            Op::I32Or => BinaryOp::U32BitwiseOr,
            Op::I32Xor => BinaryOp::U32BitwiseXor,
            _ => unreachable!(),
        };
        let instruction = Instruction::binary_op(mir_op, result_id, left, right);
        context.get_current_block()?.push_instruction(instruction);
        Ok(Some(result_id))
    }

    /// Handle I32 arithmetic and logical operations
    pub(crate) fn handle_i32_operations(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        match wasm_op {
            // U32 Operations which are immediately convertible to MIR instructions
            Op::I32Add
            | Op::I32Sub
            | Op::I32Mul
            | Op::I32DivU
            | Op::I32And
            | Op::I32Or
            | Op::I32Xor => {
                self.convert_wasm_binop_to_mir(wasm_op, inputs[0], inputs[1], MirType::U32, context)
            }

            // For comparisons, we produce a boolean result
            // This is not WASM compliant, but works if these values are only used in conditional branches
            // TODO : cast everything correctly or sync with VM so that comparisons between u32 produce u32 booleans
            Op::I32Eq | Op::I32Ne | Op::I32GtU | Op::I32GeU | Op::I32LtU | Op::I32LeU => self
                .convert_wasm_binop_to_mir(wasm_op, inputs[0], inputs[1], MirType::Bool, context),

            // Signed comparison instructions, constructed by shifting the inputs by 2^31 and then comparing the results with unsigned opcodes
            Op::I32LtS | Op::I32GtS | Op::I32LeS | Op::I32GeS => {
                let temp1 = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction1 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp1,
                    inputs[0],
                    Value::integer(0x80000000),
                );
                let temp2 = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction2 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp2,
                    inputs[1],
                    Value::integer(0x80000000),
                );
                let result_id = context.mir_function.new_typed_value_id(MirType::Bool);
                let op = match wasm_op {
                    Op::I32LtS => BinaryOp::U32Less,
                    Op::I32GtS => BinaryOp::U32Greater,
                    Op::I32LeS => BinaryOp::U32LessEqual,
                    Op::I32GeS => BinaryOp::U32GreaterEqual,
                    _ => unreachable!(),
                };
                let instruction3 = Instruction::binary_op(
                    op,
                    result_id,
                    Value::operand(temp1),
                    Value::operand(temp2),
                );
                context.get_current_block()?.push_instruction(instruction1);
                context.get_current_block()?.push_instruction(instruction2);
                context.get_current_block()?.push_instruction(instruction3);
                Ok(Some(result_id))
            }

            // Zero comparison instruction, constructed by comparing the input to 0
            // TODO : fix type of result_id
            Op::I32Eqz => {
                let result_id = context.mir_function.new_typed_value_id(MirType::Bool);
                let instruction = Instruction::binary_op(
                    BinaryOp::U32Eq,
                    result_id,
                    inputs[0],
                    Value::integer(0),
                );
                context.get_current_block()?.push_instruction(instruction);
                Ok(Some(result_id))
            }

            // Assigning a constant to a variable
            Op::I32Const { value } => {
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::assign(result_id, Value::integer(*value as u32), MirType::U32);
                context.get_current_block()?.push_instruction(instruction);
                Ok(Some(result_id))
            }

            _ => Err(DagToMirError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: context.mir_function.name.clone(),
                node_idx,
                suggestion: "This I32 operation is not yet implemented".to_string(),
            }),
        }
    }
}
