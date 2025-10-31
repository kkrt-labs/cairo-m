//! Lowering from WOMIR BlockLess DAG to Cairo-M MIR.
mod cfg;
mod context;
mod ops;

use cairo_m_compiler_mir::{MirFunction, MirModule, MirType, PassManager};
use context::DagToMirContext;
use thiserror::Error;
use womir::loader::FunctionProcessingStage;
use womir::loader::dag::ValueOrigin;

use crate::loader::{BlocklessDagModule, WasmLoadError};

#[derive(Error, Debug)]
pub enum DagToMirError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
    #[error(
        "Unsupported WASM operation {op:?} in function '{function_name}' at node {node_idx}: {suggestion}"
    )]
    UnsupportedOperation {
        op: String,
        function_name: String,
        node_idx: usize,
        suggestion: String,
    },
    #[error("Invalid control flow in function '{function_name}': {reason}")]
    InvalidControlFlow {
        function_name: String,
        reason: String,
        operation_context: String,
    },
    #[error(
        "Value mapping error in function '{function_name}' at node {node_idx}: {reason} (available: {available_count} values)"
    )]
    ValueMappingError {
        function_name: String,
        node_idx: usize,
        reason: String,
        available_count: usize,
    },
    #[error("Unsupported WASM type {wasm_type:?} in function '{function_name}': {context}")]
    UnsupportedWasmType {
        wasm_type: wasmparser::ValType,
        function_name: String,
        context: String,
    },
    #[error(
        "Loop structure error in function '{function_name}' at node {node_idx}: depth {requested_depth} exceeds available {available_depth}"
    )]
    LoopDepthError {
        function_name: String,
        node_idx: usize,
        requested_depth: u32,
        available_depth: usize,
    },
}

/// Lower a whole WOMIR program to MIR
pub fn lower_program_to_mir(
    module: &BlocklessDagModule,
    mut pipeline: PassManager,
) -> Result<MirModule, DagToMirError> {
    let mut mir_module = MirModule::new();
    let program = &module.0;
    for (func_idx, _) in program.functions.iter().enumerate() {
        let mut mir_function = function_to_mir(module, func_idx)?;
        pipeline.run(&mut mir_function);
        mir_module.add_function(mir_function);
    }
    Ok(mir_module)
}

/// Convert WASM type to MIR type (limited support for now)
fn wasm_type_to_mir_type(
    wasm_type: &wasmparser::ValType,
    function_name: &str,
    context: &str,
) -> Result<MirType, DagToMirError> {
    match wasm_type {
        wasmparser::ValType::I32 => Ok(MirType::U32),
        _ => Err(DagToMirError::UnsupportedWasmType {
            wasm_type: *wasm_type,
            function_name: function_name.to_string(),
            context: context.to_string(),
        }),
    }
}

/// Convert a single WASM function to MIR using a two-pass algorithm
fn function_to_mir(
    module: &BlocklessDagModule,
    func_idx: usize,
) -> Result<MirFunction, DagToMirError> {
    let program = &module.0;
    let func_name = program
        .m
        .exported_functions
        .get(&(func_idx as u32))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("func_{}", func_idx));

    let mut context = DagToMirContext::new(func_name.clone());

    // Get function type information for parameters and return types
    let func_type = program.m.get_func_type(func_idx as u32);

    // Handle param types with proper error handling
    let param_types: Vec<MirType> = func_type
        .ty
        .params()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function parameters"))
        .collect::<Result<Vec<MirType>, DagToMirError>>()?;

    // Handle return types with proper error handling
    let return_types: Vec<MirType> = func_type
        .ty
        .results()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function return types"))
        .collect::<Result<Vec<MirType>, DagToMirError>>()?;

    // Allocate parameters
    for (i, param_type) in param_types.iter().enumerate() {
        let param_id = context.mir_function.new_typed_value_id(param_type.clone());
        context.mir_function.parameters.push(param_id);
        context.insert_value(
            ValueOrigin {
                node: 0,
                output_idx: i as u32,
            },
            param_id,
        );
    }

    // Get the DAG for this function
    let dag = match program.functions.get(func_idx) {
        Some(FunctionProcessingStage::BlocklessDag(dag)) => dag,
        Some(_) => {
            return Err(DagToMirError::InvalidControlFlow {
                function_name: func_name,
                reason: "Function not at BlocklessDag stage".to_string(),
                operation_context: "lowering function".to_string(),
            });
        }
        None => {
            return Err(DagToMirError::ValueMappingError {
                function_name: func_name,
                node_idx: 0,
                reason: format!("Function {} not found", func_idx),
                available_count: program.functions.len(),
            });
        }
    };

    // Preallocate CFG structures and lower body
    context.allocate_blocks_and_phi_nodes(dag)?;
    context.generate_instructions_from_dag(dag, module)?;

    // Finalize all phi nodes with their operands
    context.finalize_phi_nodes()?;

    // Set entry block if we created any blocks
    if !context.mir_function.basic_blocks.is_empty() {
        context.mir_function.entry_block = 0.into();
    }

    // Populate parameter types
    for (i, &param_value_id) in context.mir_function.parameters.iter().enumerate() {
        if let Some(param_type) = param_types.get(i) {
            context
                .mir_function
                .value_types
                .insert(param_value_id, param_type.clone());
        }
    }

    // Define return values from the function signature (types/arity only).
    // The actual values returned are supplied by each Return terminator.
    context.mir_function.return_values = return_types
        .iter()
        .map(|ty| context.mir_function.new_typed_value_id(ty.clone()))
        .collect();

    Ok(context.mir_function)
}
