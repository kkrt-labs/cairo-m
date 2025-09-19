use super::{wasm_type_to_mir_type, DagToMirContext, DagToMirError};
use crate::loader::BlocklessDagModule;
use cairo_m_compiler_mir::instruction::CalleeSignature;
use cairo_m_compiler_mir::{
    instruction::Instruction, BinaryOp, FunctionId, MirType, Value, ValueId,
};
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::Node;

impl DagToMirContext {
    /// Convert a WASM binary opcode to a MIR binary opcode
    /// TODO : bit shifts, rotations, u8 operations, etc.
    pub(super) fn wasm_binary_opcode_to_mir(
        &self,
        wasm_op: &Op,
        node_idx: usize,
    ) -> Result<BinaryOp, DagToMirError> {
        match wasm_op {
            Op::I32Add => Ok(BinaryOp::U32Add),
            Op::I32Sub => Ok(BinaryOp::U32Sub),
            Op::I32Mul => Ok(BinaryOp::U32Mul),
            Op::I32DivU => Ok(BinaryOp::U32Div),
            Op::I32Eq => Ok(BinaryOp::U32Eq),
            Op::I32Ne => Ok(BinaryOp::U32Neq),
            Op::I32GtU => Ok(BinaryOp::U32Greater),
            Op::I32GeU => Ok(BinaryOp::U32GreaterEqual),
            Op::I32LtU => Ok(BinaryOp::U32Less),
            Op::I32LeU => Ok(BinaryOp::U32LessEqual),
            Op::I32And => Ok(BinaryOp::U32BitwiseAnd),
            Op::I32Or => Ok(BinaryOp::U32BitwiseOr),
            Op::I32Xor => Ok(BinaryOp::U32BitwiseXor),
            _ => Err(DagToMirError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: self.mir_function.name.clone(),
                node_idx,
                suggestion: "".to_string(),
            }),
        }
    }

    pub(super) fn convert_wasm_binop_to_mir(
        &mut self,
        node_idx: usize,
        wasm_op: &Op,
        left: Value,
        right: Value,
        dest_type: MirType,
    ) -> Result<ValueId, DagToMirError> {
        let result_id = self.mir_function.new_typed_value_id(dest_type);
        let mir_op = self.wasm_binary_opcode_to_mir(wasm_op, node_idx)?;
        let instruction = Instruction::binary_op(mir_op, result_id, left, right);
        self.get_current_block()?.push_instruction(instruction);
        Ok(result_id)
    }

    /// Convert a WASM operation to MIR instructions
    pub(super) fn convert_wasm_op_to_mir(
        &mut self,
        node_idx: usize,
        wasm_op: &Op,
        node: &Node,
        module: &BlocklessDagModule,
    ) -> Result<ValueId, DagToMirError> {
        let inputs: Result<Vec<Value>, _> = node
            .inputs
            .iter()
            .map(|input| self.get_input_value(input))
            .collect();
        let inputs = inputs?;

        match wasm_op {
            // U32 Operations which are immediately convertible to MIR instructions
            Op::I32Add
            | Op::I32Sub
            | Op::I32Mul
            | Op::I32DivU
            | Op::I32And
            | Op::I32Or
            | Op::I32Xor => self.convert_wasm_binop_to_mir(
                node_idx,
                wasm_op,
                inputs[0],
                inputs[1],
                MirType::U32,
            ),

            // For comparisons, we produce a boolean result
            // This is not WASM compliant, but works if these values are only used in conditional branches
            // TODO : cast everything correctly or sync with VM so that comparisons between u32 produce u32 booleans
            Op::I32Eq | Op::I32Ne | Op::I32GtU | Op::I32GeU | Op::I32LtU | Op::I32LeU => self
                .convert_wasm_binop_to_mir(node_idx, wasm_op, inputs[0], inputs[1], MirType::Bool),

            // Signed comparison instructions: convert to unsigned by adding 2^31 (flips sign bit)
            // This maps signed range [-2^31, 2^31-1] to unsigned [0, 2^32-1] preserving order
            Op::I32LtS | Op::I32GtS | Op::I32LeS | Op::I32GeS => {
                let temp1 = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction1 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp1,
                    inputs[0],
                    Value::integer(0x80000000),
                );
                let temp2 = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction2 = Instruction::binary_op(
                    BinaryOp::U32Add,
                    temp2,
                    inputs[1],
                    Value::integer(0x80000000),
                );
                let result_id = self.mir_function.new_typed_value_id(MirType::Bool);
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
                self.get_current_block()?.push_instruction(instruction1);
                self.get_current_block()?.push_instruction(instruction2);
                self.get_current_block()?.push_instruction(instruction3);
                Ok(result_id)
            }

            // Zero comparison instruction, constructed by comparing the input to 0
            // TODO : fix type of result_id
            Op::I32Eqz => {
                let result_id = self.mir_function.new_typed_value_id(MirType::Bool);
                let instruction = Instruction::binary_op(
                    BinaryOp::U32Eq,
                    result_id,
                    inputs[0],
                    Value::integer(0),
                );
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            // Assigning a constant to a variable
            Op::I32Const { value } => {
                let result_id = self.mir_function.new_typed_value_id(MirType::U32);
                let instruction =
                    Instruction::assign(result_id, Value::integer(*value as u32), MirType::U32);
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            // Local variable operations should be eliminated by WOMIR
            Op::LocalGet { .. } | Op::LocalSet { .. } | Op::LocalTee { .. } => {
                unreachable!()
            }

            Op::Call { function_index } => {
                let result_id = self.mir_function.new_typed_value_id(MirType::U32);
                let callee_id = FunctionId::new(*function_index as usize);

                // Get signature from wasm module
                let program = &module.0;
                let func_type = program.m.get_func_type(*function_index);

                // Handle param types with proper error handling
                let param_types: Vec<MirType> = func_type
                    .ty
                    .params()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call parameters"))
                    .collect::<Result<Vec<MirType>, DagToMirError>>()?;

                // Handle return types with proper error handling
                let return_types: Vec<MirType> = func_type
                    .ty
                    .results()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call return types"))
                    .collect::<Result<Vec<MirType>, DagToMirError>>()?;

                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                let instruction = Instruction::call(vec![result_id], callee_id, inputs, signature);
                self.get_current_block()?.push_instruction(instruction);
                Ok(result_id)
            }

            _ => {
                // Unsupported operation
                let suggestion = "This WASM operation is not yet implemented in the compiler";

                Err(DagToMirError::UnsupportedOperation {
                    op: format!("{:?}", wasm_op),
                    function_name: self.mir_function.name.clone(),
                    node_idx,
                    suggestion: suggestion.to_string(),
                })
            }
        }
    }
}
