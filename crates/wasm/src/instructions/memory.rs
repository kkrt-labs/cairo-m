use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Value, ValueId};
use stwo_prover::core::fields::m31::M31;
use wasmparser::Operator as Op;

impl DagToMir {
    /// Handle memory load/store operations
    pub(crate) fn handle_memory_operations(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        match wasm_op {
            Op::I32Load { memarg, .. } => {
                let base_address = inputs[0];
                // temp1 = base_address / 2
                let temp1 = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction1 =
                    Instruction::binary_op(BinaryOp::Div, temp1, base_address, Value::integer(2));
                context.get_current_block()?.push_instruction(instruction1);
                // temp2 = temp1 as felt
                let temp2 = context.mir_function.new_typed_value_id(MirType::Felt);
                let instruction2 =
                    Instruction::cast(temp2, Value::operand(temp1), MirType::U32, MirType::Felt);
                context.get_current_block()?.push_instruction(instruction2);
                // temp3 = - temp2
                let temp3 = context.mir_function.new_typed_value_id(MirType::Felt);
                let instruction3 = Instruction::binary_op(
                    BinaryOp::Sub,
                    temp3,
                    Value::integer(1 << 30),
                    Value::operand(temp2),
                );
                context.get_current_block()?.push_instruction(instruction3);

                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction4 = Instruction::load(
                    result_id,
                    Value::operand(temp3),
                    Value::integer(M31::from(-(memarg.offset as i32) / 4 - 1).0),
                    MirType::U32,
                );
                context.get_current_block()?.push_instruction(instruction4);

                Ok(Some(result_id))
            }

            Op::I32Store { memarg, .. } => {
                let base_address = inputs[0];
                // temp1 = base_address / 2
                let temp1 = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction1 =
                    Instruction::binary_op(BinaryOp::Div, temp1, base_address, Value::integer(2));
                context.get_current_block()?.push_instruction(instruction1);
                // temp2 = temp1 as felt
                let temp2 = context.mir_function.new_typed_value_id(MirType::Felt);
                let instruction2 =
                    Instruction::cast(temp2, Value::operand(temp1), MirType::U32, MirType::Felt);
                context.get_current_block()?.push_instruction(instruction2);
                // temp3 = - temp2
                let temp3 = context.mir_function.new_typed_value_id(MirType::Felt);
                let instruction3 = Instruction::binary_op(
                    BinaryOp::Sub,
                    temp3,
                    Value::integer(1 << 30),
                    Value::operand(temp2),
                );
                context.get_current_block()?.push_instruction(instruction3);
                let instruction4 = Instruction::store(
                    Value::operand(temp3),
                    Value::integer(M31::from(-(memarg.offset as i32 / 4) - 1).0),
                    inputs[1],
                    MirType::U32,
                );
                context.get_current_block()?.push_instruction(instruction4);

                Ok(None)
            }

            _ => Err(DagToMirError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: context.mir_function.name.clone(),
                node_idx,
                suggestion: "This memory operation is not yet implemented".to_string(),
            }),
        }
    }
}
