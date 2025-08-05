//! # Lowering Utilities
//!
//! This module contains shared utility functions used across the lowering
//! implementation.

use cairo_m_compiler_parser::parser::{Expression, Spanned};
use cairo_m_compiler_semantic::definition::DefinitionKind;
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, DefinitionIndex, ExpressionId};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;
use cairo_m_compiler_semantic::SemanticIndex;

use crate::instruction::CalleeSignature;
use crate::mir_types::InstructionEmitter;
use crate::{
    BasicBlockId, FunctionId, Instruction, InstructionKind, MirType, Value, ValueId, ValueKind,
};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Gets the MIR type for an expression by its ID
    pub fn get_expression_type(&self, expr_id: ExpressionId) -> MirType {
        let semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
        MirType::from_semantic_type(self.ctx.db, semantic_type)
    }

    /// Checks if we're currently in a loop context
    pub const fn in_loop(&self) -> bool {
        !self.state.loop_stack.is_empty()
    }

    /// Gets the current loop's continue and break targets
    pub fn current_loop_targets(&self) -> Option<(BasicBlockId, BasicBlockId)> {
        self.state.loop_stack.last().copied()
    }

    /// Binds a value to a variable identifier with complete lifecycle management
    ///
    /// This helper encapsulates:
    /// 1. Resolving the identifier to its DefinitionId and MirDefinitionId
    /// 2. Checking if the variable is used (and early return if not)
    /// 3. For simple values (not addresses), directly mapping the variable to the value
    /// 4. For addresses, allocating stack space and storing the value
    /// 5. Updating the definition_to_value mapping
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
        let is_used = is_definition_used(self.ctx.semantic_index, def_idx);
        if !is_used {
            // Map to a dummy value and return early
            let dummy_addr = self.state.mir_function.new_value_id();
            self.state
                .definition_to_value
                .insert(mir_def_id, dummy_addr);
            return Ok(());
        }

        // Check if the value is already a simple value (not an address)
        // If so, we can just map the variable directly to it without allocation
        match value {
            Value::Operand(value_id) => {
                // Check if this is a value (not an address) that we can use directly
                if self.state.mir_function.is_value(value_id) {
                    // Direct mapping - no allocation needed!
                    self.state.definition_to_value.insert(mir_def_id, value_id);
                    return Ok(());
                }
            }
            Value::Literal(_) => {
                // Literals are immediate values, not addresses
                // We need to create a value instruction for them
                let semantic_type =
                    definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
                let var_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
                let value_id = self.state.mir_function.new_typed_value_id(var_type);

                // Register as a Value since it's an immediate
                self.state
                    .mir_function
                    .register_value_kind(value_id, ValueKind::Value);

                // Create a move/immediate instruction
                self.instr()
                    .add_instruction(Instruction::assign(value_id, value));

                // Map the variable directly to this value
                self.state.definition_to_value.insert(mir_def_id, value_id);
                return Ok(());
            }
            _ => {
                // For other cases, fall through to allocation
            }
        }

        // For addresses or complex values, we need to allocate and store
        // Get the variable's semantic type and convert to MirType
        let semantic_type = definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
        let var_type = MirType::from_semantic_type(self.ctx.db, semantic_type);

        // Allocate space for the variable
        let var_addr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(var_type.clone()));

        // Register this as an address since stack_alloc returns an address
        self.state
            .mir_function
            .register_value_kind(var_addr, ValueKind::Address);

        let mut instr = self.instr();
        instr.add_instruction(Instruction::stack_alloc(var_addr, var_type.size_units()));

        // Emit the appropriate store instruction based on type
        instr.add_instruction(var_type.emit_store(Value::operand(var_addr), value)?);

        // Update the definition to value mapping
        self.state.definition_to_value.insert(mir_def_id, var_addr);
        Ok(())
    }

    /// Resolves a function call's callee to its FunctionId
    ///
    /// This centralizes the logic for resolving function calls, whether they're
    /// local functions or imported ones. Handles both simple identifiers and
    /// member access patterns.
    pub fn resolve_function(&mut self, callee: &Spanned<Expression>) -> Result<FunctionId, String> {
        match callee.value() {
            Expression::Identifier(func_name) => {
                // Get the scope for the callee from its expression info
                let callee_expr_id = self
                    .ctx
                    .semantic_index
                    .expression_id_by_span(callee.span())
                    .ok_or_else(|| {
                        format!("No ExpressionId found for callee span {:?}", callee.span())
                    })?;
                let callee_expr_info = self
                    .ctx
                    .semantic_index
                    .expression(callee_expr_id)
                    .ok_or_else(|| format!("No ExpressionInfo for callee ID {callee_expr_id:?}"))?;

                // Resolve the function name in the appropriate scope
                let (local_def_idx, local_def) = self
                    .ctx
                    .semantic_index
                    .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                    .ok_or_else(|| {
                        format!(
                            "Failed to resolve function '{}' in scope {:?}",
                            func_name.value(),
                            callee_expr_info.scope_id
                        )
                    })?;

                // Handle function resolution: local functions vs imported functions
                match &local_def.kind {
                    DefinitionKind::Function(_) => {
                        // Local function - use current file
                        let func_def_id =
                            DefinitionId::new(self.ctx.db, self.ctx.file, local_def_idx);
                        if let Some((_, func_id)) = self.ctx.function_mapping.get(&func_def_id) {
                            Ok(*func_id)
                        } else {
                            Err(format!(
                                "Function '{}' not found in function mapping",
                                func_name.value()
                            ))
                        }
                    }
                    DefinitionKind::Use(use_ref) => {
                        // Imported function - follow the import chain
                        self.resolve_imported_function(
                            use_ref.imported_module.value(),
                            func_name.value(),
                        )
                        .ok_or_else(|| {
                            format!(
                                "Failed to resolve imported function '{}' from module '{}'",
                                func_name.value(),
                                use_ref.imported_module.value()
                            )
                        })
                    }
                    _ => Err(format!(
                        "'{}' is not a function or import",
                        func_name.value()
                    )),
                }
            }
            Expression::MemberAccess { .. } => {
                // For member access patterns (e.g., module.function), use existing resolution
                self.resolve_callee_expression(callee)
            }
            _ => Err("Unsupported callee expression type".to_string()),
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
        let mut call_instr = Instruction::call(dests, func_id, args);
        if let InstructionKind::Call {
            signature: ref mut sig,
            ..
        } = &mut call_instr.kind
        {
            *sig = signature;
        }
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
            TypeData::Tuple(element_types) if element_types.is_empty() => {
                // Function returns unit/void
                let (param_types, return_types) = self.get_function_signature(func_id)?;
                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };
                self.instr().void_call(func_id, args, signature);
            }
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

pub(crate) fn is_definition_used(semantic_index: &SemanticIndex, def_idx: DefinitionIndex) -> bool {
    semantic_index.definition(def_idx).is_none_or(|definition| {
        semantic_index
            .place_table(definition.scope_id)
            .is_none_or(|place_table| {
                place_table
                    .place(definition.place_id)
                    .is_none_or(|place| place.is_used())
            })
    })
}
