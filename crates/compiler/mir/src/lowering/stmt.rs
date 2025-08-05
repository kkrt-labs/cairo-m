//! # Statement Lowering
//!
//! This module contains the trait and implementations for lowering statements
//! from the AST to MIR instructions.

use cairo_m_compiler_parser::parser::{Expression, Pattern, Spanned, Statement};
use cairo_m_compiler_semantic::definition::DefinitionKind;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;

use crate::instruction::CalleeSignature;
use crate::{Instruction, InstructionKind, MirType, Terminator, Value};

use super::builder::MirBuilder;
use super::expr::LowerExpr;

/// Trait for lowering statements to MIR
pub trait LowerStmt<'a> {
    fn lower_statement(&mut self, stmt: &Spanned<Statement>) -> Result<(), String>;
}

impl<'a, 'db> LowerStmt<'a> for MirBuilder<'a, 'db> {
    /// Lowers a single statement into MIR instructions by dispatching to a helper for each statement
    /// type.
    fn lower_statement(&mut self, stmt: &Spanned<Statement>) -> Result<(), String> {
        match stmt.value() {
            Statement::Let { pattern, value, .. } => self.lower_let_statement(pattern, value),
            Statement::Return { value } => self.lower_return_statement(value),
            Statement::Assignment { lhs, rhs } => self.lower_assignment_statement(lhs, rhs),
            Statement::Expression(expr) => self.lower_expression_statement(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
            } => self.lower_if_statement(condition, then_block, else_block.as_deref()),
            Statement::Block(statements) => self.lower_block_statement(statements),
            Statement::While { condition, body } => self.lower_while_statement(condition, body),
            Statement::Loop { body } => self.lower_loop_statement(body),
            Statement::For {
                init,
                condition,
                step,
                body,
            } => self.lower_for_statement(init, condition, step, body),
            Statement::Break => self.lower_break_statement(),
            Statement::Continue => self.lower_continue_statement(),
            Statement::Const(_) => self.lower_const_statement(),
        }
    }
}

// Placeholder implementations - these will be filled in during the actual refactoring
impl<'a, 'db> MirBuilder<'a, 'db> {
    pub(super) fn lower_let_statement(
        &mut self,
        pattern: &Pattern,
        value: &Spanned<Expression>,
    ) -> Result<(), String> {
        // Get the scope from the value expression (the let statement is in the same scope as its value)
        let expr_id = self
            .semantic_index
            .expression_id_by_span(value.span())
            .ok_or_else(|| {
                format!(
                    "MIR: No ExpressionId found for value expression span {:?}",
                    value.span()
                )
            })?;
        let expr_info = self
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for value expression ID {expr_id:?}"))?;
        let scope_id = expr_info.scope_id;

        // Check for optimizable tuple destructuring pattern
        match (pattern, value.value()) {
            // Direct tuple destructuring optimization - skip intermediate tuple allocation
            (Pattern::Tuple(names), Expression::Tuple(elements))
                if names.len() == elements.len() =>
            {
                for (name, element_expr) in names.iter().zip(elements.iter()) {
                    if let Some((def_idx, _)) = self
                        .semantic_index
                        .resolve_name_to_definition(name.value(), scope_id)
                    {
                        let def_id = DefinitionId::new(self.db, self.file, def_idx);
                        let mir_def_id = self.convert_definition_id(def_id);

                        // Check if this variable is used
                        let is_used =
                            if let Some(definition) = self.semantic_index.definition(def_idx) {
                                if let Some(place_table) =
                                    self.semantic_index.place_table(definition.scope_id)
                                {
                                    if let Some(place) = place_table.place(definition.place_id) {
                                        place.is_used()
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                        if is_used {
                            // Lower the element expression directly
                            let element_value = self.lower_expression(element_expr)?;

                            // Get the type from semantic analysis
                            let semantic_type =
                                definition_semantic_type(self.db, self.crate_id, def_id);
                            let element_mir_type =
                                MirType::from_semantic_type(self.db, semantic_type);

                            // Allocate space for the variable
                            let var_addr = self
                                .mir_function
                                .new_typed_value_id(MirType::pointer(element_mir_type.clone()));
                            self.add_instruction(Instruction::stack_alloc(
                                var_addr,
                                element_mir_type.size_units(),
                            ));

                            match element_mir_type {
                                MirType::U32 => {
                                    self.add_instruction(Instruction::store_u32(
                                        Value::operand(var_addr),
                                        element_value,
                                    ));
                                }
                                MirType::Felt | MirType::Bool => {
                                    self.add_instruction(Instruction::store(
                                        Value::operand(var_addr),
                                        element_value,
                                    ));
                                }
                                _ => {
                                    self.add_instruction(Instruction::store(
                                        Value::operand(var_addr),
                                        element_value,
                                    ));
                                }
                            }
                            self.definition_to_value.insert(mir_def_id, var_addr);
                        } else {
                            // Unused variable - still evaluate for side effects but don't store
                            let _ = self.lower_expression(element_expr)?;
                            let dummy_addr = self.mir_function.new_value_id();
                            self.definition_to_value.insert(mir_def_id, dummy_addr);
                        }
                    } else {
                        return Err(format!(
                            "Failed to resolve variable '{}' in scope {:?}",
                            name.value(),
                            scope_id
                        ));
                    }
                }
                return Ok(());
            }
            (Pattern::Tuple(names), Expression::FunctionCall { callee, args }) => {
                // Attempt to apply `let (a,b) = f()` optimization.
                let res = (|| -> Result<bool, String> {
                    // Check if callee is a simple identifier.
                    let Expression::Identifier(func_name) = callee.value() else {
                        return Ok(false);
                    };

                    // Get semantic info for the callee.
                    let callee_expr_id = self
                        .semantic_index
                        .expression_id_by_span(callee.span())
                        .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for callee span {:?}",
                            callee.span()
                        )
                    })?;
                    let callee_expr_info = self
                        .semantic_index
                        .expression(callee_expr_id)
                        .ok_or_else(|| {
                            format!("MIR: No ExpressionInfo for callee ID {callee_expr_id:?}")
                        })?;

                    // Resolve function definition and get its MIR ID.
                    let Some((local_def_idx, local_def)) = self
                        .semantic_index
                        .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                    else {
                        return Ok(false);
                    };

                    // Handle function resolution: local functions vs imported functions
                    let func_id = match &local_def.kind {
                        DefinitionKind::Function(_) => {
                            // Local function - use current file
                            let func_def_id = DefinitionId::new(self.db, self.file, local_def_idx);
                            if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
                                *func_id
                            } else {
                                return Ok(false);
                            }
                        }
                        DefinitionKind::Use(use_ref) => {
                            // Imported function - follow the import chain
                            match self.resolve_imported_function(
                                use_ref.imported_module.value(),
                                func_name.value(),
                            ) {
                                Some(func_id) => func_id,
                                None => {
                                    return Ok(false);
                                }
                            }
                        }
                        _ => {
                            return Ok(false);
                        }
                    };

                    // Check that the function call returns a tuple of the correct arity.
                    let func_call_semantic_type =
                        expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                    let TypeData::Tuple(element_types) = func_call_semantic_type.data(self.db)
                    else {
                        return Ok(false);
                    };
                    if element_types.len() != names.len() {
                        return Ok(false);
                    }

                    // Apply the optimization.
                    let arg_values = args
                        .iter()
                        .map(|arg| self.lower_expression(arg))
                        .collect::<Result<Vec<_>, _>>()?;

                    let mut dests = Vec::new();
                    for (i, name) in names.iter().enumerate() {
                        let (def_idx, _) = self
                            .semantic_index
                            .resolve_name_to_definition(name.value(), scope_id)
                            .ok_or_else(|| {
                                format!(
                                    "Failed to resolve variable '{}' in scope {:?}",
                                    name.value(),
                                    scope_id
                                )
                            })?;

                        let def_id = DefinitionId::new(self.db, self.file, def_idx);
                        let mir_def_id = self.convert_definition_id(def_id);

                        let is_used = self
                            .semantic_index
                            .definition(def_idx)
                            .and_then(|def| {
                                self.semantic_index
                                    .place_table(def.scope_id)
                                    .and_then(|pt| pt.place(def.place_id))
                            })
                            .map(|p| p.is_used())
                            .unwrap_or(true);

                        let elem_type = element_types[i];
                        let elem_mir_type = MirType::from_semantic_type(self.db, elem_type);
                        let dest = self.mir_function.new_typed_value_id(elem_mir_type);
                        dests.push(dest);

                        if is_used {
                            self.definition_to_value.insert(mir_def_id, dest);
                        } else {
                            let dummy_addr = self.mir_function.new_value_id();
                            self.definition_to_value.insert(mir_def_id, dummy_addr);
                        }
                    }

                    // Get the function signature
                    let (param_types, return_types) = self.get_function_signature(func_id)?;

                    // Create the CalleeSignature
                    let signature = CalleeSignature {
                        param_types,
                        return_types,
                    };

                    // Create the call instruction with the signature
                    let mut call_instr = Instruction::call(dests, func_id, arg_values);
                    if let InstructionKind::Call {
                        signature: ref mut sig,
                        ..
                    } = &mut call_instr.kind
                    {
                        *sig = signature;
                    }
                    self.add_instruction(call_instr);

                    Ok(true)
                })();

                match res {
                    Ok(true) => return Ok(()),
                    Ok(false) => { /* Fallthrough to generic handling */ }
                    Err(e) => return Err(e),
                }
            }
            _ => {}
        }

        // Check for binary operation optimization before falling back to normal processing
        if let Pattern::Identifier(name) = pattern {
            if let Expression::BinaryOp { op, left, right } = value.value() {
                // Binary operation optimization for let statements
                if let Some((def_idx, _)) = self
                    .semantic_index
                    .resolve_name_to_definition(name.value(), scope_id)
                {
                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Check if this variable is actually used
                    let is_used = if let Some(definition) = self.semantic_index.definition(def_idx)
                    {
                        if let Some(place_table) =
                            self.semantic_index.place_table(definition.scope_id)
                        {
                            if let Some(place) = place_table.place(definition.place_id) {
                                place.is_used()
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    };

                    if is_used {
                        // Lower the operands
                        let left_value = self.lower_expression(left)?;
                        let right_value = self.lower_expression(right)?;

                        // Get the variable type
                        let semantic_type =
                            definition_semantic_type(self.db, self.crate_id, def_id);
                        let var_type = MirType::from_semantic_type(self.db, semantic_type);

                        // Allocate storage for the variable
                        let var_storage = self.mir_function.new_typed_value_id(var_type.clone());
                        self.add_instruction(Instruction::stack_alloc(
                            var_storage,
                            var_type.size_units(),
                        ));

                        // Generate single binary operation directly to allocated storage
                        let typed_op = self.convert_binary_op(*op, left, right);
                        self.add_instruction(Instruction::binary_op(
                            typed_op,
                            var_storage,
                            left_value,
                            right_value,
                        ));

                        // Map the variable to its storage ValueId
                        self.definition_to_value.insert(mir_def_id, var_storage);
                    } else {
                        // For unused variables, still evaluate operands for side effects but don't store
                        let _ = self.lower_expression(left)?;
                        let _ = self.lower_expression(right)?;
                        let dummy_addr = self.mir_function.new_value_id();
                        self.definition_to_value.insert(mir_def_id, dummy_addr);
                    }
                    return Ok(());
                }
            }
        }

        // Fall back to normal processing for non-optimizable cases
        let rhs_value = self.lower_expression(value)?;

        match pattern {
            Pattern::Identifier(name) => {
                // Single identifier pattern
                if let Some((def_idx, _)) = self
                    .semantic_index
                    .resolve_name_to_definition(name.value(), scope_id)
                {
                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Check if the RHS is already a stack-allocated aggregate.
                    if let Expression::StructLiteral { .. } | Expression::Tuple(_) = value.value() {
                        if let Value::Operand(addr) = rhs_value {
                            // The RHS expression already allocated the object and returned its address.
                            self.definition_to_value.insert(mir_def_id, addr);
                        } else {
                            return Err("Expected an address from aggregate literal".to_string());
                        }
                    } else {
                        // Check if this variable is actually used
                        let is_used =
                            if let Some(definition) = self.semantic_index.definition(def_idx) {
                                if let Some(place_table) =
                                    self.semantic_index.place_table(definition.scope_id)
                                {
                                    if let Some(place) = place_table.place(definition.place_id) {
                                        place.is_used()
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                        if is_used {
                            let semantic_type =
                                definition_semantic_type(self.db, self.crate_id, def_id);
                            let var_type = MirType::from_semantic_type(self.db, semantic_type);
                            let var_addr = self
                                .mir_function
                                .new_typed_value_id(MirType::pointer(var_type.clone()));
                            self.add_instruction(Instruction::stack_alloc(
                                var_addr,
                                var_type.size_units(),
                            ));

                            match var_type {
                                MirType::U32 => {
                                    self.add_instruction(Instruction::store_u32(
                                        Value::operand(var_addr),
                                        rhs_value,
                                    ));
                                }
                                MirType::Felt | MirType::Bool => {
                                    self.add_instruction(Instruction::store(
                                        Value::operand(var_addr),
                                        rhs_value,
                                    ));
                                }
                                _ => {
                                    self.add_instruction(Instruction::store(
                                        Value::operand(var_addr),
                                        rhs_value,
                                    ));
                                }
                            }
                            self.definition_to_value.insert(mir_def_id, var_addr);
                        } else {
                            let dummy_addr = self.mir_function.new_value_id();
                            self.definition_to_value.insert(mir_def_id, dummy_addr);
                        }
                    }
                } else {
                    return Err(format!(
                        "Failed to resolve variable '{}' in scope {:?}",
                        name.value(),
                        scope_id
                    ));
                }
            }
            Pattern::Tuple(names) => {
                // Tuple destructuring pattern for non-literal tuples
                let rhs_semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);

                match rhs_semantic_type.data(self.db) {
                    TypeData::Tuple(element_types) => {
                        if element_types.len() != names.len() {
                            return Err(format!(
                                "Tuple pattern has {} elements but value has {} elements",
                                names.len(),
                                element_types.len()
                            ));
                        }

                        // For non-literal tuples, we need to extract from the tuple address
                        if let Value::Operand(tuple_addr) = rhs_value {
                            // Extract each element from consecutive memory locations
                            for (index, name) in names.iter().enumerate() {
                                if let Some((def_idx, _)) = self
                                    .semantic_index
                                    .resolve_name_to_definition(name.value(), scope_id)
                                {
                                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                                    let mir_def_id = self.convert_definition_id(def_id);

                                    let is_used = if let Some(definition) =
                                        self.semantic_index.definition(def_idx)
                                    {
                                        if let Some(place_table) =
                                            self.semantic_index.place_table(definition.scope_id)
                                        {
                                            if let Some(place) =
                                                place_table.place(definition.place_id)
                                            {
                                                place.is_used()
                                            } else {
                                                true
                                            }
                                        } else {
                                            true
                                        }
                                    } else {
                                        true
                                    };

                                    if is_used {
                                        // Get the element type
                                        let element_type = element_types[index];
                                        let element_mir_type =
                                            MirType::from_semantic_type(self.db, element_type);

                                        // Get pointer to tuple element
                                        let elem_ptr = self.mir_function.new_typed_value_id(
                                            MirType::pointer(element_mir_type.clone()),
                                        );
                                        self.add_instruction(
                                            Instruction::get_element_ptr(
                                                elem_ptr,
                                                Value::operand(tuple_addr),
                                                Value::integer(index as i32),
                                            )
                                            .with_comment(format!(
                                                "Get address of tuple element {}",
                                                index
                                            )),
                                        );

                                        // Load the element
                                        let elem_value = self
                                            .mir_function
                                            .new_typed_value_id(element_mir_type.clone());
                                        self.add_instruction(
                                            Instruction::load(elem_value, Value::operand(elem_ptr))
                                                .with_comment(format!(
                                                    "Load tuple element {}",
                                                    index
                                                )),
                                        );

                                        // Allocate space for the variable
                                        let var_addr = self.mir_function.new_typed_value_id(
                                            MirType::pointer(element_mir_type.clone()),
                                        );
                                        self.add_instruction(Instruction::stack_alloc(
                                            var_addr,
                                            element_mir_type.size_units(),
                                        ));

                                        // Store the loaded value
                                        self.add_instruction(Instruction::store(
                                            Value::operand(var_addr),
                                            Value::operand(elem_value),
                                        ));

                                        self.definition_to_value.insert(mir_def_id, var_addr);
                                    } else {
                                        let dummy_addr = self.mir_function.new_value_id();
                                        self.definition_to_value.insert(mir_def_id, dummy_addr);
                                    }
                                } else {
                                    return Err(format!(
                                        "Failed to resolve variable '{}' in scope {:?}",
                                        name.value(),
                                        scope_id
                                    ));
                                }
                            }
                        } else {
                            return Err("Tuple destructuring from non-operand expressions not yet supported".to_string());
                        }
                    }
                    _ => {
                        return Err(
                            "Cannot destructure non-tuple type in tuple pattern".to_string()
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn lower_return_statement(
        &mut self,
        value: &Option<Spanned<Expression>>,
    ) -> Result<(), String> {
        let terminator = if let Some(expr) = value {
            // Check if we're returning a tuple literal
            if let Expression::Tuple(elements) = expr.value() {
                // Lower each element of the tuple
                let mut return_values = Vec::new();
                for element in elements {
                    return_values.push(self.lower_expression(element)?);
                }
                Terminator::Return {
                    values: return_values,
                }
            } else {
                // Check if the expression type is a tuple
                let expr_id = self
                    .semantic_index
                    .expression_id_by_span(expr.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for return expression span {:?}",
                            expr.span()
                        )
                    })?;
                let expr_semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);

                // Check if it's a tuple type
                if let TypeData::Tuple(element_types) = expr_semantic_type.data(self.db) {
                    // We're returning a tuple variable - need to extract each element
                    let tuple_addr = self.lower_lvalue_expression(expr)?;
                    let mut return_values = Vec::new();

                    // Load each element from the tuple (stored consecutively)
                    for (i, elem_type) in element_types.iter().enumerate() {
                        let mir_type = MirType::from_semantic_type(self.db, *elem_type);

                        // Tuples are stored as consecutive values, so offset is just the index
                        let offset = i;

                        // Get element pointer
                        let elem_ptr = self
                            .mir_function
                            .new_typed_value_id(MirType::pointer(mir_type.clone()));
                        self.add_instruction(
                            Instruction::get_element_ptr(
                                elem_ptr,
                                tuple_addr,
                                Value::integer(offset as i32),
                            )
                            .with_comment(format!("Get address of tuple element {}", i)),
                        );

                        // Load the element
                        let elem_value = self.mir_function.new_typed_value_id(mir_type);
                        self.add_instruction(
                            Instruction::load(elem_value, Value::operand(elem_ptr))
                                .with_comment(format!("Load tuple element {}", i)),
                        );

                        return_values.push(Value::operand(elem_value));
                    }

                    Terminator::Return {
                        values: return_values,
                    }
                } else {
                    // Single value return
                    let return_value = self.lower_expression(expr)?;
                    Terminator::Return {
                        values: vec![return_value],
                    }
                }
            }
        } else {
            Terminator::Return { values: vec![] }
        };

        self.terminate_current_block(terminator);
        self.is_terminated = true;
        Ok(())
    }

    pub(super) fn lower_assignment_statement(
        &mut self,
        lhs: &Spanned<Expression>,
        rhs: &Spanned<Expression>,
    ) -> Result<(), String> {
        // Check if RHS is a binary operation that we can optimize
        match rhs.value() {
            Expression::BinaryOp { op, left, right } => {
                // Optimization: Generate direct assignment with binary operation
                // Instead of: temp = left op right; lhs = temp
                // Generate: lhs = left op right (single instruction)

                // Lower the left and right operands separately
                let left_value = self.lower_expression(left)?;
                let right_value = self.lower_expression(right)?;

                // Get the expression ID for the RHS binary operation to get type information
                let rhs_expr_id = self
                    .semantic_index
                    .expression_id_by_span(rhs.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for RHS binary op span {:?}",
                            rhs.span()
                        )
                    })?;

                // Query semantic type system for result type
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, rhs_expr_id, None);
                let result_type = MirType::from_semantic_type(self.db, semantic_type);

                // Try to get the LHS ValueId directly if it's a simple identifier
                let lhs_value_id = if let Expression::Identifier(name) = lhs.value() {
                    // Get the expression info for the LHS to get its scope
                    let lhs_expr_id = self
                        .semantic_index
                        .expression_id_by_span(lhs.span())
                        .ok_or_else(|| {
                            format!("MIR: No ExpressionId found for LHS span {:?}", lhs.span())
                        })?;
                    let lhs_expr_info =
                        self.semantic_index.expression(lhs_expr_id).ok_or_else(|| {
                            format!("MIR: No ExpressionInfo for LHS ID {lhs_expr_id:?}")
                        })?;

                    // Resolve the identifier to a definition
                    if let Some((def_idx, _)) = self
                        .semantic_index
                        .resolve_name_to_definition(name.value(), lhs_expr_info.scope_id)
                    {
                        let def_id = DefinitionId::new(self.db, self.file, def_idx);
                        let mir_def_id = self.convert_definition_id(def_id);

                        // Look up the ValueId for this definition
                        self.definition_to_value.get(&mir_def_id).copied()
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(dest_id) = lhs_value_id {
                    // Generate single binary operation instruction directly to LHS
                    let typed_op = self.convert_binary_op(*op, left, right);
                    self.add_instruction(Instruction::binary_op(
                        typed_op,
                        dest_id,
                        left_value,
                        right_value,
                    ));
                } else {
                    // Fall back to two-instruction approach for complex LHS expressions
                    let dest = self.mir_function.new_typed_value_id(result_type);
                    let typed_op = self.convert_binary_op(*op, left, right);
                    self.add_instruction(Instruction::binary_op(
                        typed_op,
                        dest,
                        left_value,
                        right_value,
                    ));
                    let lhs_address = self.lower_lvalue_expression(lhs)?;
                    self.add_instruction(Instruction::store(lhs_address, Value::operand(dest)));
                }
            }
            _ => {
                // Standard assignment: lower RHS then store to LHS
                let rhs_value = self.lower_expression(rhs)?;
                let lhs_address = self.lower_lvalue_expression(lhs)?;
                self.add_instruction(Instruction::store(lhs_address, rhs_value));
            }
        }
        Ok(())
    }

    pub(super) fn lower_expression_statement(
        &mut self,
        expr: &Spanned<Expression>,
    ) -> Result<(), String> {
        // For statement expressions, check if it's a function call that should be void
        if let Expression::FunctionCall { callee, args } = expr.value() {
            // Handle function calls as statements (void calls)
            if let Expression::Identifier(func_name) = callee.value() {
                // Get the scope for the callee from its expression info
                let expr_id = self
                    .semantic_index
                    .expression_id_by_span(expr.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for statement expression span {:?}",
                            expr.span()
                        )
                    })?;
                let expr_info = self.semantic_index.expression(expr_id).ok_or_else(|| {
                    format!("MIR: No ExpressionInfo for statement expression ID {expr_id:?}")
                })?;

                if let Some((local_def_idx, local_def)) = self
                    .semantic_index
                    .resolve_name_to_definition(func_name.value(), expr_info.scope_id)
                {
                    // Handle function resolution: local functions vs imported functions
                    let func_id = match &local_def.kind {
                        DefinitionKind::Function(_) => {
                            // Local function - use current file
                            let func_def_id = DefinitionId::new(self.db, self.file, local_def_idx);
                            if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
                                *func_id
                            } else {
                                // Local function not found in mapping, return error
                                return Ok(());
                            }
                        }
                        DefinitionKind::Use(use_ref) => {
                            // Imported function - follow the import chain
                            match self.resolve_imported_function(
                                use_ref.imported_module.value(),
                                func_name.value(),
                            ) {
                                Some(func_id) => func_id,
                                None => {
                                    // Import resolution failed, return error
                                    return Ok(());
                                }
                            }
                        }
                        _ => {
                            // Neither function nor import, return error
                            return Ok(());
                        }
                    };

                    {
                        // Lower arguments
                        let mut arg_values = Vec::new();
                        for arg in args {
                            arg_values.push(self.lower_expression(arg)?);
                        }

                        // Check the function's return type
                        let func_expr_semantic_type = expression_semantic_type(
                            self.db,
                            self.crate_id,
                            self.file,
                            expr_id,
                            None,
                        );

                        if let TypeData::Tuple(element_types) =
                            func_expr_semantic_type.data(self.db)
                        {
                            // Function returns a tuple - create destinations but don't use them
                            let mut dests = Vec::new();
                            for elem_type in element_types {
                                let mir_type = MirType::from_semantic_type(self.db, elem_type);
                                dests.push(self.mir_function.new_typed_value_id(mir_type));
                            }
                            // Get the function signature
                            let (param_types, return_types) =
                                self.get_function_signature(func_id)?;

                            // Create the CalleeSignature
                            let signature = CalleeSignature {
                                param_types,
                                return_types,
                            };

                            // Create the call instruction with the signature
                            let mut call_instr = Instruction::call(dests, func_id, arg_values);
                            if let InstructionKind::Call {
                                signature: ref mut sig,
                                ..
                            } = &mut call_instr.kind
                            {
                                *sig = signature;
                            }
                            self.add_instruction(call_instr);
                        } else if let TypeData::Tuple(types) =
                            func_expr_semantic_type.data(self.db)
                            && types.is_empty()
                        {
                            // Function returns unit/void
                            let (param_types, return_types) =
                                self.get_function_signature(func_id)?;
                            let signature = CalleeSignature {
                                param_types,
                                return_types,
                            };
                            self.add_instruction(Instruction::void_call(
                                func_id, arg_values, signature,
                            ));
                        } else {
                            // Function returns a single value - create a destination but don't use it
                            let return_type =
                                MirType::from_semantic_type(self.db, func_expr_semantic_type);
                            let dest = self.mir_function.new_typed_value_id(return_type);
                            // Get the function signature
                            let (param_types, return_types) =
                                self.get_function_signature(func_id)?;

                            // Create the CalleeSignature
                            let signature = CalleeSignature {
                                param_types,
                                return_types,
                            };

                            // Create the call instruction with the signature
                            let mut call_instr = Instruction::call(vec![dest], func_id, arg_values);
                            if let InstructionKind::Call {
                                signature: ref mut sig,
                                ..
                            } = &mut call_instr.kind
                            {
                                *sig = signature;
                            }
                            self.add_instruction(call_instr);
                        }
                        return Ok(());
                    }
                }
            }
        }

        // For other statement expressions, lower normally and discard the result
        self.lower_expression(expr)?;
        Ok(())
    }

    pub(super) fn lower_if_statement(
        &mut self,
        condition: &Spanned<Expression>,
        then_block: &Spanned<Statement>,
        else_block: Option<&Spanned<Statement>>,
    ) -> Result<(), String> {
        // Lower the condition expression
        let condition_value = self.lower_expression(condition)?;

        // Create the then block
        let then_block_id = self.mir_function.add_basic_block();

        // Keep track of the final blocks from each branch that might need to be connected to the merge block
        let mut final_blocks = Vec::new();

        if let Some(else_stmt) = else_block {
            // There is an else block - create separate blocks for then and else
            let else_block_id = self.mir_function.add_basic_block();

            // Terminate the current block with a conditional branch
            self.terminate_current_block(Terminator::branch(
                condition_value,
                then_block_id,
                else_block_id,
            ));

            // Lower the then block
            self.current_block_id = then_block_id;
            self.lower_statement(then_block)?;

            // Check if the then branch terminated
            if !self.current_block().is_terminated() {
                final_blocks.push(self.current_block_id);
            }

            // Lower the else block
            self.current_block_id = else_block_id;
            self.lower_statement(else_stmt)?;

            // Check if the else branch terminated
            if !self.current_block().is_terminated() {
                final_blocks.push(self.current_block_id);
            }
        } else {
            // No else block - optimize by branching directly to merge block
            let merge_block_id = self.mir_function.add_basic_block();

            // Terminate the current block with a conditional branch
            // If condition is true, go to then_block, otherwise go directly to merge_block
            self.terminate_current_block(Terminator::branch(
                condition_value,
                then_block_id,
                merge_block_id,
            ));

            // Lower the then block
            self.current_block_id = then_block_id;
            self.lower_statement(then_block)?;

            // Check if the then branch terminated
            if !self.current_block().is_terminated() {
                final_blocks.push(self.current_block_id);
            }

            // The merge block always gets control flow from the false condition
            // Continue generating code in the merge block
            self.current_block_id = merge_block_id;

            // Connect any non-terminated branches to the merge block
            for block_id in final_blocks {
                // Temporarily switch to the block needing termination
                let block_to_terminate = self.mir_function.get_basic_block_mut(block_id).unwrap();
                block_to_terminate.set_terminator(Terminator::jump(merge_block_id));
            }

            return Ok(());
        }

        // Create and connect the merge block (if needed) - for the else case
        if final_blocks.is_empty() {
            // All paths through the if-else ended in a terminator.
            // The current control flow path is now terminated.
            self.is_terminated = true;
        } else {
            // At least one branch needs to continue. Create the merge block.
            let merge_block_id = self.mir_function.add_basic_block();

            // Connect all non-terminated branches to the merge block
            for block_id in final_blocks {
                // Temporarily switch to the block needing termination
                let block_to_terminate = self.mir_function.get_basic_block_mut(block_id).unwrap();
                block_to_terminate.set_terminator(Terminator::jump(merge_block_id));
            }

            // Continue generating code in the new merge block
            self.current_block_id = merge_block_id;
        }

        Ok(())
    }

    pub(super) fn lower_block_statement(
        &mut self,
        statements: &[Spanned<Statement>],
    ) -> Result<(), String> {
        // Lower all statements in the block sequentially
        for stmt in statements {
            self.lower_statement(stmt)?;

            // If a statement terminates the block (like return), stop processing
            if self.current_block().is_terminated() {
                break;
            }
        }
        Ok(())
    }

    pub(super) fn lower_while_statement(
        &mut self,
        condition: &Spanned<Expression>,
        body: &Spanned<Statement>,
    ) -> Result<(), String> {
        // While Loop Pattern:
        // entry:
        //     jump loop_header
        // loop_header:
        //     %cond = evaluate_condition
        //     if %cond then loop_body else exit
        // loop_body:
        //     ... body statements ...
        //     jump loop_header
        // exit:
        //     ... continue after loop ...

        // Create the necessary blocks
        let loop_header = self.mir_function.add_basic_block();
        let loop_body = self.mir_function.add_basic_block();
        let loop_exit = self.mir_function.add_basic_block();

        // Jump to the loop header from the current block
        self.terminate_current_block(Terminator::jump(loop_header));

        // Push loop context for break/continue
        self.loop_stack.push((loop_header, loop_exit));

        // Generate the loop header block - evaluate condition and branch
        self.current_block_id = loop_header;
        let condition_value = self.lower_expression(condition)?;
        self.terminate_current_block(Terminator::branch(condition_value, loop_body, loop_exit));

        // Generate the loop body
        self.current_block_id = loop_body;
        self.lower_statement(body)?;

        // If the body didn't terminate, jump back to the header
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::jump(loop_header));
        }

        // Pop loop context
        self.loop_stack.pop();

        // Continue in the exit block
        self.current_block_id = loop_exit;
        Ok(())
    }

    pub(super) fn lower_loop_statement(&mut self, body: &Spanned<Statement>) -> Result<(), String> {
        // Infinite Loop Pattern:
        // entry:
        //     jump loop_body
        // loop_body:
        //     ... body statements ...
        //     jump loop_body  // or exit via break
        // exit:
        //     ... continue after loop ...

        // Create the necessary blocks
        let loop_body = self.mir_function.add_basic_block();
        let loop_exit = self.mir_function.add_basic_block();

        // Jump to the loop body from the current block
        self.terminate_current_block(Terminator::jump(loop_body));

        // Push loop context for break/continue
        // For infinite loops, the header is the body itself
        self.loop_stack.push((loop_body, loop_exit));

        // Generate the loop body
        self.current_block_id = loop_body;
        self.lower_statement(body)?;

        // If the body didn't terminate, jump back to itself
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::jump(loop_body));
        }

        // Pop loop context
        self.loop_stack.pop();

        // Continue in the exit block
        self.current_block_id = loop_exit;
        Ok(())
    }

    pub(super) fn lower_for_statement(
        &mut self,
        init: &Spanned<Statement>,
        condition: &Spanned<Expression>,
        step: &Spanned<Statement>,
        body: &Spanned<Statement>,
    ) -> Result<(), String> {
        // 1. Initialization runs once in the current block
        self.lower_statement(init)?;
        if self.current_block().is_terminated() {
            // Init somehow terminated control flow — nothing more to do.
            return Ok(());
        }

        // 2. Create loop blocks
        let loop_header = self.mir_function.add_basic_block(); // evaluates condition
        let loop_body = self.mir_function.add_basic_block(); // body of the loop
        let loop_step = self.mir_function.add_basic_block(); // step statement
        let loop_exit = self.mir_function.add_basic_block(); // code after the loop

        // Jump from current block to condition header
        self.terminate_current_block(Terminator::jump(loop_header));

        // Push loop context: for `continue`, jump to step; for `break`, jump to exit
        self.loop_stack.push((loop_step, loop_exit));

        // 3. Header: evaluate condition and branch
        self.current_block_id = loop_header;
        let cond_val = self.lower_expression(condition)?;
        self.terminate_current_block(Terminator::branch(cond_val, loop_body, loop_exit));

        // 4. Body: generate code, then jump to step if not terminated
        self.current_block_id = loop_body;
        self.lower_statement(body)?;
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::jump(loop_step));
        }

        // 5. Step: execute step statement, then jump back to header
        self.current_block_id = loop_step;
        self.lower_statement(step)?;
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::jump(loop_header));
        }

        // 6. Pop loop context and continue in exit block
        self.loop_stack.pop();
        self.current_block_id = loop_exit;

        Ok(())
    }

    pub(super) fn lower_break_statement(&mut self) -> Result<(), String> {
        if let Some((_, loop_exit)) = self.loop_stack.last() {
            // Jump to the exit block of the current loop
            self.terminate_current_block(Terminator::jump(*loop_exit));
            self.is_terminated = true;
            Ok(())
        } else {
            Err("'break' statement outside of loop".to_string())
        }
    }

    pub(super) fn lower_continue_statement(&mut self) -> Result<(), String> {
        if let Some((continue_target, _)) = self.loop_stack.last() {
            // Jump to the continue target of the current loop
            // For while/infinite loops: this is the loop header (condition check)
            // For for-loops: this is the step block (increment statement)
            self.terminate_current_block(Terminator::jump(*continue_target));
            self.is_terminated = true;
            Ok(())
        } else {
            Err("'continue' statement outside of loop".to_string())
        }
    }

    pub(super) const fn lower_const_statement(&self) -> Result<(), String> {
        // Constants are handled during semantic analysis, skip in MIR generation
        Ok(())
    }
}
