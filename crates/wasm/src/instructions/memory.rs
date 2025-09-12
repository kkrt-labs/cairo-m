use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Value, ValueId};
use stwo_prover::core::fields::m31::M31;
use wasmparser::Operator as Op;

impl DagToMir {
    /// Compute the Cairo-M memory base from a WASM address value.
    /// cm_base = heap_start - (wasm_address / 2)
    /// This is done dynamically using 3 mir instructions, which is pretty inefficient.
    fn compute_cm_base_from_wasm_address(
        &self,
        context: &mut DagToMirContext,
        wasm_address: Value,
    ) -> Result<ValueId, DagToMirError> {
        // temp1 = wasm_address / 2
        let temp1 = context.mir_function.new_typed_value_id(MirType::U32);
        let inst_div_by_2 =
            Instruction::binary_op(BinaryOp::U32Div, temp1, wasm_address, Value::integer(2));

        // temp2 = temp1 as felt
        let temp2 = context.mir_function.new_typed_value_id(MirType::Felt);
        let inst_cast =
            Instruction::cast(temp2, Value::operand(temp1), MirType::U32, MirType::Felt);

        // cm_base = heap_start - temp2
        let cm_base = context.mir_function.new_typed_value_id(MirType::Felt);
        let inst_sub = Instruction::binary_op(
            BinaryOp::Sub,
            cm_base,
            Value::integer(self.heap_start),
            Value::operand(temp2),
        );

        context.get_current_block()?.push_instruction(inst_div_by_2);
        context.get_current_block()?.push_instruction(inst_cast);
        context.get_current_block()?.push_instruction(inst_sub);
        Ok(cm_base)
    }

    /// Convert a WASM i32 memory offset (in bytes) to Cairo-M offset (in felts)
    /// Using: cm_offset = -(wasm_offset / 2) - 1
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

                if let Some(&global_address) = self.global_addresses.get(&idx) {
                    let ty = self.global_types[&idx].clone(); // should always be u32
                    let result_id = context.mir_function.new_typed_value_id(ty.clone());
                    // Materialize base address
                    let base_id =
                        self.materialize_felt_value(context, Value::integer(global_address))?;

                    let get_instruction = Instruction::load(
                        result_id,
                        Value::operand(base_id),
                        Value::integer(0),
                        ty,
                    );
                    context
                        .get_current_block()?
                        .push_instruction(get_instruction);
                    Ok(Some(result_id))
                } else {
                    Err(DagToMirError::GlobalAddressNotFound {
                        global_index: *global_index as usize,
                    })
                }
            }

            // Store a value in a global variable (only u32 supported for now)
            Op::GlobalSet { global_index } => {
                let idx = (*global_index) as usize;
                if let Some(&global_address) = self.global_addresses.get(&idx) {
                    let ty = self.global_types[&idx].clone();
                    // Materialize base address
                    let base_id =
                        self.materialize_felt_value(context, Value::integer(global_address))?;
                    let instruction = Instruction::store(
                        Value::operand(base_id),
                        Value::integer(0),
                        inputs[0],
                        ty,
                    );
                    context.get_current_block()?.push_instruction(instruction);
                    Ok(None)
                } else {
                    Err(DagToMirError::GlobalAddressNotFound {
                        global_index: *global_index as usize,
                    })
                }
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
            // See above for address computation
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cm_offset_from_wasm_i32_offset_even_offsets() {
        // wasm_offset 0 -> half 0 -> cm_offset -1
        let dummy = DagToMir {
            module: crate::loader::BlocklessDagModule::from_file(
                "tests/test_cases/arithmetic.wasm",
            )
            .unwrap(),
            global_addresses: std::collections::HashMap::new(),
            global_types: std::collections::HashMap::new(),
            heap_start: 0,
        };
        let v0 = dummy.cm_offset_from_wasm_i32_offset(0);
        assert_eq!(v0, Value::integer(M31::from(-1).0));

        // wasm_offset 2 -> half 1 -> cm_offset -2
        let v2 = dummy.cm_offset_from_wasm_i32_offset(2);
        assert_eq!(v2, Value::integer(M31::from(-2).0));

        // wasm_offset 4 -> half 2 -> cm_offset -3
        let v4 = dummy.cm_offset_from_wasm_i32_offset(4);
        assert_eq!(v4, Value::integer(M31::from(-3).0));
    }

    #[test]
    fn test_cm_offset_from_wasm_i32_offset_odd_offsets_floor_division() {
        let dummy = DagToMir {
            module: crate::loader::BlocklessDagModule::from_file(
                "tests/test_cases/arithmetic.wasm",
            )
            .unwrap(),
            global_addresses: std::collections::HashMap::new(),
            global_types: std::collections::HashMap::new(),
            heap_start: 0,
        };
        // wasm_offset 1 -> half 0 -> cm_offset -1
        let v1 = dummy.cm_offset_from_wasm_i32_offset(1);
        assert_eq!(v1, Value::integer(M31::from(-1).0));

        // wasm_offset 3 -> half 1 -> cm_offset -2
        let v3 = dummy.cm_offset_from_wasm_i32_offset(3);
        assert_eq!(v3, Value::integer(M31::from(-2).0));

        // larger odd
        let v9 = dummy.cm_offset_from_wasm_i32_offset(9);
        assert_eq!(v9, Value::integer(M31::from(-5).0));
    }
}
