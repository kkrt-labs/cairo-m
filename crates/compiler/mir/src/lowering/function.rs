//! # Function-level MIR Lowering
//!
//! This module contains the main entry point for MIR generation and the
//! orchestration logic for lowering entire functions from the semantic AST.

use std::collections::HashMap;
use std::sync::Arc;

use cairo_m_compiler_diagnostics::Diagnostic;
use cairo_m_compiler_parser::parse_file;
use cairo_m_compiler_parser::parser::{FunctionDef, Parameter, Spanned, Statement, TopLevelItem};
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::definition::{Definition, DefinitionKind};
use cairo_m_compiler_semantic::semantic_index::DefinitionId;
use cairo_m_compiler_semantic::type_resolution::definition_semantic_type;
use cairo_m_compiler_semantic::FileScopeId;
use rustc_hash::FxHashMap;

use crate::db::MirDb;
use crate::pipeline::{optimize_module, PipelineConfig};
use crate::{MirFunction, MirModule, MirType, ValueId};

use super::builder::MirBuilder;
use super::stmt::LowerStmt;

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
    let pipeline_config = PipelineConfig::from_environment();

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
        let _file_id: u64 = crate_id
            .modules(db)
            .iter()
            .position(|(name, _)| name == module_name)
            .expect("Module should exist in crate") as u64;

        // Get the parsed module for this file
        let (_, parsed_module) = parsed_modules
            .get(module_name)
            .expect("Module should have been parsed");

        // Collect all function definitions in the module
        for (def_idx, def) in semantic_index.all_definitions() {
            if let DefinitionKind::Function(_) = &def.kind {
                // Find the corresponding AST node
                if let Some(_func_ast) = find_function_ast(&parsed_module.items, &def.name) {
                    let def_id = DefinitionId::new(db, file, def_idx);

                    // Create a placeholder function that will be filled in during lowering
                    let placeholder_func = MirFunction::new(def.name.clone());
                    let func_id = mir_module.add_function(placeholder_func);

                    // Map semantic DefinitionId to MIR FunctionId
                    function_mapping.insert(def_id, (def, func_id));
                } else {
                    log::warn!(
                        "Function '{}' found in semantic index but not in AST",
                        def.name
                    );
                }
            }
        }
    }

    // Second pass: Now lower all function bodies with the complete function mapping
    for (module_name, semantic_index) in crate_semantic_index.modules() {
        let file = *modules_map
            .get(module_name)
            .expect("Module file should exist");
        let file_id: u64 = crate_id
            .modules(db)
            .iter()
            .position(|(name, _)| name == module_name)
            .expect("Module should exist in crate") as u64;

        // Get the parsed module for this file
        let (_, parsed_module) = parsed_modules
            .get(module_name)
            .expect("Module should have been parsed");

        // Process all functions in the module
        for (def_idx, def) in semantic_index.all_definitions() {
            if let DefinitionKind::Function(_) = &def.kind {
                let func_def_id = DefinitionId::new(db, file, def_idx);

                // Find the corresponding AST node
                if let Some(func_ast) = find_function_ast(&parsed_module.items, &def.name) {
                    // Get the assigned FunctionId from the mapping
                    let func_id = function_mapping
                        .get(&func_def_id)
                        .map(|(_, id)| *id)
                        .expect("Function should have been registered");

                    // Create a builder for this function
                    let builder = MirBuilder::new(
                        db,
                        file,
                        semantic_index,
                        &function_mapping,
                        file_id,
                        crate_id,
                    );

                    // Lower the function
                    match lower_function(builder, func_def_id, def, func_ast) {
                        Ok(mut mir_function) => {
                            // Use direct indexing to replace the placeholder function
                            mir_module.functions[func_id] = mir_function;
                        }
                        Err(e) => {
                            log::error!("Failed to lower function '{}': {}", def.name, e);
                            // Continue with other functions even if one fails
                        }
                    }
                } else {
                    log::warn!(
                        "Function '{}' found in semantic index but not in AST",
                        def.name
                    );
                }
            }
        }
    }

    // Run optimization pipeline on the entire module
    optimize_module(&mut mir_module, &pipeline_config);

    Ok(Arc::new(mir_module))
}

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

/// Lowers a single function from the AST into a `MirFunction`
pub(super) fn lower_function<'a, 'db>(
    mut builder: MirBuilder<'a, 'db>,
    func_def_id: DefinitionId<'db>,
    func_def: &Definition,
    func_ast: &Spanned<FunctionDef>,
) -> Result<MirFunction, String> {
    // Store the function definition ID for type resolution
    builder.state.function_def_id = Some(func_def_id);
    builder.state.mir_function.name = func_def.name.clone();

    // Get the function's inner scope, where parameters are defined
    let func_inner_scope_id = builder
        .ctx
        .semantic_index
        .scope_for_span(func_ast.span())
        .ok_or_else(|| format!("Could not find scope for function '{}'", func_def.name))?;

    lower_parameters(&mut builder, func_ast, func_inner_scope_id)?;

    lower_body(&mut builder, func_ast)?;

    lower_return_type(&mut builder, func_def_id)?;

    Ok(builder.state.mir_function)
}

fn lower_parameters<'a, 'db>(
    builder: &mut MirBuilder<'a, 'db>,
    func_ast: &Spanned<FunctionDef>,
    func_inner_scope_id: FileScopeId,
) -> Result<(), String> {
    let func_data = func_ast.value();

    for param_ast in &func_data.params {
        lower_parameter(builder, param_ast, func_inner_scope_id)?;
    }

    Ok(())
}

fn lower_parameter<'a, 'db>(
    builder: &mut MirBuilder<'a, 'db>,
    param_ast: &Parameter,
    func_inner_scope_id: FileScopeId,
) -> Result<(), String> {
    let (def_idx, _) = builder
        .ctx
        .semantic_index
        .resolve_name_to_definition(param_ast.name.value(), func_inner_scope_id)
        .ok_or_else(|| {
            format!(
                "Internal Compiler Error: Could not resolve parameter '{}'",
                param_ast.name.value()
            )
        })?;

    let def_id = DefinitionId::new(builder.ctx.db, builder.ctx.file, def_idx);
    let mir_def_id = builder.convert_definition_id(def_id);

    // 1. Query semantic type system for actual parameter type
    let semantic_type = definition_semantic_type(builder.ctx.db, builder.ctx.crate_id, def_id);
    let param_type = MirType::from_semantic_type(builder.ctx.db, semantic_type);

    let incoming_param_val = builder.state.mir_function.new_typed_value_id(param_type);

    builder
        .state
        .mir_function
        .parameters
        .push(incoming_param_val);

    // 2. Map the semantic definition to its stack address
    builder
        .state
        .definition_to_value
        .insert(mir_def_id, incoming_param_val);
    Ok(())
}

fn lower_body<'a, 'db>(
    builder: &mut MirBuilder<'a, 'db>,
    func_ast: &Spanned<FunctionDef>,
) -> Result<(), String> {
    // Treat the entire function body as a single block statement
    // This ensures all statements are processed sequentially, even after complex control flow
    let body_statements = func_ast.value().body.clone();
    let representative_span = func_ast.span(); // Use function span as representative
    let body_as_block = Spanned::new(Statement::Block(body_statements), representative_span);
    builder.lower_statement(&body_as_block)?;
    Ok(())
}

fn lower_return_type<'a, 'db>(
    builder: &mut MirBuilder<'a, 'db>,
    func_def_id: DefinitionId<'db>,
) -> Result<(), String> {
    // Get the function's return types and allocate ValueIds for them
    let return_types = builder.get_function_return_types(func_def_id)?;
    let return_value_ids: Vec<ValueId> = return_types
        .into_iter()
        .map(|ty| builder.state.mir_function.new_typed_value_id(ty))
        .collect();
    builder.state.mir_function.return_values = return_value_ids;
    Ok(())
}
