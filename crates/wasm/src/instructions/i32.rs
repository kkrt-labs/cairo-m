use crate::flattening::{DagToMir, DagToMirContext, DagToMirError};
use cairo_m_compiler_mir::{BinaryOp, Instruction, MirType, Terminator, Value, ValueId};
use wasmparser::Operator as Op;

impl DagToMir {
    /// Convert a WASM binary opcode to a MIR binary opcode
    /// TODO : bit shifts, rotations, u8 operations, etc.
    fn wasm_binary_opcode_to_mir(
        &self,
        wasm_op: &Op,
        node_idx: usize,
        context: &DagToMirContext,
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
                function_name: context.mir_function.name.clone(),
                node_idx,
                suggestion: "".to_string(),
            }),
        }
    }

    pub fn convert_wasm_binop_to_mir(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        left: Value,
        right: Value,
        dest_type: MirType,
        context: &mut DagToMirContext,
    ) -> Result<Option<ValueId>, DagToMirError> {
        let result_id = context.mir_function.new_typed_value_id(dest_type);
        let mir_op = self.wasm_binary_opcode_to_mir(wasm_op, node_idx, context)?;
        let instruction = Instruction::binary_op(mir_op, result_id, left, right);
        context.get_current_block()?.push_instruction(instruction);
        Ok(Some(result_id))
    }

    /// Helper: mask a u32 shift amount to 0..31 and cast to Felt for loop counters
    fn mask_shift_to_felt(
        &self,
        node_idx: usize,
        wasm_op: &Op,
        shift_val: Value,
        context: &mut DagToMirContext,
    ) -> Result<Value, DagToMirError> {
        let masked_u32: Value = match shift_val {
            Value::Literal(value) => {
                let masked = value.as_integer().unwrap() & 0b11111;
                Value::integer(masked)
            }
            Value::Operand(_id) => {
                let masked_id = context.mir_function.new_typed_value_id(MirType::U32);
                let mask_instr = Instruction::binary_op(
                    BinaryOp::U32BitwiseAnd,
                    masked_id,
                    shift_val,
                    Value::integer(0b11111),
                );
                context.get_current_block()?.push_instruction(mask_instr);
                Value::operand(masked_id)
            }
            Value::Error => {
                return Err(DagToMirError::UnsupportedOperation {
                    op: format!("{:?}", wasm_op),
                    function_name: context.mir_function.name.clone(),
                    node_idx,
                    suggestion: "".to_string(),
                });
            }
        };

        // Cast masked amount to Felt for use as loop counter
        let count_felt_id = context.mir_function.new_typed_value_id(MirType::Felt);
        let cast = Instruction::cast(count_felt_id, masked_u32, MirType::U32, MirType::Felt);
        context.get_current_block()?.push_instruction(cast);
        Ok(Value::operand(count_felt_id))
    }

    /// Helper: emit a generic shift loop using multiply/divide by 2
    /// - initial_val: U32 value to be shifted
    /// - count_felt: Felt loop counter (masked to 0..31)
    /// - step_op: U32Mul for left shift, U32Div for logical right shift
    fn emit_shift(
        &self,
        context: &mut DagToMirContext,
        initial_val: Value,
        count_felt: Value,
        step_op: BinaryOp,
    ) -> Result<ValueId, DagToMirError> {
        let header_block = context.mir_function.add_basic_block();
        let body_block = context.mir_function.add_basic_block();
        let exit_block = context.mir_function.add_basic_block();

        let counter = context.mir_function.new_typed_value_id(MirType::Felt);
        let val = context.mir_function.new_typed_value_id(MirType::U32);
        let shifted = context.mir_function.new_typed_value_id(MirType::U32);
        let decremented = context.mir_function.new_typed_value_id(MirType::Felt);

        // header phis
        let phi_counter = Instruction::phi(
            counter,
            MirType::Felt,
            vec![
                (context.get_current_block_id(), count_felt),
                (body_block, Value::operand(decremented)),
            ],
        );
        let phi_val = Instruction::phi(
            val,
            MirType::U32,
            vec![
                (context.get_current_block_id(), initial_val),
                (body_block, Value::operand(shifted)),
            ],
        );

        // body ops
        let step = Instruction::binary_op(step_op, shifted, Value::operand(val), Value::integer(2));
        let dec = Instruction::binary_op(
            BinaryOp::Sub,
            decremented,
            Value::operand(counter),
            Value::integer(1),
        );

        // wire CFG
        let t1 = Terminator::Jump {
            target: header_block,
        };
        context.get_current_block()?.set_terminator(t1);
        context.set_current_block(header_block);
        context.get_current_block()?.push_instruction(phi_counter);
        context.get_current_block()?.push_instruction(phi_val);
        let t2 = Terminator::If {
            condition: Value::operand(counter),
            then_target: body_block,
            else_target: exit_block,
        };
        context.get_current_block()?.set_terminator(t2);
        context.set_current_block(body_block);
        context.get_current_block()?.push_instruction(step);
        context.get_current_block()?.push_instruction(dec);
        let t3 = Terminator::Jump {
            target: header_block,
        };
        context.get_current_block()?.set_terminator(t3);
        context.set_current_block(exit_block);

        Ok(val)
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
            | Op::I32Xor => self.convert_wasm_binop_to_mir(
                node_idx,
                wasm_op,
                inputs[0],
                inputs[1],
                MirType::U32,
                context,
            ),

            // Unsigned remainder: a % b = a - (a / b) * b
            Op::I32RemU => {
                // q = a / b
                let q_id = context.mir_function.new_typed_value_id(MirType::U32);
                let div_instr =
                    Instruction::binary_op(BinaryOp::U32Div, q_id, inputs[0], inputs[1]);

                // t = q * b
                let t_id = context.mir_function.new_typed_value_id(MirType::U32);
                let mul_instr =
                    Instruction::binary_op(BinaryOp::U32Mul, t_id, Value::operand(q_id), inputs[1]);

                // r = a - t
                let r_id = context.mir_function.new_typed_value_id(MirType::U32);
                let sub_instr =
                    Instruction::binary_op(BinaryOp::U32Sub, r_id, inputs[0], Value::operand(t_id));

                let block = context.get_current_block()?;
                block.push_instruction(div_instr);
                block.push_instruction(mul_instr);
                block.push_instruction(sub_instr);

                Ok(Some(r_id))
            }

            // For comparisons, we produce a boolean result
            // This is not WASM compliant, but works if these values are only used in conditional branches
            // TODO : cast everything correctly or sync with VM so that comparisons between u32 produce u32 booleans
            Op::I32Eq | Op::I32Ne | Op::I32GtU | Op::I32GeU | Op::I32LtU | Op::I32LeU => self
                .convert_wasm_binop_to_mir(
                    node_idx,
                    wasm_op,
                    inputs[0],
                    inputs[1],
                    MirType::Bool,
                    context,
                ),

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

            // Shift left operation, using repeated multiplication (very bad performance)
            Op::I32Shl => {
                let count_felt = self.mask_shift_to_felt(node_idx, wasm_op, inputs[1], context)?;
                let result = self.emit_shift(context, inputs[0], count_felt, BinaryOp::U32Mul)?;
                Ok(Some(result))
            }

            // Unsigned shift right operation, using repeated division (very bad performance)
            Op::I32ShrU => {
                let count_felt = self.mask_shift_to_felt(node_idx, wasm_op, inputs[1], context)?;
                let result = self.emit_shift(context, inputs[0], count_felt, BinaryOp::U32Div)?;
                Ok(Some(result))
            }

            // Rotate left
            Op::I32Rotl => {
                // Mask amount and build left/right shifts using helper
                let count_felt = self.mask_shift_to_felt(node_idx, wasm_op, inputs[1], context)?;
                let left_val =
                    self.emit_shift(context, inputs[0], count_felt.clone(), BinaryOp::U32Mul)?;

                // Compute (32 - n) and emit right shift
                let comp_id = context.mir_function.new_typed_value_id(MirType::Felt);
                let comp =
                    Instruction::binary_op(BinaryOp::Sub, comp_id, Value::integer(32), count_felt);
                context.get_current_block()?.push_instruction(comp);
                let right_val = self.emit_shift(
                    context,
                    inputs[0],
                    Value::operand(comp_id),
                    BinaryOp::U32Div,
                )?;

                // Or the two parts
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let combine = Instruction::binary_op(
                    BinaryOp::U32BitwiseOr,
                    result_id,
                    Value::operand(left_val),
                    Value::operand(right_val),
                );
                context.get_current_block()?.push_instruction(combine);
                Ok(Some(result_id))
            }

            // Rotate right
            Op::I32Rotr => {
                // Mask amount and build left/right shifts using helper
                let count_felt = self.mask_shift_to_felt(node_idx, wasm_op, inputs[1], context)?;
                let left_val =
                    self.emit_shift(context, inputs[0], count_felt.clone(), BinaryOp::U32Div)?;

                // Compute (32 - n) and emit right shift
                let comp_id = context.mir_function.new_typed_value_id(MirType::Felt);
                let comp =
                    Instruction::binary_op(BinaryOp::Sub, comp_id, Value::integer(32), count_felt);
                context.get_current_block()?.push_instruction(comp);
                let right_val = self.emit_shift(
                    context,
                    inputs[0],
                    Value::operand(comp_id),
                    BinaryOp::U32Mul,
                )?;

                // Or the two parts
                let result_id = context.mir_function.new_typed_value_id(MirType::U32);
                let combine = Instruction::binary_op(
                    BinaryOp::U32BitwiseOr,
                    result_id,
                    Value::operand(left_val),
                    Value::operand(right_val),
                );
                context.get_current_block()?.push_instruction(combine);
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
