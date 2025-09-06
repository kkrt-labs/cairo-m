use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Value, ValueId};
use stwo_prover::core::fields::m31::M31;
use wasmparser::Operator as Op;

impl DagToMir {
    /// Compute the Cairo-M memory base from a WASM address value.
    /// cm_base = heap_start - (wasm_address / 2)
    fn compute_cm_base_from_wasm_address(
        &self,
        context: &mut DagToMirContext,
        wasm_address: Value,
    ) -> Result<ValueId, DagToMirError> {
        // temp1 = wasm_address / 2
        let temp1 = context.mir_function.new_typed_value_id(MirType::U32);
        let instruction1 =
            Instruction::binary_op(BinaryOp::Div, temp1, wasm_address, Value::integer(2));
        context.get_current_block()?.push_instruction(instruction1);
        // temp2 = temp1 as felt
        let temp2 = context.mir_function.new_typed_value_id(MirType::Felt);
        let instruction2 =
            Instruction::cast(temp2, Value::operand(temp1), MirType::U32, MirType::Felt);
        context.get_current_block()?.push_instruction(instruction2);
        // cm_base = heap_start - temp2
        let cm_base = context.mir_function.new_typed_value_id(MirType::Felt);
        let instruction3 = Instruction::binary_op(
            BinaryOp::Sub,
            cm_base,
            Value::integer(self.heap_start),
            Value::operand(temp2),
        );
        context.get_current_block()?.push_instruction(instruction3);
        Ok(cm_base)
    }

    /// Convert a WASM i32 memory offset (in bytes) to Cairo-M offset (in felts)
    /// Using: cm_offset = -(wasm_offset / 2) - 1 (moved the -1 from base to offset)
    fn cm_offset_from_wasm_i32_offset(&self, wasm_offset: u64) -> Value {
        let half: i64 = (wasm_offset / 2) as i64;
        let cm_off: i64 = -half - 1;
        Value::integer(M31::from(cm_off as i32).0)
    }

    /// Materialize a felt-typed value id for a given immediate/address value.
    fn materialize_felt_value(
        &self,
        context: &mut DagToMirContext,
        value: Value,
    ) -> Result<ValueId, DagToMirError> {
        let id = context.mir_function.new_typed_value_id(MirType::Felt);
        let assign = Instruction::assign(id, value, MirType::Felt);
        context.get_current_block()?.push_instruction(assign);
        Ok(id)
    }

    /// Handle memory load/store operations
    pub(crate) fn handle_memory_operations(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        inputs: &[Value],
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        match wasm_op {
            // Retrieve the value of a global variable (only u32 supported for now)
            Op::GlobalGet { global_index } => {
                let idx = (*global_index) as usize;
                let global_address = Value::integer(self.global_addresses[&idx]);
                let ty = self.global_types[&idx].clone(); // should always be u32
                let result_id = context.mir_function.new_typed_value_id(ty.clone());
                // Materialize base address
                let base_id = self.materialize_felt_value(context, global_address)?;

                let get_instruction =
                    Instruction::load(result_id, Value::operand(base_id), Value::integer(0), ty);
                context
                    .get_current_block()?
                    .push_instruction(get_instruction);
                Ok(Some(result_id))
            }

            // Store a value in a global variable (only u32 supported for now)
            Op::GlobalSet { global_index } => {
                let idx = (*global_index) as usize;
                let global_address = Value::integer(self.global_addresses[&idx]);
                let ty = self.global_types[&idx].clone();
                // Materialize base address
                let base_id = self.materialize_felt_value(context, global_address)?;
                let instruction =
                    Instruction::store(Value::operand(base_id), Value::integer(0), inputs[0], ty);
                context.get_current_block()?.push_instruction(instruction);
                Ok(None)
            }

            // Load I32 from memory
            // The conversion from wasm address to MIR address is :
            // cm_address = heap_start - (wasm_address / 2) - 1
            // cm_offset = -(wasm_offset / 2)
            // Where the 1/2 factor comes from the size conversion u32 = 4 bytes = 2 felts
            // To save an opcode we move the -1 from base_address to offset which is equivalent
            Op::I32Load { memarg, .. } => {
                let base_address = inputs[0];
                let cm_base = self.compute_cm_base_from_wasm_address(context, base_address)?;
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let instruction = Instruction::load(
                    result_id,
                    Value::operand(cm_base),
                    self.cm_offset_from_wasm_i32_offset(memarg.offset),
                    MirType::U32,
                );
                context.get_current_block()?.push_instruction(instruction);
                Ok(Some(result_id))
            }

            // Store I32 in memory
            // The conversion from wasm address to MIR address is :
            // cm_address = heap_start - (wasm_address / 2) - 1
            // cm_offset = -(wasm_offset / 2)
            // Where the 1/2 factor comes from the size conversion u32 = 4 bytes = 2 felts
            // To save an opcode we move the -1 from base_address to offset which is equivalent
            Op::I32Store { memarg, .. } => {
                let base_address = inputs[0];
                let cm_base = self.compute_cm_base_from_wasm_address(context, base_address)?;
                let instruction = Instruction::store(
                    Value::operand(cm_base),
                    self.cm_offset_from_wasm_i32_offset(memarg.offset),
                    inputs[1],
                    MirType::U32,
                );
                context.get_current_block()?.push_instruction(instruction);
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
