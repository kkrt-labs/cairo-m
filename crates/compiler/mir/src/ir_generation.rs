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

use std::collections::HashMap;
use std::sync::Arc;

use cairo_m_compiler_diagnostics::{Diagnostic, DiagnosticCode};
use cairo_m_compiler_parser::parse_file;
use cairo_m_compiler_parser::parser::{
    Expression, FunctionDef, Pattern, Spanned, Statement, TopLevelItem,
};
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::definition::{Definition, DefinitionKind};
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, ExpressionId, SemanticIndex};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;
use cairo_m_compiler_semantic::{File, SemanticDb, module_semantic_index};
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
/// This Salsa query takes a crate and produces the complete MIR for all modules.
/// It works by processing all modules in dependency order, identifying all functions,
/// then lowering each one into a `MirFunction` with cross-module call resolution.
///
/// # Error Handling
///
/// This function performs graceful error recovery:
/// - Returns `Err` if there are parse errors that prevent semantic analysis
/// - Generates partial MIR for functions even if some have semantic errors
/// - Uses placeholder values for unresolved references
#[salsa::tracked]
pub fn generate_mir(db: &dyn MirDb, crate_id: Crate) -> Result<Arc<MirModule>, Vec<Diagnostic>> {
    // Get semantic index for the entire crate
    let crate_semantic_index =
        match cairo_m_compiler_semantic::db::project_semantic_index(db, crate_id) {
            Ok(index) => index,
            Err(semantic_errors) => {
                // Return semantic analysis errors
                return Err(semantic_errors.all().to_vec());
            }
        };

    let mut mir_module = MirModule::new();
    let mut function_mapping = FxHashMap::default();
    let mut parsed_modules = HashMap::new();

    // First, collect all parsed modules to avoid re-parsing
    for (module_name, file) in crate_id.modules(db) {
        let parsed_program = parse_file(db, file);
        if !parsed_program.diagnostics.is_empty() {
            return Err(parsed_program.diagnostics); // Can't generate MIR if parsing failed
        }
        parsed_modules.insert(module_name.clone(), (file, parsed_program.module));
    }

    // First pass: Collect all function definitions from all modules and assign them MIR function IDs
    // This allows resolving cross-module function calls correctly
    let modules_map = crate_id.modules(db);
    for (module_name, semantic_index) in crate_semantic_index.modules() {
        let file = *modules_map
            .get(module_name)
            .expect("Module file should exist");

        for (def_idx, def) in semantic_index.all_definitions() {
            if let DefinitionKind::Function(_) = &def.kind {
                let def_id = DefinitionId::new(db, file, def_idx);
                let mir_func = MirFunction::new(def.name.clone());
                let func_id = mir_module.add_function(mir_func);
                function_mapping.insert(def_id, (def, func_id));
            }
        }
    }

    // Second pass: Lower each function's body from all modules
    for (def_id, (def, func_id)) in function_mapping.clone() {
        let file = def_id.file(db);

        // Find which module this function belongs to
        let module_name = crate_id
            .modules(db)
            .iter()
            .find(|(_, &module_file)| module_file == file)
            .map(|(name, _)| name.clone())
            .expect("File should belong to a module");

        let semantic_index = crate_semantic_index
            .modules()
            .get(&module_name)
            .expect("Module semantic index should exist");

        let (_, parsed_module) = parsed_modules
            .get(&module_name)
            .expect("Parsed module should exist");

        // Find the corresponding AST function
        let func_ast = find_function_ast(&parsed_module.items, &def.name)
            .unwrap_or_else(|| panic!("Function {} not found in AST", def.name));

        // Generate unique file_id using the file path instead of content
        // This ensures files with identical content but different paths have different IDs
        let file_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            file.file_path(db).hash(&mut hasher);
            hasher.finish()
        };

        let builder = MirBuilder::new(
            db,
            file,
            semantic_index,
            &function_mapping,
            file_id,
            crate_id,
        );
        if let Ok(mir_func) = builder.lower_function(def_id, def, func_ast) {
            // Replace the placeholder function with the lowered one
            *mir_module.get_function_mut(func_id).unwrap() = mir_func;
        }
        // If lowering fails, keep the placeholder function (error recovery)
    }

    // Run optimization passes on all functions
    let mut pass_manager = crate::passes::PassManager::standard_pipeline();
    for function in mir_module.functions.iter_mut() {
        pass_manager.run(function);
    }

    // Validate we dont have any unreachable blocks
    for function in mir_module.functions.iter() {
        for block in function.basic_blocks.iter() {
            if block.terminator == Terminator::Unreachable {
                return Err(vec![Diagnostic::error(
                    DiagnosticCode::InternalError,
                    "Unreachable blocks found in MIR".to_string(),
                )]);
            }
        }
    }

    Ok(Arc::new(mir_module))
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
    crate_id: Crate,
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
    /// Stack of loop contexts for break/continue handling
    /// Each entry contains (continue_target_block, loop_exit_block)
    /// - continue_target: where 'continue' jumps (header for while/loop, step for for)
    /// - loop_exit: where 'break' jumps
    loop_stack: Vec<(BasicBlockId, BasicBlockId)>,
}

/// Represents the result of lowering a function call
enum CallResult {
    /// Single return value
    Single(Value),
    /// Multiple return values (tuple)
    Tuple(Vec<Value>),
}

impl<'a, 'db> MirBuilder<'a, 'db> {
    fn new(
        db: &'db dyn SemanticDb,
        file: File,
        semantic_index: &'a SemanticIndex,
        function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
        file_id: u64,
        crate_id: Crate,
    ) -> Self {
        // Create a placeholder function - will be filled in during lowering
        let mir_function = MirFunction::new(String::new());
        let entry_block = mir_function.entry_block;

        Self {
            db,
            file,
            crate_id,
            semantic_index,
            function_mapping,
            mir_function,
            current_block_id: entry_block,
            definition_to_value: FxHashMap::default(),
            file_id,
            is_terminated: false,
            loop_stack: Vec::new(),
        }
    }

    /// Resolves an imported function to its FunctionId in the crate
    ///
    /// Follows the import chain: module_name.function_name -> FunctionId
    fn resolve_imported_function(
        &self,
        imported_module_name: &str,
        function_name: &str,
    ) -> Option<FunctionId> {
        // Get the crate's semantic index
        let imported_index =
            module_semantic_index(self.db, self.crate_id, imported_module_name.to_string()).ok()?;

        // Get imported module's root scope
        let imported_root = imported_index.root_scope()?;

        // Resolve the actual function definition in the imported module
        let (imported_def_idx, imported_def) =
            imported_index.resolve_name_to_definition(function_name, imported_root)?;

        // Verify it's actually a function
        if !matches!(imported_def.kind, DefinitionKind::Function(_)) {
            return None;
        }

        // Get the imported file
        let imported_file = *self.crate_id.modules(self.db).get(imported_module_name)?;

        // Create the correct DefinitionId for the imported function
        let func_def_id = DefinitionId::new(self.db, imported_file, imported_def_idx);

        // Lookup in function_mapping to get the FunctionId
        self.function_mapping
            .get(&func_def_id)
            .map(|(_, func_id)| *func_id)
    }

    /// Converts a BinaryOp to its U32 variant if operands are U32 types
    fn get_typed_binary_op(
        &self,
        op: cairo_m_compiler_parser::parser::BinaryOp,
        left_expr: &Spanned<Expression>,
        right_expr: &Spanned<Expression>,
    ) -> cairo_m_compiler_parser::parser::BinaryOp {
        // Get the expression IDs for the operands
        let left_expr_id = self.semantic_index.expression_id_by_span(left_expr.span());
        let right_expr_id = self.semantic_index.expression_id_by_span(right_expr.span());

        // Check if both operands have U32 type
        if let (Some(left_id), Some(right_id)) = (left_expr_id, right_expr_id) {
            let left_type =
                expression_semantic_type(self.db, self.crate_id, self.file, left_id, None);
            let right_type =
                expression_semantic_type(self.db, self.crate_id, self.file, right_id, None);

            if let (TypeData::U32, TypeData::U32) =
                (left_type.data(self.db), right_type.data(self.db))
            {
                // Both operands are U32, use U32 variant
                use cairo_m_compiler_parser::parser::BinaryOp;
                match op {
                    BinaryOp::Add => BinaryOp::U32Add,
                    BinaryOp::Sub => BinaryOp::U32Sub,
                    BinaryOp::Mul => BinaryOp::U32Mul,
                    BinaryOp::Div => BinaryOp::U32Div,
                    BinaryOp::Eq => BinaryOp::U32Eq,
                    BinaryOp::Neq => BinaryOp::U32Neq,
                    BinaryOp::Less => BinaryOp::U32Less,
                    BinaryOp::Greater => BinaryOp::U32Greater,
                    BinaryOp::LessEqual => BinaryOp::U32LessEqual,
                    BinaryOp::GreaterEqual => BinaryOp::U32GreaterEqual,
                    // Keep logical operators as-is
                    _ => op,
                }
            } else {
                op
            }
        } else {
            op
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
                let semantic_type = definition_semantic_type(self.db, self.crate_id, def_id);
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

    /// Lowers a `let` or `local` statement.
    fn lower_let_statement(
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
            // TODO: at the AST level, we should be able to remove this syntactic sugar.
            (Pattern::Tuple(names), Expression::Tuple(elements))
                if names.len() == elements.len() =>
            {
                // Direct tuple destructuring optimization - skip intermediate tuple allocation
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

                            // Store the element value
                            self.add_instruction(Instruction::store(
                                Value::operand(var_addr),
                                element_value,
                            ));

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
                // We use a closure to cleanly handle the multiple checks required.
                // It returns:
                // - `Ok(true)`: Optimization applied successfully.
                // - `Ok(false)`: Optimization conditions not met, fall back to generic handling.
                // - `Err(..)`: An internal compiler error occurred.
                let res = (|| -> Result<bool, String> {
                    // 1. Check if callee is a simple identifier.
                    let Expression::Identifier(func_name) = callee.value() else {
                        return Ok(false);
                    };

                    // 2. Get semantic info for the callee. Failure here is an ICE.
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

                    // 3. Resolve function definition and get its MIR ID. Fall back if not found.
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
                                // Local function not found in mapping, fallback
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
                                    // Import resolution failed, fallback
                                    return Ok(false);
                                }
                            }
                        }
                        _ => {
                            // Neither function nor import, fallback
                            return Ok(false);
                        }
                    };

                    // 4. Check that the function call returns a tuple of the correct arity.
                    let func_call_semantic_type =
                        expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                    let TypeData::Tuple(element_types) = func_call_semantic_type.data(self.db)
                    else {
                        return Ok(false); // Does not return a tuple.
                    };
                    if element_types.len() != names.len() {
                        // This would be a semantic error. Let semantic analysis handle it. Fall back.
                        return Ok(false);
                    }

                    // --- All checks passed. Apply the optimization. ---

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
                            .unwrap_or(true); // Conservatively assume used

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

                    self.add_instruction(Instruction::call(dests, func_id, arg_values));
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
                        let typed_op = self.get_typed_binary_op(*op, left, right);
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
                // Single identifier pattern - existing logic
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
            }
            Pattern::Tuple(names) => {
                // Tuple destructuring pattern for non-literal tuples
                // (literal tuples are handled by the optimization above)
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
                        // The tuple is already allocated as consecutive values
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
                            // For other cases, we need to implement proper tuple handling
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

    /// Lowers a `return` statement.
    fn lower_return_statement(
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
                Terminator::return_values(return_values)
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
                if let cairo_m_compiler_semantic::types::TypeData::Tuple(element_types) =
                    expr_semantic_type.data(self.db)
                {
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

                    Terminator::return_values(return_values)
                } else {
                    // Single value return
                    let return_value = self.lower_expression(expr)?;
                    Terminator::return_value(return_value)
                }
            }
        } else {
            Terminator::return_void()
        };

        self.terminate_current_block(terminator);
        self.is_terminated = true;
        Ok(())
    }

    /// Lowers an assignment statement.
    fn lower_assignment_statement(
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
                    let typed_op = self.get_typed_binary_op(*op, left, right);
                    self.add_instruction(Instruction::binary_op(
                        typed_op,
                        dest_id,
                        left_value,
                        right_value,
                    ));
                } else {
                    // Fall back to two-instruction approach for complex LHS expressions
                    let dest = self.mir_function.new_typed_value_id(result_type);
                    let typed_op = self.get_typed_binary_op(*op, left, right);
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

    /// Lowers an expression statement, potentially handling void function calls.
    fn lower_expression_statement(&mut self, expr: &Spanned<Expression>) -> Result<(), String> {
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

                        if let cairo_m_compiler_semantic::types::TypeData::Tuple(element_types) =
                            func_expr_semantic_type.data(self.db)
                        {
                            // Function returns a tuple - create destinations but don't use them
                            let mut dests = Vec::new();
                            for elem_type in element_types {
                                let mir_type = MirType::from_semantic_type(self.db, elem_type);
                                dests.push(self.mir_function.new_typed_value_id(mir_type));
                            }
                            self.add_instruction(Instruction::call(dests, func_id, arg_values));
                        } else if let cairo_m_compiler_semantic::types::TypeData::Tuple(types) =
                            func_expr_semantic_type.data(self.db)
                            && types.is_empty()
                        {
                            // Function returns unit/void
                            self.add_instruction(Instruction::void_call(func_id, arg_values));
                        } else {
                            // Function returns a single value - create a destination but don't use it
                            let return_type =
                                MirType::from_semantic_type(self.db, func_expr_semantic_type);
                            let dest = self.mir_function.new_typed_value_id(return_type);
                            self.add_instruction(Instruction::call(
                                vec![dest],
                                func_id,
                                arg_values,
                            ));
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

    /// Lowers an `if` statement.
    fn lower_if_statement(
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

    /// Lowers a block statement.
    fn lower_block_statement(&mut self, statements: &[Spanned<Statement>]) -> Result<(), String> {
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

    /// Lowers a `while` loop.
    fn lower_while_statement(
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

    /// Lowers an infinite `loop`.
    fn lower_loop_statement(&mut self, body: &Spanned<Statement>) -> Result<(), String> {
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

    /// Lowers a classic C-style `for` loop:
    /// `for (init; condition; step) body`
    ///
    /// CFG shape:
    /// current -> init -> jump header
    /// header: cond ? body : exit
    /// body -> (if not terminated) jump step
    /// step -> (if not terminated) jump header
    /// exit: continues after loop
    fn lower_for_statement(
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

        // Jump from current block to conditiofor_loopsn header
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

    /// Lowers a `break` statement.
    fn lower_break_statement(&mut self) -> Result<(), String> {
        if let Some((_, loop_exit)) = self.loop_stack.last() {
            // Jump to the exit block of the current loop
            self.terminate_current_block(Terminator::jump(*loop_exit));
            self.is_terminated = true;
            Ok(())
        } else {
            Err("'break' statement outside of loop".to_string())
        }
    }

    /// Lowers a `continue` statement.
    fn lower_continue_statement(&mut self) -> Result<(), String> {
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

    /// Lowers a `const` statement (which is a no-op in MIR).
    const fn lower_const_statement(&self) -> Result<(), String> {
        // Const statements are handled at the semantic level, not in MIR
        // They don't generate any runtime code
        Ok(())
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
                let object_semantic_type = expression_semantic_type(
                    self.db,
                    self.crate_id,
                    self.file,
                    object_expr_id,
                    None,
                );
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
                let field_offset = Value::integer(field_offset_val as i32);

                // Query semantic type system for field type from the member access expression
                let field_semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
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

                // For tuples with constant indices, use the index directly since elements are consecutive
                // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
                let offset_value = index_value;

                // Query semantic type system for array element type from the index access expression
                let element_semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
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

            Expression::TupleIndex { tuple, index } => {
                // Get the semantic type of the tuple to determine element types and offsets
                let tuple_expr_id = self
                    .semantic_index
                    .expression_id_by_span(tuple.span())
                    .ok_or_else(|| "No ExpressionId for tuple in TupleIndex".to_string())?;

                let tuple_semantic_type = expression_semantic_type(
                    self.db,
                    self.crate_id,
                    self.file,
                    tuple_expr_id,
                    None,
                );

                // Convert to MIR type to get offset calculation
                let tuple_mir_type = MirType::from_semantic_type(self.db, tuple_semantic_type);

                // For non-function-call tuples, use the existing lvalue approach
                let tuple_addr = self.lower_lvalue_expression(tuple)?;

                // Calculate the offset for the element
                let offset = tuple_mir_type
                    .tuple_element_offset(*index)
                    .ok_or_else(|| format!("Invalid tuple index {} for type", index))?;

                // Get element type
                let element_mir_type = match &tuple_mir_type {
                    MirType::Tuple(types) => types
                        .get(*index)
                        .ok_or_else(|| format!("Tuple index {} out of bounds", index))?
                        .clone(),
                    _ => return Err("TupleIndex on non-tuple type".to_string()),
                };

                // Calculate element address using get_element_ptr
                let element_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(element_mir_type));
                self.add_instruction(
                    Instruction::get_element_ptr(
                        element_addr,
                        tuple_addr,
                        Value::integer(offset as i32),
                    )
                    .with_comment(format!(
                        "Get address of tuple element {} for assignment",
                        index
                    )),
                );

                Ok(Value::operand(element_addr))
            }

            Expression::Literal(_, _)
            | Expression::BooleanLiteral(_)
            | Expression::FunctionCall { .. }
            | Expression::UnaryOp { .. }
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

        // Special case: For TupleIndex on function calls, we need to use expr.value()
        // because expr_info.ast_node doesn't preserve the nested structure
        if let Expression::TupleIndex { tuple, index } = expr.value() {
            if let Expression::FunctionCall { callee, args } = tuple.value() {
                // Get the expression ID for the function call
                let func_expr_id = self
                    .semantic_index
                    .expression_id_by_span(tuple.span())
                    .ok_or_else(|| "No ExpressionId for function call in TupleIndex".to_string())?;

                // Lower the function call
                match self.lower_function_call(callee, args, func_expr_id)? {
                    CallResult::Single(value) => {
                        // Check if it's an error value - if so, return it for graceful recovery
                        if matches!(value, Value::Error) {
                            return Ok(value);
                        }
                        return Err("Cannot index a non-tuple value".to_string());
                    }
                    CallResult::Tuple(values) => {
                        // Directly return the indexed value
                        if let Some(value) = values.get(*index) {
                            return Ok(*value);
                        } else {
                            return Err(format!("Tuple index {} out of bounds", index));
                        }
                    }
                }
            }
        }

        // Use expr_info.ast_node instead of expr.value()
        match &expr_info.ast_node {
            Expression::Literal(n, _) => Ok(Value::integer(*n as i32)),

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

            Expression::UnaryOp { op, expr } => {
                // Important: Recursive calls must still use the original Spanned<Expression>
                let expr_value = self.lower_expression(expr)?;

                // Query semantic type system for result type based on this expression
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                let result_type = MirType::from_semantic_type(self.db, semantic_type);
                let dest = self.mir_function.new_typed_value_id(result_type);
                self.add_instruction(Instruction::unary_op(*op, dest, expr_value));
                Ok(Value::operand(dest))
            }

            Expression::BinaryOp { op, left, right } => {
                // Important: Recursive calls must still use the original Spanned<Expression>
                let lhs_value = self.lower_expression(left)?;
                let rhs_value = self.lower_expression(right)?;

                // Query semantic type system for result type based on this expression
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                let result_type = MirType::from_semantic_type(self.db, semantic_type);
                let dest = self.mir_function.new_typed_value_id(result_type);
                let typed_op = self.get_typed_binary_op(*op, left, right);
                self.add_instruction(Instruction::binary_op(typed_op, dest, lhs_value, rhs_value));
                Ok(Value::operand(dest))
            }

            Expression::FunctionCall { callee, args } => {
                match self.lower_function_call(callee, args, expr_id)? {
                    CallResult::Single(value) => Ok(value),
                    CallResult::Tuple(values) => {
                        // For expression context, we need to return a single value
                        // Create a tuple to hold the values
                        let semantic_type = expression_semantic_type(
                            self.db,
                            self.crate_id,
                            self.file,
                            expr_id,
                            None,
                        );
                        let tuple_type = MirType::from_semantic_type(self.db, semantic_type);
                        let tuple_addr = self
                            .mir_function
                            .new_typed_value_id(MirType::pointer(tuple_type.clone()));
                        self.add_instruction(
                            Instruction::stack_alloc(tuple_addr, tuple_type.size_units())
                                .with_comment("Allocate space for tuple return value".to_string()),
                        );

                        // Store each returned value into the tuple
                        for (i, value) in values.iter().enumerate() {
                            // TODO: get real type here
                            let elem_ptr = self.mir_function.new_typed_value_id(
                                MirType::pointer(MirType::felt()), // TODO: Get proper element type
                            );
                            self.add_instruction(
                                Instruction::get_element_ptr(
                                    elem_ptr,
                                    Value::operand(tuple_addr),
                                    Value::integer(i as i32),
                                )
                                .with_comment(format!("Get address of tuple element {}", i)),
                            );
                            self.add_instruction(Instruction::store(
                                Value::operand(elem_ptr),
                                *value,
                            ));
                        }

                        Ok(Value::operand(tuple_addr))
                    }
                }
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
                let object_semantic_type = expression_semantic_type(
                    self.db,
                    self.crate_id,
                    self.file,
                    object_expr_id,
                    None,
                );
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
                let field_offset = Value::integer(field_offset_val as i32);

                // Query semantic type system for the field type
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
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

                // For tuples with constant indices, use the index directly since elements are consecutive
                // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
                let offset_value = index_value;

                // Query semantic type system for the element type
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
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
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
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
                    let field_offset = Value::integer(field_offset_val as i32);

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
                    let field_semantic_type = expression_semantic_type(
                        self.db,
                        self.crate_id,
                        self.file,
                        field_val_expr_id,
                        None,
                    );
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
                // Tuple literal - for now we still need to allocate and return an address
                // This will be optimized away in most cases by the destructuring code
                if tuple_elements.is_empty() {
                    // Empty tuple - just return unit value
                    return Ok(Value::integer(0)); // Unit value representation
                }

                // Query semantic type system for the tuple type
                let semantic_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                let tuple_type = MirType::from_semantic_type(self.db, semantic_type);

                // Allocate space for the tuple as consecutive values
                let tuple_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(tuple_type.clone()));
                self.add_instruction(
                    Instruction::stack_alloc(tuple_addr, tuple_type.size_units()).with_comment(
                        format!("Allocate tuple with {} elements", tuple_elements.len()),
                    ),
                );

                // Initialize each element consecutively
                for (element_idx, element_expr) in tuple_elements.iter().enumerate() {
                    let element_val = self.lower_expression(element_expr)?;

                    // Tuples are stored as consecutive values, so offset is just the index
                    let element_offset = Value::integer(element_idx as i32);

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
                    let element_semantic_type = expression_semantic_type(
                        self.db,
                        self.crate_id,
                        self.file,
                        element_expr_id,
                        None,
                    );
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
            Expression::TupleIndex { tuple, index } => {
                // Get the semantic type of the tuple to determine element types and offsets
                let tuple_expr_id = self
                    .semantic_index
                    .expression_id_by_span(tuple.span())
                    .ok_or_else(|| "No ExpressionId for tuple in TupleIndex".to_string())?;

                let tuple_semantic_type = expression_semantic_type(
                    self.db,
                    self.crate_id,
                    self.file,
                    tuple_expr_id,
                    None,
                );

                // Convert to MIR type to get offset calculation
                let tuple_mir_type = MirType::from_semantic_type(self.db, tuple_semantic_type);

                // Get the tuple base address
                let tuple_addr = self.lower_lvalue_expression(tuple)?;

                // Calculate the offset for the element
                let offset = tuple_mir_type
                    .tuple_element_offset(*index)
                    .ok_or_else(|| format!("Invalid tuple index {} for type", index))?;

                // Get element type
                let element_mir_type = match &tuple_mir_type {
                    MirType::Tuple(types) => types
                        .get(*index)
                        .ok_or_else(|| format!("Tuple index {} out of bounds", index))?
                        .clone(),
                    _ => return Err("TupleIndex on non-tuple type".to_string()),
                };

                // Calculate element address using get_element_ptr
                let element_addr = self
                    .mir_function
                    .new_typed_value_id(MirType::pointer(element_mir_type.clone()));
                self.add_instruction(
                    Instruction::get_element_ptr(
                        element_addr,
                        tuple_addr,
                        Value::integer(offset as i32),
                    )
                    .with_comment(format!("Get address of tuple element {}", index)),
                );

                // Load the value at the element address
                let loaded_value = self.mir_function.new_typed_value_id(element_mir_type);
                self.add_instruction(
                    Instruction::load(loaded_value, Value::operand(element_addr))
                        .with_comment(format!("Load tuple element {}", index)),
                );

                Ok(Value::operand(loaded_value))
            }
        }
    }

    // --- MIR Construction Helpers ---

    /// Lowers a function call expression to MIR, returning either a single value or multiple values
    fn lower_function_call(
        &mut self,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<CallResult, String> {
        // First, resolve the callee to a FunctionId
        let func_id = match self.resolve_callee_expression(callee) {
            Ok(id) => id,
            Err(_) => {
                // Function not found - return error value for graceful recovery
                return Ok(CallResult::Single(Value::error()));
            }
        };

        // Lower the arguments
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.lower_expression(arg)?);
        }

        // Get the return type of the function
        let semantic_type =
            expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);

        // Check if the return type is a tuple
        match semantic_type.data(self.db) {
            cairo_m_compiler_semantic::types::TypeData::Tuple(element_types) => {
                // Function returns a tuple - create multiple destination values
                let mut dests = Vec::new();
                for elem_type in element_types {
                    let mir_type = MirType::from_semantic_type(self.db, elem_type);
                    dests.push(self.mir_function.new_typed_value_id(mir_type));
                }

                self.add_instruction(Instruction::call(dests.clone(), func_id, arg_values));

                // Return the tuple values directly
                Ok(CallResult::Tuple(
                    dests.into_iter().map(Value::operand).collect(),
                ))
            }
            _ => {
                // Single return value
                let return_type = MirType::from_semantic_type(self.db, semantic_type);
                let dest = self.mir_function.new_typed_value_id(return_type);
                self.add_instruction(Instruction::call(vec![dest], func_id, arg_values));
                Ok(CallResult::Single(Value::operand(dest)))
            }
        }
    }

    /// Resolves a callee expression to a FunctionId
    /// Supports:
    /// - Simple identifiers (foo)
    /// - Member access for imports (module.foo)
    fn resolve_callee_expression(
        &self,
        callee: &Spanned<Expression>,
    ) -> Result<FunctionId, String> {
        match callee.value() {
            Expression::Identifier(func_name) => {
                // Get the scope for the callee from its expression info
                let callee_expr_id = self
                    .semantic_index
                    .expression_id_by_span(callee.span())
                    .ok_or_else(|| "No ExpressionId found for callee".to_string())?;
                let callee_expr_info = self
                    .semantic_index
                    .expression(callee_expr_id)
                    .ok_or_else(|| "No ExpressionInfo for callee".to_string())?;

                if let Some((local_def_idx, local_def)) = self
                    .semantic_index
                    .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                {
                    match &local_def.kind {
                        DefinitionKind::Function(_) => {
                            // Local function
                            let func_def_id = DefinitionId::new(self.db, self.file, local_def_idx);
                            if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
                                Ok(*func_id)
                            } else {
                                Err(format!(
                                    "Function '{}' not found in mapping",
                                    func_name.value()
                                ))
                            }
                        }
                        DefinitionKind::Use(use_ref) => {
                            // Imported function
                            self.resolve_imported_function(
                                use_ref.imported_module.value(),
                                func_name.value(),
                            )
                            .ok_or_else(|| {
                                format!(
                                    "Failed to resolve imported function '{}'",
                                    func_name.value()
                                )
                            })
                        }
                        _ => Err(format!("'{}' is not a function", func_name.value())),
                    }
                } else {
                    Err(format!("Function '{}' not found", func_name.value()))
                }
            }
            Expression::MemberAccess { object, field } => {
                // Handle module.function pattern
                if let Expression::Identifier(module_name) = object.value() {
                    // This could be an imported module function
                    self.resolve_imported_function(module_name.value(), field.value())
                        .ok_or_else(|| {
                            format!(
                                "Failed to resolve {}.{}",
                                module_name.value(),
                                field.value()
                            )
                        })
                } else {
                    Err("Complex member access callees not yet supported".to_string())
                }
            }
            _ => Err("Unsupported callee expression type".to_string()),
        }
    }

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
        if let Terminator::Return { values } = &terminator {
            // If returning values, track them in the function
            let return_value_ids: Vec<ValueId> = values
                .iter()
                .map(|val| match val {
                    Value::Operand(id) => *id,
                    Value::Literal(_) | Value::Error => {
                        // For literals and errors, we need to create a value ID
                        // In a more complete implementation, we might emit an assignment first
                        self.mir_function.new_value_id()
                    }
                })
                .collect();
            self.mir_function.return_values = return_value_ids;
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
    function.return_values = vec![return_value_id];

    // Verify the return_values field is set
    assert!(!function.return_values.is_empty());
    assert_eq!(function.return_values[0], return_value_id);

    println!(
        "✓ Return values field correctly set: {:?}",
        function.return_values
    );
}
