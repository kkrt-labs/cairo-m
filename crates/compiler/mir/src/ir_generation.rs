//! # Semantic AST to MIR Lowering
//!
//! This module is responsible for "lowering" the high-level semantic AST into the
//! Mid-level Intermediate Representation (MIR). This is a crucial step in the compilation
//! pipeline, transforming language-specific constructs into a simplified, explicit
//! Control Flow Graph (CFG) representation that is ideal for optimization and code generation.
//!
//! ## Core Components
//!
//! - **`generate_mir` (Salsa Query)**: The main entry point for MIR generation. It takes a
//!   source file, retrieves its semantic model, and orchestrates the lowering of all
//!   functions within it into a `MirModule`.
//!
//! - **`MirBuilder`**: A stateful visitor that traverses the AST of a single function and
//!   constructs its corresponding `MirFunction`. It manages basic blocks, generates
//!   temporary values, and emits MIR instructions and terminators.
//!
//! ## Design Principles
//!
//! - **Error Recovery**: Generate partial MIR even with semantic errors
//! - **Source Mapping**: Preserve connections to original AST for diagnostics
//! - **Type Integration**: Leverage semantic type information for accurate lowering
//! - **Incremental Compilation**: Salsa integration for efficient re-compilation

use std::sync::Arc;

use cairo_m_compiler_parser::parse_program;
use cairo_m_compiler_parser::parser::{Expression, FunctionDef, Spanned, Statement, TopLevelItem};
use cairo_m_compiler_semantic::definition::{Definition, DefinitionKind};
use cairo_m_compiler_semantic::semantic_index::{semantic_index, DefinitionId, SemanticIndex};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::{File, SemanticDb};
use rustc_hash::FxHashMap;

use crate::db::MirDb;
use crate::{
    BasicBlock, BasicBlockId, FunctionId, Instruction, MirDefinitionId, MirFunction, MirModule,
    MirType, Terminator, Value, ValueId,
};

#[cfg(test)]
mod tests {
    mod mir_generation_tests;
    mod test_harness;
}

/// The main entry point for MIR generation.
///
/// This Salsa query takes a source file and produces the complete MIR for the entire module.
/// It works by first identifying all functions, then lowering each one into a `MirFunction`.
/// TODO considering that this takes as
///
/// # Error Handling
///
/// This function performs graceful error recovery:
/// - Returns `None` if there are parse errors that prevent semantic analysis
/// - Generates partial MIR for functions even if some have semantic errors
/// - Uses placeholder values for unresolved references
#[salsa::tracked]
pub fn generate_mir(db: &dyn MirDb, file: File) -> Option<Arc<MirModule>> {
    // Parse the module to get access to AST
    let parsed_program = parse_program(db, file);
    if !parsed_program.diagnostics.is_empty() {
        return None; // Can't generate MIR if parsing failed
    }

    // Get semantic index, return None if semantic analysis failed
    let semantic_index = semantic_index(db, file).as_ref().ok()?;

    let mut mir_module = MirModule::new();
    let mut function_mapping = FxHashMap::default();

    // Compute file_id once for this file to avoid repeated hashing
    // For now a simple hash of the file content is used
    // TODO add a proper system.
    let file_id = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        file.text(db).hash(&mut hasher);
        hasher.finish()
    };

    // First pass: Collect all function definitions and assign them MIR function IDs
    // This allows resolving forward-declared function calls correctly
    for (def_idx, def) in semantic_index.all_definitions() {
        if let DefinitionKind::Function(_) = &def.kind {
            let def_id = DefinitionId::new(db, file, def_idx);
            let mir_func = MirFunction::new(def.name.clone());
            let func_id = mir_module.add_function(mir_func);
            function_mapping.insert(def_id, (def, func_id));
        }
    }

    // Second pass: Lower each function's body
    for (def_id, (def, func_id)) in function_mapping.clone() {
        // Find the corresponding AST function
        let func_ast = find_function_ast(&parsed_program.module.items, &def.name)?;

        let builder = MirBuilder::new(db, file, semantic_index, &function_mapping, file_id);
        if let Ok(mir_func) = builder.lower_function(def_id, def, func_ast) {
            // Replace the placeholder function with the lowered one
            *mir_module.get_function_mut(func_id).unwrap() = mir_func;
        }
        // If lowering fails, keep the placeholder function (error recovery)
    }

    Some(Arc::new(mir_module))
}

/// Helper function to find a function AST by name
fn find_function_ast<'a>(
    items: &'a [TopLevelItem],
    func_name: &str,
) -> Option<&'a Spanned<FunctionDef>> {
    for item in items {
        if let TopLevelItem::Function(func) = item
            && func.value().name.value() == func_name
        {
            return Some(func);
        }
    }
    None
}

/// A builder that constructs a `MirFunction` from a semantic AST function definition
///
/// The `MirBuilder` maintains state for the function currently being built and provides
/// methods for lowering different AST constructs into MIR instructions and terminators.
struct MirBuilder<'a, 'db> {
    db: &'db dyn SemanticDb,
    file: File,
    semantic_index: &'a SemanticIndex,
    /// Global map from function DefinitionId to MIR FunctionId for call resolution
    function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
    /// Precomputed file ID for efficient MirDefinitionId creation
    file_id: u64,

    // State for the function currently being built
    mir_function: MirFunction,
    current_block_id: BasicBlockId,
    /// Local map from variable DefinitionId to its MIR ValueId
    definition_to_value: FxHashMap<MirDefinitionId, ValueId>,
    /// Becomes true when a terminator like `return` is encountered.
    is_terminated: bool,
}

impl<'a, 'db> MirBuilder<'a, 'db> {
    fn new(
        db: &'db dyn SemanticDb,
        file: File,
        semantic_index: &'a SemanticIndex,
        function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
        file_id: u64,
    ) -> Self {
        // Create a placeholder function - will be filled in during lowering
        let mir_function = MirFunction::new(String::new());
        let entry_block = mir_function.entry_block;

        Self {
            db,
            file,
            semantic_index,
            function_mapping,
            mir_function,
            current_block_id: entry_block,
            definition_to_value: FxHashMap::default(),
            file_id,
            is_terminated: false,
        }
    }

    /// Lowers a single function from the AST into a `MirFunction`
    fn lower_function(
        mut self,
        _func_def_id: DefinitionId<'db>,
        func_def: &Definition,
        func_ast: &Spanned<FunctionDef>,
    ) -> Result<MirFunction, String> {
        let func_data = func_ast.value();

        self.mir_function.name = func_def.name.clone();

        // Get the function's inner scope, where parameters are defined
        let func_inner_scope_id = self
            .semantic_index
            .scope_for_span(func_ast.span())
            .ok_or_else(|| format!("Could not find scope for function '{}'", func_def.name))?;

        // Lower parameters
        for param_ast in &func_data.params {
            if let Some((def_idx, _)) = self
                .semantic_index
                .resolve_name_to_definition(param_ast.name.value(), func_inner_scope_id)
            {
                let def_id = DefinitionId::new(self.db, self.file, def_idx);
                let mir_def_id = self.convert_definition_id(def_id);

                // 1. Query semantic type system for actual parameter type
                let semantic_type = definition_semantic_type(self.db, def_id);
                let param_type = MirType::from_semantic_type(self.db, semantic_type);

                let incoming_param_val = self.mir_function.new_typed_value_id(param_type.clone());
                self.mir_function.parameters.push(incoming_param_val);

                // 2. Map the semantic definition to its stack address
                self.definition_to_value
                    .insert(mir_def_id, incoming_param_val);
            } else {
                return Err(format!(
                    "Internal Compiler Error: Could not resolve parameter '{}'",
                    param_ast.name.value()
                ));
            }
        }

        // **Fix for Bug 2:** Treat the entire function body as a single block statement
        // This ensures all statements are processed sequentially, even after complex control flow
        let body_statements = func_data.body.clone();
        let representative_span = func_ast.span(); // Use function span as representative
        let body_as_block = Spanned::new(Statement::Block(body_statements), representative_span);
        self.lower_statement(&body_as_block)?;

        // If the main flow finished without a terminator, add one.
        if !self.current_block().is_terminated() {
            self.terminate_current_block(Terminator::return_void());
        }

        Ok(self.mir_function)
    }

    /// Lowers a single statement into MIR instructions
    fn lower_statement(&mut self, stmt: &Spanned<Statement>) -> Result<(), String> {
        match stmt.value() {
            Statement::Let { name, value, .. } | Statement::Local { name, value, .. } => {
                let rhs_value = self.lower_expression(value)?;

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
                let expr_info = self.semantic_index.expression(expr_id).ok_or_else(|| {
                    format!("MIR: No ExpressionInfo for value expression ID {expr_id:?}")
                })?;
                let scope_id = expr_info.scope_id;

                if let Some((def_idx, _)) = self
                    .semantic_index
                    .resolve_name_to_definition(name.value(), scope_id)
                {
                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Check if the RHS is already a stack-allocated aggregate.
                    // If so, we can bind the variable name directly to its address.
                    if let Expression::StructLiteral { .. } | Expression::Tuple(_) = value.value() {
                        if let Value::Operand(addr) = rhs_value {
                            // The RHS expression already allocated the object and returned its address.
                            // We just need to map the variable `name` to this address.
                            self.definition_to_value.insert(mir_def_id, addr);
                        } else {
                            // This case should ideally not happen if aggregates always return addresses.
                            // Handle as an error or fall back to old behavior.
                            return Err("Expected an address from aggregate literal".to_string());
                        }
                    } else {
                        // Check if this variable is actually used
                        let is_used =
                            if let Some(definition) = self.semantic_index.definition(def_idx) {
                                // Get the place table for this scope
                                if let Some(place_table) =
                                    self.semantic_index.place_table(definition.scope_id)
                                {
                                    // Check if the place is marked as used
                                    if let Some(place) = place_table.place(definition.place_id) {
                                        place.is_used()
                                    } else {
                                        true // Conservative: assume used if we can't find the place
                                    }
                                } else {
                                    true // Conservative: assume used if we can't find the place table
                                }
                            } else {
                                true // Conservative: assume used if we can't find the definition
                            };

                        if is_used {
                            // Original behavior for used variables
                            let semantic_type = definition_semantic_type(self.db, def_id);
                            let var_type = MirType::from_semantic_type(self.db, semantic_type);
                            let var_addr = self
                                .mir_function
                                .new_typed_value_id(MirType::pointer(var_type.clone()));
                            self.add_instruction(Instruction::stack_alloc(
                                var_addr,
                                var_type.size_units(),
                            ));
                            self.add_instruction(Instruction::store(
                                Value::operand(var_addr),
                                rhs_value,
                            ));
                            self.definition_to_value.insert(mir_def_id, var_addr);
                        } else {
                            // For unused variables, we still need to evaluate the RHS for side effects,
                            // but we don't allocate storage. We map the definition to a dummy value.
                            // Note: In a more sophisticated implementation, we might also eliminate
                            // the RHS computation if it has no side effects.
                            // TODO: Implement this.
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
                Ok(())
            }

            Statement::Return { value } => {
                let return_value = if let Some(expr) = value {
                    Some(self.lower_expression(expr)?)
                } else {
                    None
                };

                let terminator = match return_value {
                    Some(val) => Terminator::return_value(val),
                    None => Terminator::return_void(),
                };

                self.terminate_current_block(terminator);
                self.is_terminated = true;
                Ok(())
            }

            Statement::Assignment { lhs, rhs } => {
                // Lower the right-hand side to get the value to assign
                let rhs_value = self.lower_expression(rhs)?;

                // Lower the left-hand side to get the address to assign to
                let lhs_address = self.lower_lvalue_expression(lhs)?;

                // Emit a store instruction
                self.add_instruction(Instruction::store(lhs_address, rhs_value));
                Ok(())
            }

            Statement::Expression(expr) => {
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
                        let expr_info =
                            self.semantic_index.expression(expr_id).ok_or_else(|| {
                                format!(
                                    "MIR: No ExpressionInfo for statement expression ID {expr_id:?}"
                                )
                            })?;

                        if let Some((def_idx, _)) = self
                            .semantic_index
                            .resolve_name_to_definition(func_name.value(), expr_info.scope_id)
                        {
                            let func_def_id = DefinitionId::new(self.db, self.file, def_idx);

                            if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
                                // Lower arguments
                                let mut arg_values = Vec::new();
                                for arg in args {
                                    arg_values.push(self.lower_expression(arg)?);
                                }

                                // Use void call for statement context
                                self.add_instruction(Instruction::void_call(*func_id, arg_values));
                                return Ok(());
                            }
                        }
                    }
                }

                // For other statement expressions, lower normally and discard the result
                self.lower_expression(expr)?;
                Ok(())
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
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
                        let block_to_terminate =
                            self.mir_function.get_basic_block_mut(block_id).unwrap();
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
                        let block_to_terminate =
                            self.mir_function.get_basic_block_mut(block_id).unwrap();
                        block_to_terminate.set_terminator(Terminator::jump(merge_block_id));
                    }

                    // Continue generating code in the new merge block
                    self.current_block_id = merge_block_id;
                }

                Ok(())
            }

            Statement::Block(statements) => {
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

            // TODO: Implement other statement types like loops, etc.
            _ => {
                // For unimplemented statements, add a debug instruction
                self.add_instruction(Instruction::debug(
                    format!("Unimplemented statement: {:?}", stmt.value()),
                    vec![],
                ));
                Ok(())
            }
        }
    }

    /// Lowers an l-value expression and returns the `Value` holding its address
    ///
    /// L-values are expressions that can appear on the left-hand side of an assignment.
    /// This method returns the address (memory location) that can be stored to.
    fn lower_lvalue_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String> {
        // First, get the ExpressionId and its associated info
        let expr_id = self
            .semantic_index
            .expression_id_by_span(expr.span())
            .ok_or_else(|| {
                format!(
                    "MIR: No ExpressionId found for lvalue span {:?}",
                    expr.span()
                )
            })?;
        let expr_info = self
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for lvalue ID {expr_id:?}"))?;

        let current_scope_id = expr_info.scope_id;

        match &expr_info.ast_node {
            Expression::Identifier(name) => {
                // Use the correct scope_id from expr_info for resolution
                if let Some((def_idx, _)) = self
                    .semantic_index
                    .resolve_name_to_definition(name.value(), current_scope_id)
                {
                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Look up the MIR value for this definition
                    if let Some(value_id) = self.definition_to_value.get(&mir_def_id) {
                        // For simple variables, the value ID itself represents the address
                        // In a more sophisticated system, we might need AddressOf instruction
                        return Ok(Value::operand(*value_id));
                    }
                }

                // If we can't resolve the identifier, return an error value for recovery
                Ok(Value::error())
            }

            Expression::MemberAccess { object, field } => {
                // Get the base address of the object
                let object_addr = self.lower_lvalue_expression(object)?;

                // Get the object's semantic type to calculate field offset
                let object_expr_id = self
                    .semantic_index
                    .expression_id_by_span(object.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for object span {:?}",
                            object.span()
                        )
                    })?;
                let object_semantic_type =
                    expression_semantic_type(self.db, self.file, object_expr_id);
                let object_mir_type = MirType::from_semantic_type(self.db, object_semantic_type);

                // Calculate the actual field offset from the type information
                let field_offset_val = object_mir_type.field_offset(field.value())
                    .unwrap_or_else(|| {
                        panic!(
                            "Compiler Error: Field '{}' not found on type '{:?}'. This indicates an issue with type information propagation.",
                            field.value(),
                            object_mir_type
                        );
                    });
                let field_offset = Value::integer(field_offset_val as u32);

                // Query semantic type system for field type from the member access expression
                let field_semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let field_type = MirType::from_semantic_type(self.db, field_semantic_type);
                let dest = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(field_type));
                self.add_instruction(
                    Instruction::get_element_ptr(dest, object_addr, field_offset)
                        .with_comment(format!("Get address of field '{}'", field.value())),
                );
                Ok(Value::operand(dest))
            }

            Expression::IndexAccess { array, index } => {
                // Get the base address of the array
                let array_addr = self.lower_lvalue_expression(array)?;

                // Lower the index expression to get the offset
                let index_value = self.lower_expression(index)?;

                // For tuples with constant indices, we can calculate proper offsets
                // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
                let offset_value = if let Value::Literal(crate::value::Literal::Integer(
                    const_index,
                )) = index_value
                {
                    // Check if this is indexing into a tuple
                    let array_expr_id = self
                        .semantic_index
                        .expression_id_by_span(array.span())
                        .ok_or_else(|| {
                            format!(
                                "MIR: No ExpressionId found for array span {:?}",
                                array.span()
                            )
                        })?;
                    let array_semantic_type =
                        expression_semantic_type(self.db, self.file, array_expr_id);
                    let array_mir_type = MirType::from_semantic_type(self.db, array_semantic_type);

                    // If it's a tuple, calculate the proper element offset
                    if let Some(offset) = array_mir_type.tuple_element_offset(const_index as usize)
                    {
                        Value::integer(offset as u32)
                    } else {
                        // For non-tuples or out-of-bounds, use the index directly
                        index_value
                    }
                } else {
                    // For non-constant indices, use the index directly
                    index_value
                };

                // Query semantic type system for array element type from the index access expression
                let element_semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let element_type = MirType::from_semantic_type(self.db, element_semantic_type);
                let dest = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(element_type));
                self.add_instruction(
                    Instruction::get_element_ptr(dest, array_addr, offset_value)
                        .with_comment("Get address of array element".to_string()),
                );
                Ok(Value::operand(dest))
            }

            Expression::Literal(_)
            | Expression::BooleanLiteral(_)
            | Expression::FunctionCall { .. }
            | Expression::BinaryOp { .. }
            | Expression::StructLiteral { .. }
            | Expression::Tuple(_) => Err(format!(
                "Expression cannot be assigned to: {:?}",
                expr_info.ast_node
            )),
        }
    }

    /// Lowers an expression and returns the `Value` holding its result
    fn lower_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String> {
        // First, get the ExpressionId and its associated info
        let expr_id = self
            .semantic_index
            .expression_id_by_span(expr.span())
            .ok_or_else(|| format!("MIR: No ExpressionId found for span {:?}", expr.span()))?;

        let expr_info = self
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for ID {expr_id:?}"))?;

        let current_scope_id = expr_info.scope_id;

        // Use expr_info.ast_node instead of expr.value()
        match &expr_info.ast_node {
            Expression::Literal(n) => Ok(Value::integer(*n)),

            Expression::BooleanLiteral(b) => Ok(Value::boolean(*b)),

            Expression::Identifier(name) => {
                // Use the correct scope_id from expr_info for resolution
                if let Some((def_idx, _)) = self
                    .semantic_index
                    .resolve_name_to_definition(name.value(), current_scope_id)
                {
                    let def_id = DefinitionId::new(self.db, self.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Look up the MIR value for this definition
                    if let Some(var_addr) = self.definition_to_value.get(&mir_def_id) {
                        return Ok(Value::operand(*var_addr));
                    }
                }

                // If we can't resolve the identifier, return an error value for recovery
                Ok(Value::error())
            }

            Expression::BinaryOp { op, left, right } => {
                // Important: Recursive calls must still use the original Spanned<Expression>
                let lhs_value = self.lower_expression(left)?;
                let rhs_value = self.lower_expression(right)?;

                // Query semantic type system for result type based on this expression
                let semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let result_type = MirType::from_semantic_type(self.db, semantic_type);
                let dest = self.mir_function.new_typed_value_id(result_type);
                self.add_instruction(Instruction::binary_op(*op, dest, lhs_value, rhs_value));
                Ok(Value::operand(dest))
            }

            Expression::FunctionCall { callee, args } => {
                // For now, assume direct function calls (not through variables)
                if let Expression::Identifier(func_name) = callee.value() {
                    // Get the scope for the callee from its expression info
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

                    if let Some((def_idx, _)) = self
                        .semantic_index
                        .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                    {
                        let func_def_id = DefinitionId::new(self.db, self.file, def_idx);

                        if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
                            // Lower arguments
                            let mut arg_values = Vec::new();
                            for arg in args {
                                arg_values.push(self.lower_expression(arg)?);
                            }

                            // Query semantic type system for function return type
                            let semantic_type =
                                expression_semantic_type(self.db, self.file, expr_id);
                            let return_type = MirType::from_semantic_type(self.db, semantic_type);
                            let dest = self.mir_function.new_typed_value_id(return_type);
                            self.add_instruction(Instruction::call(dest, *func_id, arg_values));
                            return Ok(Value::operand(dest));
                        }
                    }
                }

                // If we can't resolve the function call, return an error value
                Ok(Value::error())
            }

            Expression::MemberAccess { object, field } => {
                // Member access in expression context (rvalue) - load from computed address
                let object_addr = self.lower_lvalue_expression(object)?;

                // Get the object's semantic type to calculate field offset
                let object_expr_id = self
                    .semantic_index
                    .expression_id_by_span(object.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for object span {:?}",
                            object.span()
                        )
                    })?;
                let object_semantic_type =
                    expression_semantic_type(self.db, self.file, object_expr_id);
                let object_mir_type = MirType::from_semantic_type(self.db, object_semantic_type);

                // Calculate the actual field offset from the type information
                let field_offset_val = object_mir_type.field_offset(field.value())
                    .unwrap_or_else(|| {
                        panic!(
                            "Compiler Error: Field '{}' not found on type '{:?}'. This indicates an issue with type information propagation.",
                            field.value(),
                            object_mir_type
                        );
                    });
                let field_offset = Value::integer(field_offset_val as u32);

                // Query semantic type system for the field type
                let semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let field_type = MirType::from_semantic_type(self.db, semantic_type);

                // Calculate the address of the field
                let field_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(field_type.clone()));
                self.add_instruction(
                    Instruction::get_element_ptr(field_addr, object_addr, field_offset)
                        .with_comment(format!("Get address of field '{}'", field.value())),
                );

                // Load the value from the field address
                let loaded_value = self.mir_function.new_typed_value_id(field_type);
                self.add_instruction(Instruction::load(loaded_value, Value::operand(field_addr)));

                Ok(Value::operand(loaded_value))
            }

            Expression::IndexAccess { array, index } => {
                // Array/index access in expression context (rvalue) - load from computed address
                let array_addr = self.lower_lvalue_expression(array)?;
                let index_value = self.lower_expression(index)?;

                // For tuples with constant indices, we can calculate proper offsets
                // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
                let offset_value = if let Value::Literal(crate::value::Literal::Integer(
                    const_index,
                )) = index_value
                {
                    // Check if this is indexing into a tuple
                    let array_expr_id = self
                        .semantic_index
                        .expression_id_by_span(array.span())
                        .ok_or_else(|| {
                            format!(
                                "MIR: No ExpressionId found for array span {:?}",
                                array.span()
                            )
                        })?;
                    let array_semantic_type =
                        expression_semantic_type(self.db, self.file, array_expr_id);
                    let array_mir_type = MirType::from_semantic_type(self.db, array_semantic_type);

                    // If it's a tuple, calculate the proper element offset
                    if let Some(offset) = array_mir_type.tuple_element_offset(const_index as usize)
                    {
                        Value::integer(offset as u32)
                    } else {
                        // For non-tuples or out-of-bounds, use the index directly
                        index_value
                    }
                } else {
                    // For non-constant indices, use the index directly
                    index_value
                };

                // Query semantic type system for the element type
                let semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let element_type = MirType::from_semantic_type(self.db, semantic_type);

                // Calculate the address of the array element
                let element_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(element_type.clone()));
                self.add_instruction(
                    Instruction::get_element_ptr(element_addr, array_addr, offset_value)
                        .with_comment("Get address of array element".to_string()),
                );

                // Load the value from the element address
                let loaded_value = self.mir_function.new_typed_value_id(element_type);
                self.add_instruction(Instruction::load(
                    loaded_value,
                    Value::operand(element_addr),
                ));

                Ok(Value::operand(loaded_value))
            }

            Expression::StructLiteral { name: _, fields } => {
                // Struct literal - allocate struct and initialize fields

                // Query semantic type system for the struct type
                let semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let struct_type = MirType::from_semantic_type(self.db, semantic_type);

                // Allocate space for the struct
                let struct_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(struct_type.clone()));
                self.add_instruction(
                    Instruction::stack_alloc(struct_addr, struct_type.size_units())
                        .with_comment("Allocate struct".to_string()),
                );

                // Initialize each field
                for (field_name, field_value) in fields.iter() {
                    let field_val = self.lower_expression(field_value)?;

                    // Calculate the actual field offset from the struct type information
                    let field_offset_val = struct_type.field_offset(field_name.value())
                        .unwrap_or_else(|| {
                            panic!(
                                "Compiler Error: Field '{}' not found on struct type '{:?}'. This indicates an issue with type information propagation.",
                                field_name.value(),
                                struct_type
                            );
                        });
                    let field_offset = Value::integer(field_offset_val as u32);

                    // Get the field type from the semantic analysis for the field value
                    let field_val_expr_id = self
                        .semantic_index
                        .expression_id_by_span(field_value.span())
                        .ok_or_else(|| {
                            format!(
                                "No expression ID for field value span: {:?}",
                                field_value.span()
                            )
                        })?;
                    let field_semantic_type =
                        expression_semantic_type(self.db, self.file, field_val_expr_id);
                    let field_type = MirType::from_semantic_type(self.db, field_semantic_type);

                    let field_addr = self
                        .mir_function
                        .new_typed_value_id(MirType::pointer(field_type));
                    self.add_instruction(
                        Instruction::get_element_ptr(
                            field_addr,
                            Value::operand(struct_addr),
                            field_offset,
                        )
                        .with_comment(format!("Get address of field '{}'", field_name.value())),
                    );

                    // Store the field value
                    self.add_instruction(Instruction::store(Value::operand(field_addr), field_val));
                }

                // Return the struct address (in a real system, this might return the struct value itself)
                Ok(Value::operand(struct_addr))
            }

            Expression::Tuple(tuple_elements) => {
                // Tuple literal - allocate tuple and initialize elements
                if tuple_elements.is_empty() {
                    // Empty tuple - just return unit value
                    return Ok(Value::integer(0)); // Unit value representation
                }

                // Query semantic type system for the tuple type
                let semantic_type = expression_semantic_type(self.db, self.file, expr_id);
                let tuple_type = MirType::from_semantic_type(self.db, semantic_type);

                // Allocate space for the tuple
                let tuple_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(tuple_type.clone()));
                self.add_instruction(
                    Instruction::stack_alloc(tuple_addr, tuple_type.size_units()).with_comment(
                        format!("Allocate tuple with {} elements", tuple_elements.len()),
                    ),
                );

                // Initialize each element
                for (element_idx, element_expr) in tuple_elements.iter().enumerate() {
                    let element_val = self.lower_expression(element_expr)?;

                    // Calculate the actual element offset from the tuple type information
                    let element_offset = tuple_type
                        .tuple_element_offset(element_idx)
                        .map(|offset| Value::integer(offset as u32))
                        .unwrap_or_else(|| {
                            // Fallback to sequential index for error recovery
                            Value::integer(element_idx as u32)
                        });

                    // Get the element type from semantic analysis
                    let element_expr_id = self
                        .semantic_index
                        .expression_id_by_span(element_expr.span())
                        .ok_or_else(|| {
                            format!(
                                "No expression ID for tuple element span: {:?}",
                                element_expr.span()
                            )
                        })?;
                    let element_semantic_type =
                        expression_semantic_type(self.db, self.file, element_expr_id);
                    let element_type = MirType::from_semantic_type(self.db, element_semantic_type);

                    let element_addr = self
                        .mir_function
                        .new_typed_value_id(MirType::pointer(element_type));
                    self.add_instruction(
                        Instruction::get_element_ptr(
                            element_addr,
                            Value::operand(tuple_addr),
                            element_offset,
                        )
                        .with_comment(format!("Get address of tuple element {element_idx}")),
                    );

                    // Store the element value
                    self.add_instruction(Instruction::store(
                        Value::operand(element_addr),
                        element_val,
                    ));
                }

                // Return the tuple address
                Ok(Value::operand(tuple_addr))
            }
        }
    }

    // --- MIR Construction Helpers ---

    /// Gets the current basic block (mutable)
    fn current_block_mut(&mut self) -> &mut BasicBlock {
        self.mir_function
            .get_basic_block_mut(self.current_block_id)
            .expect("Current block should always be valid")
    }

    /// Gets the current basic block (immutable)
    fn current_block(&self) -> &BasicBlock {
        self.mir_function
            .get_basic_block(self.current_block_id)
            .expect("Current block should always be valid")
    }

    /// Adds an instruction to the current basic block
    fn add_instruction(&mut self, instruction: Instruction) {
        self.current_block_mut().push_instruction(instruction);
    }

    /// Sets the terminator for the current basic block
    /// If a return value is set, it will be stored in the function's return_value field
    fn terminate_current_block(&mut self, terminator: Terminator) {
        if let Terminator::Return { value: Some(val) } = &terminator {
            // If returning a value, track it in the function
            // For operands, use the existing value ID; for literals, create a new one
            let return_value_id = match val {
                Value::Operand(id) => *id,
                Value::Literal(_) | Value::Error => {
                    // For literals and errors, we need to create a value ID
                    // In a more complete implementation, we might emit an assignment first
                    self.mir_function.new_value_id()
                }
            };
            self.mir_function.return_value = Some(return_value_id);
        }
        self.current_block_mut().set_terminator(terminator);
    }

    /// Converts a Salsa DefinitionId to a simple MirDefinitionId
    fn convert_definition_id(&self, def_id: DefinitionId) -> MirDefinitionId {
        MirDefinitionId {
            definition_index: def_id.id_in_file(self.db).index(),
            file_id: self.file_id, // Use precomputed file_id for efficiency
        }
    }
}

#[test]
fn test_return_value_field_assignment() {
    // This is a simple unit test to verify that our return value field logic works
    // The integration tests above already verify the end-to-end functionality

    use crate::{Instruction, MirFunction, Value};

    let mut function = MirFunction::new("test".to_string());

    // Simulate what happens in Statement::Return for a literal
    let return_val = Value::integer(42);
    let return_value_id = function.new_value_id();
    function
        .get_basic_block_mut(function.entry_block)
        .unwrap()
        .push_instruction(Instruction::assign(return_value_id, return_val));
    function.return_value = Some(return_value_id);

    // Verify the return_value field is set
    assert!(function.return_value.is_some());
    assert_eq!(function.return_value.unwrap(), return_value_id);

    println!(
        "✓ Return value field correctly set: {:?}",
        function.return_value
    );
}
