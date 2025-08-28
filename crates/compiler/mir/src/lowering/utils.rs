//! # Lowering Utilities
//!
//! This module contains shared utility functions used across the lowering
//! implementation.

use cairo_m_compiler_semantic::type_resolution::expression_semantic_type;
use cairo_m_compiler_semantic::types::TypeData;

use crate::instruction::CalleeSignature;
use crate::{FunctionId, Instruction, MirType, Value, ValueId};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    // Note: get_expression_type has been removed - use get_expr_type in builder.rs instead (has caching)

    /// Checks if we're currently in a loop context
    pub const fn in_loop(&self) -> bool {
        !self.state.loop_stack.is_empty()
    }

    /// Emits a call instruction with destinations and proper signature
    ///
    /// This helper centralizes the logic for emitting function calls with
    /// proper signatures and destination handling.
    pub(crate) fn emit_call_with_destinations(
        &mut self,
        func_id: FunctionId,
        args: Vec<Value>,
        dests: Vec<ValueId>,
    ) -> Result<(), String> {
        // Get the function signature
        let (param_types, return_types) = self.get_function_signature(func_id)?;

        // Create the CalleeSignature
        let signature = CalleeSignature {
            param_types,
            return_types,
        };

        // Create the call instruction with the signature
        let call_instr = Instruction::call(dests, func_id, args, signature);
        self.instr().add_instruction(call_instr);
        Ok(())
    }

    /// Emits a call instruction and discards the result(s)
    ///
    /// This is used for function calls in expression statements where the
    /// return value is not used. It handles both void functions and functions
    /// that return values by creating dummy destinations.
    pub(crate) fn emit_call_and_discard_result(
        &mut self,
        func_id: FunctionId,
        args: Vec<Value>,
        expr_id: cairo_m_compiler_semantic::semantic_index::ExpressionId,
    ) -> Result<(), String> {
        // Check the function's return type
        let func_expr_semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);

        match func_expr_semantic_type.data(self.ctx.db) {
            TypeData::Tuple(element_types) => {
                // Function returns a tuple - create destinations but don't use them
                let mut dests = Vec::new();
                for elem_type in element_types {
                    let mir_type = MirType::from_semantic_type(self.ctx.db, elem_type);
                    dests.push(self.state.mir_function.new_typed_value_id(mir_type));
                }
                self.emit_call_with_destinations(func_id, args, dests)?;
            }
            _ => {
                // Function returns a single value - create a destination but don't use it
                let return_type = MirType::from_semantic_type(self.ctx.db, func_expr_semantic_type);
                let dest = self.state.mir_function.new_typed_value_id(return_type);
                self.emit_call_with_destinations(func_id, args, vec![dest])?;
            }
        }
        Ok(())
    }
}
