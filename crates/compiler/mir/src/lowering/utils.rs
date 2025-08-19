//! # Lowering Utilities
//!
//! This module contains shared utility functions used across the lowering
//! implementation.

use cairo_m_compiler_parser::parser::Spanned;
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;

use crate::instruction::CalleeSignature;
use crate::{BasicBlockId, FunctionId, Instruction, Literal, MirType, Value, ValueId};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    // Note: get_expression_type has been removed - use get_expr_type in builder.rs instead (has caching)

    /// Checks if we're currently in a loop context
    pub const fn in_loop(&self) -> bool {
        !self.state.loop_stack.is_empty()
    }

    /// Gets the current loop's continue and break targets
    pub fn current_loop_targets(&self) -> Option<(BasicBlockId, BasicBlockId)> {
        self.state.loop_stack.last().copied()
    }

    /// Binds a value to a variable identifier using pure value mapping
    ///
    /// This helper encapsulates:
    /// 1. Resolving the identifier to its DefinitionId and MirDefinitionId
    /// 2. For operand values, directly mapping the variable to the value
    /// 3. For literals, creating an assign instruction and mapping to the result
    /// 4. Updating the definition_to_value mapping
    ///
    /// Note: Variables are always bound to values,
    /// never to memory addresses. Arrays are handled separately with memory operations.
    pub fn bind_variable(
        &mut self,
        name: &Spanned<String>,
        scope: FileScopeId,
        value: Value,
    ) -> Result<(), String> {
        // Resolve the identifier to its definition
        let (def_idx, _) = self
            .ctx
            .semantic_index
            .resolve_name_to_definition(name.value(), scope)
            .ok_or_else(|| {
                format!(
                    "Failed to resolve variable '{}' in scope {:?}",
                    name.value(),
                    scope
                )
            })?;

        let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
        let mir_def_id = self.convert_definition_id(def_id);

        let semantic_type = definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
        let var_type = MirType::from_semantic_type(self.ctx.db, semantic_type);

        match value {
            Value::Operand(value_id) => {
                // All operand values are bound directly
                self.state.definition_to_value.insert(mir_def_id, value_id);
                Ok(())
            }
            Value::Literal(lit) => {
                // Literals are immediate values - create a value instruction for them
                let literal_type = match lit {
                    Literal::Integer(_) => {
                        // Use the variable's type to preserve u32 vs felt distinction
                        match &var_type {
                            MirType::U32 => MirType::u32(),
                            MirType::Felt => MirType::felt(),
                            _ => panic!("Literal type mismatch: {:?} != {:?}", var_type, lit),
                        }
                    }
                    Literal::Boolean(_) => MirType::bool(),
                    Literal::Unit => MirType::unit(),
                };
                let value_id = self
                    .state
                    .mir_function
                    .new_typed_value_id(literal_type.clone());
                self.instr()
                    .add_instruction(Instruction::assign(value_id, value, literal_type));
                self.state.definition_to_value.insert(mir_def_id, value_id);
                Ok(())
            }
            Value::Error => {
                // Error values are used for recovery - create a placeholder
                let value_id = self.state.mir_function.new_typed_value_id(var_type);
                self.state.definition_to_value.insert(mir_def_id, value_id);
                Ok(())
            }
        }
    }

    /// Emits a call instruction with destinations and proper signature
    ///
    /// This helper centralizes the logic for emitting function calls with
    /// proper signatures and destination handling.
    pub fn emit_call_with_destinations(
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
    pub fn emit_call_and_discard_result(
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
