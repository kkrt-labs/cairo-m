pub mod i32;
pub mod memory;

use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{
    instruction::CalleeSignature, FunctionId, Instruction, MirType, Value, ValueId,
};
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::Node;

impl DagToMir {
    /// Convert a WASM operation to MIR instructions
    pub(crate) fn convert_wasm_op_to_mir(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        node: &Node,
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        let inputs: Result<Vec<Value>, _> = node
            .inputs
            .iter()
            .map(|input| self.get_input_value(input, context))
            .collect();
        let inputs = inputs?;

        match wasm_op {
            // Usual I32 operations
            Op::I32Add
            | Op::I32Sub
            | Op::I32Mul
            | Op::I32DivU
            | Op::I32DivS // unsupported
            | Op::I32RemS // unsupported
            | Op::I32RemU
            | Op::I32And
            | Op::I32Or
            | Op::I32Xor
            | Op::I32Shl
            | Op::I32ShrU
            | Op::I32ShrS // unsupported
            | Op::I32Rotl
            | Op::I32Rotr
            | Op::I32Eq
            | Op::I32Ne
            | Op::I32GtU
            | Op::I32GeU
            | Op::I32LtU
            | Op::I32LeU
            | Op::I32LtS
            | Op::I32GtS
            | Op::I32LeS
            | Op::I32GeS
            | Op::I32Eqz
            | Op::I32Const { .. } => {
                self.handle_i32_operations(node_idx, wasm_op, &inputs, context)
            }

            // Memory operations
            Op::I32Load { .. }
            | Op::I32Store { .. }
            | Op::GlobalGet { .. }
            | Op::GlobalSet { .. }
            => {
                self.handle_memory_operations(node_idx, wasm_op, &inputs, context)
            }

            // Local variable operations should be eliminated by WOMIR
            Op::LocalGet { .. } | Op::LocalSet { .. } | Op::LocalTee { .. } => {
                unreachable!()
            }

            Op::Call { function_index } => {
                let callee_id = FunctionId::new(*function_index as usize);

                // Get signature from wasm module
                let signature = self.module.with_program(|program| {
                    let func_type = program.c.get_func_type(*function_index);

                    // Handle param types with proper error handling
                    let param_types: Result<Vec<MirType>, DagToMirError> = func_type
                        .ty
                        .params()
                        .iter()
                        .map(|ty| {
                            Self::wasm_type_to_mir_type(ty, "unknown", "function call parameters")
                        })
                        .collect();

                    // Handle return types with proper error handling
                    let return_types: Result<Vec<MirType>, DagToMirError> = func_type
                        .ty
                        .results()
                        .iter()
                        .map(|ty| {
                            Self::wasm_type_to_mir_type(ty, "unknown", "function call return types")
                        })
                        .collect();

                    // Return both results
                    (param_types, return_types)
                });

                // Handle the errors from type conversion
                let (param_types, return_types) = signature;
                let param_types = param_types?;
                let return_types = return_types?;

                let signature = CalleeSignature { param_types, return_types };

                // Allocate one destination per return type
                let mut dests: Vec<ValueId> = Vec::new();
                for ty in &signature.return_types {
                    let id = context.mir_function.new_typed_value_id(ty.clone());
                    dests.push(id);
                }

                let instruction = Instruction::call(dests.clone(), callee_id, inputs, signature);
                context.get_current_block()?.push_instruction(instruction);

                match dests.len() {
                    0 => Ok(None),
                    1 => Ok(Some(dests[0])),
                    _ => unreachable!(),
                }
            }

            _ => {
                // Unsupported operation
                let suggestion = "This WASM operation is not yet implemented in the compiler";

                Err(DagToMirError::UnsupportedOperation {
                    op: format!("{:?}", wasm_op),
                    function_name: context.mir_function.name.clone(),
                    node_idx,
                    suggestion: suggestion.to_string(),
                })
            }
        }
    }
}
