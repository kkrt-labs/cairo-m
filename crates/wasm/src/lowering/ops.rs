use super::{wasm_type_to_mir_type, DagToCasmContext, DagToCasmError};
use crate::loader::BlocklessDagModule;
use cairo_m_compiler_mir::instruction::CalleeSignature;
use cairo_m_compiler_mir::{BinaryOp, MirType, Value, ValueId};
use cairo_m_runner::memory::MAX_ADDRESS;
use wasmparser::Operator as Op;
use womir::loader::blockless_dag::Node;

impl DagToCasmContext {
    /// Convert a WASM binary opcode to a casm binary opcode
    /// TODO : bit shifts, rotations, u8 operations, etc.
    pub(super) fn wasm_binary_opcode_to_mir(
        &self,
        wasm_op: &Op,
        node_idx: usize,
    ) -> Result<BinaryOp, DagToCasmError> {
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
            _ => Err(DagToCasmError::UnsupportedOperation {
                op: format!("{:?}", wasm_op),
                function_name: self.casm_builder.layout.name.clone(),
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
    ) -> Result<Option<ValueId>, DagToCasmError> {
        let result_id = self.new_typed_value_id(dest_type);
        let mir_op = self.wasm_binary_opcode_to_mir(wasm_op, node_idx)?;
        self.casm_builder
            .binary_op(mir_op, result_id, left, right, None)?;
        Ok(Some(result_id))
    }

    /// Compute the Cairo-M memory address from a WASM address value.
    /// cm_address = heap_start - (wasm_address / 2) - (wasm_offset / 2) - 1
    /// This is done dynamically using 3 mir instructions, which is pretty inefficient.
    fn compute_cm_address_from_wasm_address(
        &mut self,
        wasm_address: Value,
        wasm_offset: u64,
    ) -> Result<ValueId, DagToCasmError> {
        // temp1 = wasm_address / 2
        let temp1 = self.new_typed_value_id(MirType::U32);
        self.casm_builder.binary_op(
            BinaryOp::U32Div,
            temp1,
            wasm_address,
            Value::integer(2),
            None,
        )?;

        // cm_address = heap_start + cm_offset - temp1
        // without globals, heap starts at MAX_ADDRESS
        let cm_address = self.new_typed_value_id(MirType::Felt);
        let cm_offset = self.cm_offset_from_wasm_i32_offset(wasm_offset);
        self.casm_builder.binary_op(
            BinaryOp::Sub,
            cm_address,
            Value::integer((MAX_ADDRESS as i32 + cm_offset) as u32),
            Value::operand(temp1),
            None,
        )?;

        Ok(cm_address)
    }

    /// Convert a WASM i32 memory offset (in bytes) to Cairo-M offset (in felts)
    const fn cm_offset_from_wasm_i32_offset(&self, wasm_offset: u64) -> i32 {
        -((wasm_offset / 2) as i32) - 1
    }

    /// Convert a WASM operation to MIR instructions
    pub(super) fn convert_wasm_op_to_casm(
        &mut self,
        node_idx: usize,
        wasm_op: &Op,
        node: &Node,
        module: &BlocklessDagModule,
    ) -> Result<Option<ValueId>, DagToCasmError> {
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
            Op::I32Eq | Op::I32LtU => self.convert_wasm_binop_to_mir(
                node_idx,
                wasm_op,
                inputs[0],
                inputs[1],
                MirType::Bool,
            ),

            // (a > b) == (b < a)
            Op::I32GtU => self.convert_wasm_binop_to_mir(
                node_idx,
                &Op::I32LtU,
                inputs[1],
                inputs[0],
                MirType::Bool,
            ),

            // a != b == 1 - (a == b)
            // TODO : peephole optimization to swap conditional branches instead if possible
            Op::I32Ne => {
                let result_id = self.new_typed_value_id(MirType::Bool);
                let temp = self.new_typed_value_id(MirType::Bool);
                self.casm_builder
                    .binary_op(BinaryOp::U32Eq, temp, inputs[0], inputs[1], None)?;
                self.casm_builder.binary_op(
                    BinaryOp::Sub,
                    result_id,
                    Value::integer(1),
                    Value::operand(temp),
                    None,
                )?;
                Ok(Some(result_id))
            }

            // (a <= b) == !(b < a)
            // (a >= b) == !(a < b)
            Op::I32LeU | Op::I32GeU => {
                let temp = self.new_typed_value_id(MirType::Bool);
                let result_id = self.new_typed_value_id(MirType::Bool);
                let (left, right, op) = match wasm_op {
                    Op::I32LeU => (inputs[1], inputs[0], BinaryOp::U32Less),
                    Op::I32GeU => (inputs[0], inputs[1], BinaryOp::U32Less),
                    _ => unreachable!(),
                };
                self.casm_builder.binary_op(op, temp, left, right, None)?;
                self.casm_builder.binary_op(
                    BinaryOp::Sub,
                    result_id,
                    Value::integer(1),
                    Value::operand(temp),
                    None,
                )?;
                Ok(Some(result_id))
            }

            // Signed comparison instructions: convert to unsigned by adding 2^31 (flips sign bit)
            // This maps signed range [-2^31, 2^31-1] to unsigned [0, 2^32-1] preserving order
            Op::I32LtS | Op::I32GtS | Op::I32LeS | Op::I32GeS => {
                let temp1 = self.new_typed_value_id(MirType::U32);
                self.casm_builder.binary_op(
                    BinaryOp::U32Add,
                    temp1,
                    inputs[0],
                    Value::integer(0x80000000),
                    None,
                )?;
                let temp2 = self.new_typed_value_id(MirType::U32);
                self.casm_builder.binary_op(
                    BinaryOp::U32Add,
                    temp2,
                    inputs[1],
                    Value::integer(0x80000000),
                    None,
                )?;
                let temp = self.new_typed_value_id(MirType::Bool);
                let (left, right) = match wasm_op {
                    Op::I32LtS => (temp1, temp2),
                    Op::I32GtS => (temp2, temp1),
                    Op::I32LeS => (temp2, temp1),
                    Op::I32GeS => (temp1, temp2),
                    _ => unreachable!(),
                };
                self.casm_builder.binary_op(
                    BinaryOp::U32Less,
                    temp,
                    Value::operand(left),
                    Value::operand(right),
                    None,
                )?;
                // For "or equal" variants, we apply the same 1 - (a < b) trick as above
                let result_id = if wasm_op == &Op::I32LeS || wasm_op == &Op::I32GeS {
                    let result_id = self.new_typed_value_id(MirType::Bool);
                    self.casm_builder.binary_op(
                        BinaryOp::Sub,
                        result_id,
                        Value::integer(1),
                        Value::operand(temp),
                        None,
                    )?;
                    result_id
                } else {
                    temp
                };
                Ok(Some(result_id))
            }

            // Zero comparison instruction, constructed by comparing the input to 0
            // TODO : fix type of result_id
            Op::I32Eqz => {
                let result_id = self.new_typed_value_id(MirType::Bool);
                self.casm_builder.binary_op(
                    BinaryOp::U32Eq,
                    result_id,
                    inputs[0],
                    Value::integer(0),
                    None,
                )?;
                Ok(Some(result_id))
            }

            // Assigning a constant to a variable
            Op::I32Const { value } => {
                let result_id = self.new_typed_value_id(MirType::U32);
                self.casm_builder.assign(
                    result_id,
                    Value::integer(*value as u32),
                    &MirType::U32,
                    None,
                )?;
                Ok(Some(result_id))
            }

            // Local variable operations should be eliminated by WOMIR
            Op::LocalGet { .. } | Op::LocalSet { .. } | Op::LocalTee { .. } => {
                unreachable!()
            }

            Op::Call { function_index } => {
                // Get signature from wasm module
                let program = &module.0;
                let func_type = program.m.get_func_type(*function_index);

                // Handle param types with proper error handling
                let param_types: Vec<MirType> = func_type
                    .ty
                    .params()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call parameters"))
                    .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

                // Handle return types with proper error handling
                let return_types: Vec<MirType> = func_type
                    .ty
                    .results()
                    .iter()
                    .map(|ty| wasm_type_to_mir_type(ty, "unknown", "function call return types"))
                    .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                let result_id = self.new_typed_value_id(MirType::U32);

                // Get function name from module exports
                let func_name = module
                    .0
                    .m
                    .exported_functions
                    .get(function_index)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("func_{}", function_index));

                self.casm_builder
                    .lower_call(&func_name, &inputs, &signature, &[result_id])?;
                Ok(Some(result_id))
            }

            // Load I32 from memory
            // The conversion from wasm address to MIR address is :
            // cm_address = heap_start - (wasm_address / 2) - 1
            // cm_offset = -(wasm_offset / 2)
            // Where the 1/2 factor comes from the size conversion u32 = 4 bytes = 2 felts
            Op::I32Load { memarg, .. } => {
                let base_address = inputs[0];
                let cm_address =
                    self.compute_cm_address_from_wasm_address(base_address, memarg.offset)?;
                let result_id = self.new_typed_value_id(MirType::U32);

                // Load u32 from memory: read two slots from [cm_address + 0] and [cm_address + 1]
                let base_off = self.casm_builder.layout.get_offset(cm_address)?;
                let dest_off = self.casm_builder.layout.allocate_local(result_id, 2)?;

                // Load first slot: [[fp + base_off] + 0]
                self.casm_builder.store_from_double_deref_fp_imm(
                    base_off,
                    0,
                    dest_off,
                    format!("[fp + {}] = [[fp + {}] + 0] (u32 lo)", dest_off, base_off),
                );

                // Load second slot: [[fp + base_off] + 1]
                self.casm_builder.store_from_double_deref_fp_imm(
                    base_off,
                    1,
                    dest_off + 1,
                    format!(
                        "[fp + {}] = [[fp + {}] + 1] (u32 hi)",
                        dest_off + 1,
                        base_off
                    ),
                );

                Ok(Some(result_id))
            }

            // Store I32 in memory
            // See above for address computation
            Op::I32Store { memarg, .. } => {
                let base_address = inputs[0];
                let cm_address =
                    self.compute_cm_address_from_wasm_address(base_address, memarg.offset)?;
                let value = inputs[1];

                // Store u32 to memory: write two slots to [cm_address + 0] and [cm_address + 1]
                let base_off = self.casm_builder.layout.get_offset(cm_address)?;

                // Get the value offset (it's a u32, so 2 slots)
                let value_off = match value {
                    Value::Operand(vid) => self.casm_builder.layout.get_offset(vid)?,
                    _ => {
                        return Err(DagToCasmError::UnsupportedOperation {
                            op: "I32Store with non-operand value".to_string(),
                            function_name: self.casm_builder.layout.name.clone(),
                            node_idx,
                            suggestion: "Store operands only".to_string(),
                        })
                    }
                };

                // Store first slot: [[fp + base_off] + 0] = [fp + value_off]
                self.casm_builder.store_to_double_deref_fp_imm(
                    value_off,
                    base_off,
                    0,
                    format!("[[fp + {}] + 0] = [fp + {}] (u32 lo)", base_off, value_off),
                );

                // Store second slot: [[fp + base_off] + 1] = [fp + value_off + 1]
                self.casm_builder.store_to_double_deref_fp_imm(
                    value_off + 1,
                    base_off,
                    1,
                    format!(
                        "[[fp + {}] + 1] = [fp + {}] (u32 hi)",
                        base_off,
                        value_off + 1
                    ),
                );

                Ok(None)
            }

            _ => {
                // Unsupported operation
                let suggestion = "This WASM operation is not yet implemented in the compiler";

                Err(DagToCasmError::UnsupportedOperation {
                    op: format!("{:?}", wasm_op),
                    function_name: self.casm_builder.layout.name.clone(),
                    node_idx,
                    suggestion: suggestion.to_string(),
                })
            }
        }
    }
}
