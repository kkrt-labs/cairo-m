//! # Statement Lowering
//!
//! This module contains the trait and implementations for lowering statements
//! from the AST to MIR instructions.

use cairo_m_compiler_parser::parser::{Expression, Pattern, Spanned, Statement};
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;

use crate::{Instruction, MirType, Terminator, Value};

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
        let expr_id = self.expr_id(value.span())?;
        let expr_info =
            self.ctx.semantic_index.expression(expr_id).ok_or_else(|| {
                format!("MIR: No ExpressionInfo for value expression ID {expr_id:?}")
            })?;
        let scope_id = expr_info.scope_id;

        // Lower the expression and bind to pattern using the generic pattern lowering
        let rhs_value = self.lower_expression(value)?;
        self.lower_pattern(pattern, rhs_value, scope_id)?;
        Ok(())
    }

    pub(super) fn lower_return_statement(
        &mut self,
        value: &Option<Spanned<Expression>>,
    ) -> Result<(), String> {
        // No return value - return unit
        if value.is_none() {
            self.terminate_with_return(vec![]);
            return Ok(());
        }

        let expr = value.as_ref().unwrap();
        // Check if the expression type is a tuple
        let expr_id = self.expr_id(expr.span())?;
        let expr_semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);

        // Check if it's a tuple type
        if let TypeData::Tuple(_) = expr_semantic_type.data(self.ctx.db) {
            let expr_value = self.lower_expression(expr)?;

            // We expect a tuple value
            if let Value::Operand(value_id) = expr_value {
                if let Some(MirType::Tuple(elem_types)) =
                    self.state.mir_function.get_value_type(value_id)
                {
                    // We have a tuple value - extract its elements for the return
                    // Clone elem_types to avoid borrow checker issues
                    let elem_types_cloned = elem_types.clone();
                    let mut return_values = Vec::new();
                    for (i, elem_type) in elem_types_cloned.iter().enumerate() {
                        let elem_value =
                            self.extract_tuple_element(expr_value, i, elem_type.clone());
                        return_values.push(Value::operand(elem_value));
                    }
                    self.terminate_with_return(return_values);
                    return Ok(());
                }
            }
            return Err(format!("Expected tuple value but got: {:?}", expr_value));
        } else {
            // Single value return
            let return_value = self.lower_expression(expr)?;
            self.terminate_with_return(vec![return_value]);
        }

        Ok(())
    }

    pub(super) fn lower_assignment_statement(
        &mut self,
        lhs: &Spanned<Expression>,
        rhs: &Spanned<Expression>,
    ) -> Result<(), String> {
        // Check if RHS is a binary operation that we can optimize
        // Check if LHS is a field assignment that can be optimized
        let lhs_expr_id = self.expr_id(lhs.span())?;
        let lhs_expr_info = self
            .ctx
            .semantic_index
            .expression(lhs_expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for LHS ID {lhs_expr_id:?}"))?;

        // Early out: plain identifier assignment => SSA rebind
        if let Expression::Identifier(ref name) = &lhs_expr_info.ast_node {
            let rhs_value = self.lower_expression(rhs)?;
            return self.bind_variable(name, lhs_expr_info.scope_id, rhs_value);
        }

        // Check if this is a field assignment (rect.width = value)
        if let Expression::MemberAccess { object, field } = &lhs_expr_info.ast_node {
            // Check if the object is a value-based variable (not memory-allocated)
            let struct_val = self.lower_expression(object)?;

            // Query semantic type system for the container (struct) type, not the field type
            let struct_type = self.expr_mir_type(object.span())?;
            let rhs_value = self.lower_expression(rhs)?;
            // Insert the field using the container (struct) type
            let new_struct_id =
                self.insert_struct_field(struct_val, field.value(), rhs_value, struct_type);
            let obj_scope = lhs_expr_info.scope_id;
            // Rebind the object to the new value for pure SSA form
            if let Expression::Identifier(obj_name) = object.value() {
                self.bind_variable(obj_name, obj_scope, Value::operand(new_struct_id))?;
            }
            return Ok(());
        }

        // Check if this is a tuple assignment (tuple.index  = value)
        if let Expression::TupleIndex { tuple, index } = &lhs_expr_info.ast_node {
            // Check if the tuple is a value-based variable (not memory-allocated)
            let tuple_val = self.lower_expression(tuple)?;
            let rhs_value = self.lower_expression(rhs)?;
            // Query semantic type system for the container (tuple) type, not the element type
            let tuple_type = self.expr_mir_type(tuple.span())?;
            let new_tuple_id = self.insert_tuple(tuple_val, *index, rhs_value, tuple_type);
            let obj_scope = lhs_expr_info.scope_id;
            // Rebind the object to the new value for pure SSA form
            if let Expression::Identifier(obj_name) = tuple.value() {
                self.bind_variable(obj_name, obj_scope, Value::operand(new_tuple_id))?;
            }
            return Ok(());
        }

        // TODO: Handle array assignment (array[index] = value)
        if let Expression::IndexAccess {
            array: _array,
            index: _index,
        } = &lhs_expr_info.ast_node
        {
            panic!("Array assignment not implemented");
        }

        // Standard assignment
        let rhs_type = self.expr_mir_type(rhs.span())?;
        let lhs_address = self.lower_expression(lhs)?;
        let rhs_value = self.lower_expression(rhs)?;
        self.instr().store(lhs_address, rhs_value, rhs_type);

        Ok(())
    }

    pub(super) fn lower_expression_statement(
        &mut self,
        expr: &Spanned<Expression>,
    ) -> Result<(), String> {
        // For statement expressions, check if it's a function call that should be void
        if let Expression::FunctionCall { callee, args } = expr.value() {
            // Handle function calls as statements (void calls)
            let expr_id = self.expr_id(expr.span())?;

            // Try to resolve the function using our helper
            if let Ok(func_id) = self.resolve_callee_expression(callee) {
                // Lower arguments
                let arg_values = args
                    .iter()
                    .map(|arg| self.lower_expression(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                // Use our helper to emit the call and discard results
                self.emit_call_and_discard_result(func_id, arg_values, expr_id)?;
                return Ok(());
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
        let then_block_id = self.create_block();

        // Keep track of the final blocks from each branch that might need to be connected to the merge block
        let mut final_blocks = Vec::new();

        if let Some(else_stmt) = else_block {
            // There is an else block - create separate blocks for then and else
            let else_block_id = self.create_block();

            // Terminate the current block with a conditional branch
            self.terminate_with_branch(condition_value, then_block_id, else_block_id);

            // Lower the then block
            self.switch_to_block(then_block_id);
            self.lower_statement(then_block)?;

            // Check if the then branch terminated
            if !self.is_current_block_terminated() {
                final_blocks.push(self.state.current_block_id);
            }

            // Lower the else block
            self.switch_to_block(else_block_id);
            self.lower_statement(else_stmt)?;

            // Check if the else branch terminated
            if !self.is_current_block_terminated() {
                final_blocks.push(self.state.current_block_id);
            }
        } else {
            // No else block - optimize by branching directly to merge block
            let merge_block_id = self.create_block();

            // Terminate the current block with a conditional branch
            // If condition is true, go to then_block, otherwise go directly to merge_block
            self.terminate_with_branch(condition_value, then_block_id, merge_block_id);

            // Lower the then block
            self.switch_to_block(then_block_id);
            self.lower_statement(then_block)?;

            // Check if the then branch terminated
            if !self.is_current_block_terminated() {
                final_blocks.push(self.state.current_block_id);
            }

            // The merge block always gets control flow from the false condition
            // Continue generating code in the merge block
            self.switch_to_block(merge_block_id);

            // Connect any non-terminated branches to the merge block
            for block_id in final_blocks {
                // Use a fresh cfg for each terminator setting
                let mut cfg = self.cfg();
                cfg.set_block_terminator(block_id, Terminator::jump(merge_block_id));
            }

            return Ok(());
        }

        // Create and connect the merge block (if needed) - for the else case
        if final_blocks.is_empty() {
            // All paths through the if-else ended in a terminator.
            // The current control flow path is now terminated.
            self.state.is_terminated = true;
        } else {
            // At least one branch needs to continue. Create the merge block.
            let merge_block_id = self.create_block();

            // Connect all non-terminated branches to the merge block
            for block_id in final_blocks {
                // Use a fresh cfg for each terminator setting
                let mut cfg = self.cfg();
                cfg.set_block_terminator(block_id, Terminator::jump(merge_block_id));
            }

            // Continue generating code in the new merge block
            self.switch_to_block(merge_block_id);
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
            if self.is_current_block_terminated() {
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

        // Create the necessary blocks using CfgBuilder convenience method
        let (loop_header, loop_body, loop_exit) = self.create_loop_blocks();

        // Jump to the loop header from the current block
        self.terminate_with_jump(loop_header);

        // Push loop context for break/continue
        self.state.loop_stack.push((loop_header, loop_exit));

        // Generate the loop header block - evaluate condition and branch
        self.switch_to_block(loop_header);
        let condition_value = self.lower_expression(condition)?;
        self.terminate_with_branch(condition_value, loop_body, loop_exit);

        // Generate the loop body
        self.switch_to_block(loop_body);
        self.lower_statement(body)?;

        // If the body didn't terminate, jump back to the header
        if !self.is_current_block_terminated() {
            self.terminate_with_jump(loop_header);
        }

        // Pop loop context
        self.state.loop_stack.pop();

        // Continue in the exit block
        self.switch_to_block(loop_exit);
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
        let loop_body = self.create_block();
        let loop_exit = self.create_block();

        // Jump to the loop body from the current block
        self.terminate_with_jump(loop_body);

        // Push loop context for break/continue
        // For infinite loops, the header is the body itself
        self.state.loop_stack.push((loop_body, loop_exit));

        // Generate the loop body
        self.switch_to_block(loop_body);
        self.lower_statement(body)?;

        // If the body didn't terminate, jump back to itself
        if !self.is_current_block_terminated() {
            self.terminate_with_jump(loop_body);
        }

        // Pop loop context
        self.state.loop_stack.pop();

        // Continue in the exit block
        self.switch_to_block(loop_exit);
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
        if self.is_current_block_terminated() {
            // Init somehow terminated control flow — nothing more to do.
            return Ok(());
        }

        // 2. Create loop blocks using CfgBuilder convenience method
        let (loop_header, loop_body, loop_step, loop_exit) = self.create_for_loop_blocks();

        // Jump from current block to condition header
        self.terminate_with_jump(loop_header);

        // Push loop context: for `continue`, jump to step; for `break`, jump to exit
        self.state.loop_stack.push((loop_step, loop_exit));

        // 3. Header: evaluate condition and branch
        self.switch_to_block(loop_header);
        let cond_val = self.lower_expression(condition)?;
        self.terminate_with_branch(cond_val, loop_body, loop_exit);

        // 4. Body: generate code, then jump to step if not terminated
        self.switch_to_block(loop_body);
        self.lower_statement(body)?;
        if !self.is_current_block_terminated() {
            self.terminate_with_jump(loop_step);
        }

        // 5. Step: execute step statement, then jump back to header
        self.switch_to_block(loop_step);
        self.lower_statement(step)?;
        if !self.is_current_block_terminated() {
            self.terminate_with_jump(loop_header);
        }

        // 6. Pop loop context and continue in exit block
        self.state.loop_stack.pop();
        self.switch_to_block(loop_exit);

        Ok(())
    }

    pub(super) fn lower_break_statement(&mut self) -> Result<(), String> {
        if let Some((_, loop_exit)) = self.state.loop_stack.last() {
            // Jump to the exit block of the current loop
            self.terminate_with_jump(*loop_exit);
            Ok(())
        } else {
            Err("'break' statement outside of loop".to_string())
        }
    }

    pub(super) fn lower_continue_statement(&mut self) -> Result<(), String> {
        if let Some((continue_target, _)) = self.state.loop_stack.last() {
            // Jump to the continue target of the current loop
            // For while/infinite loops: this is the loop header (condition check)
            // For for-loops: this is the step block (increment statement)
            self.terminate_with_jump(*continue_target);
            Ok(())
        } else {
            Err("'continue' statement outside of loop".to_string())
        }
    }

    pub(super) const fn lower_const_statement(&self) -> Result<(), String> {
        // Constants are handled during semantic analysis, skip in MIR generation
        Ok(())
    }

    /// Generic pattern binding for already-lowered values
    ///
    /// This is the fallback path that handles binding a lowered value to a pattern.
    /// Supports both identifier patterns and tuple destructuring patterns.
    fn lower_pattern(
        &mut self,
        pattern: &Pattern,
        rhs_value: Value,
        scope_id: FileScopeId,
    ) -> Result<(), String> {
        match pattern {
            Pattern::Identifier(name) => {
                // Simple identifier binding - use our new helper
                self.bind_variable(name, scope_id, rhs_value)?;
            }
            Pattern::Tuple(names) => {
                // Tuple destructuring from a value-based tuple
                let Value::Operand(tuple_value_id) = rhs_value else {
                    return Err(
                        "Tuple destructuring from non-operand expressions not yet supported"
                            .to_string(),
                    );
                };

                // Build the tuple type from element types
                let mut element_types = Vec::new();
                for name in names.iter() {
                    let (def_idx, _) = self
                        .ctx
                        .semantic_index
                        .resolve_name_to_definition(name.value(), scope_id)
                        .ok_or_else(|| {
                            format!(
                                "Failed to resolve variable '{}' in scope {:?}",
                                name.value(),
                                scope_id
                            )
                        })?;
                    let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
                    let semantic_type =
                        definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
                    element_types.push(MirType::from_semantic_type(self.ctx.db, semantic_type));
                }

                // Extract each element from the tuple value and bind to variables
                for (index, name) in names.iter().enumerate() {
                    let element_mir_type = element_types[index].clone();

                    // Extract the element using ExtractTupleElement instruction
                    let elem_value_id = self
                        .state
                        .mir_function
                        .new_typed_value_id(element_mir_type.clone());

                    self.instr()
                        .add_instruction(Instruction::extract_tuple_element(
                            elem_value_id,
                            Value::operand(tuple_value_id),
                            index,
                            element_mir_type,
                        ));

                    // Bind the extracted value to the variable using the unified helper
                    self.bind_variable(name, scope_id, Value::operand(elem_value_id))?;
                }
            }
        }
        Ok(())
    }
}
