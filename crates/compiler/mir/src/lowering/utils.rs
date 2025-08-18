//! # Lowering Utilities
//!
//! This module contains shared utility functions used across the lowering
//! implementation.

use cairo_m_compiler_parser::parser::Spanned;
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, DefinitionIndex};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;
use cairo_m_compiler_semantic::SemanticIndex;

use crate::instruction::CalleeSignature;
use crate::layout::DataLayout;
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

        // If the variable is not used, map to a dummy value and exit
        let is_used = is_definition_used(self.ctx.semantic_index, def_idx);
        if !is_used {
            let dummy_addr = self.state.mir_function.new_value_id();
            self.state
                .definition_to_value
                .insert(mir_def_id, dummy_addr);
            return Ok(());
        }

        let semantic_type = definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
        let var_type = MirType::from_semantic_type(self.ctx.db, semantic_type);

        // Handle primitive types and literals
        match value {
            Value::Operand(value_id) => {
                if let Some(value_type) = self.state.mir_function.get_value_type(value_id) {
                    if !matches!(value_type, MirType::Pointer(_)) {
                        // It's a simple value, not a pointer. Use directly.
                        self.state.definition_to_value.insert(mir_def_id, value_id);
                        return Ok(());
                    }
                }
            }
            Value::Literal(lit) => {
                // Literals are immediate values - create a value instruction for them
                // IMPORTANT: For primitive types, use the variable's type to preserve u32 vs felt distinction
                // For aggregate types (tuple/struct), this path shouldn't be taken - those are handled elsewhere
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
                return Ok(());
            }
            _ => {}
        }

        // For pointers (including tuple/struct addresses), just bind directly
        // The allocation already exists and is populated
        if let Value::Operand(value_id) = value {
            if let Some(value_type) = self.state.mir_function.get_value_type(value_id) {
                if matches!(value_type, MirType::Pointer(_)) {
                    self.state.definition_to_value.insert(mir_def_id, value_id);
                    return Ok(());
                }
            }
        }

        // Fallback for cases we haven't handled
        let var_addr = self.alloc_frame(var_type.clone());
        self.instr()
            .store(Value::operand(var_addr), value, var_type);
        self.state.definition_to_value.insert(mir_def_id, var_addr);
        Ok(())
    }

    /// Copies a composite type (tuple or struct) from a source address to a destination address.
    ///
    /// This function generates a series of `getelementptr`, `load`, and `store` instructions
    /// to perform an element-wise copy, avoiding incorrect "composite store" operations.
    pub fn copy_composite_type(
        &mut self,
        dest_addr: Value,
        src_addr: Value,
        ty: &MirType,
    ) -> Result<(), String> {
        let layout = DataLayout::new();
        match ty {
            MirType::Tuple(element_types) => {
                for (i, elem_type) in element_types.iter().enumerate() {
                    let offset = layout
                        .tuple_offset(ty, i)
                        .ok_or_else(|| format!("Invalid tuple index {} for type", i))?;
                    let offset_val = Value::integer(offset as i32);

                    // Get pointer to source element
                    let src_elem_ptr = self
                        .state
                        .mir_function
                        .new_typed_value_id(MirType::pointer(elem_type.clone()));
                    self.instr()
                        .get_element_ptr_to(src_elem_ptr, src_addr, offset_val);

                    // Load value from source
                    let loaded_val = self
                        .state
                        .mir_function
                        .new_typed_value_id(elem_type.clone());
                    self.instr().load_to(
                        elem_type.clone(),
                        loaded_val,
                        Value::operand(src_elem_ptr),
                    );

                    // Get pointer to destination element
                    let dest_elem_ptr = self
                        .state
                        .mir_function
                        .new_typed_value_id(MirType::pointer(elem_type.clone()));
                    self.instr()
                        .get_element_ptr_to(dest_elem_ptr, dest_addr, offset_val);

                    // Store value to destination
                    self.store_value(
                        Value::operand(dest_elem_ptr),
                        Value::operand(loaded_val),
                        elem_type.clone(),
                    );
                }
            }
            MirType::Struct { fields, .. } => {
                for (field_name, field_type) in fields {
                    let offset = layout
                        .field_offset(ty, field_name)
                        .ok_or_else(|| format!("Field {} not found in struct type", field_name))?;
                    let offset_val = Value::integer(offset as i32);

                    // Get pointer to source field
                    let src_elem_ptr = self
                        .state
                        .mir_function
                        .new_typed_value_id(MirType::pointer(field_type.clone()));
                    self.instr()
                        .get_element_ptr_to(src_elem_ptr, src_addr, offset_val);

                    // Load value from source
                    let loaded_val = self
                        .state
                        .mir_function
                        .new_typed_value_id(field_type.clone());
                    self.instr().load_to(
                        field_type.clone(),
                        loaded_val,
                        Value::operand(src_elem_ptr),
                    );

                    // Get pointer to destination field
                    let dest_elem_ptr = self
                        .state
                        .mir_function
                        .new_typed_value_id(MirType::pointer(field_type.clone()));
                    self.instr()
                        .get_element_ptr_to(dest_elem_ptr, dest_addr, offset_val);

                    // Store value to destination
                    self.store_value(
                        Value::operand(dest_elem_ptr),
                        Value::operand(loaded_val),
                        field_type.clone(),
                    );
                }
            }
            _ => {
                return Err(format!(
                    "copy_composite_type called on non-composite type: {:?}",
                    ty
                ))
            }
        }
        Ok(())
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
