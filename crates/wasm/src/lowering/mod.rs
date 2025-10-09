//! Lowering from WOMIR BlockLess DAG to Cairo-M MIR.
mod cfg;
mod context;
mod ops;

use std::collections::HashMap;

use crate::loader::{BlocklessDagModule, WasmLoadError};
use cairo_m_common::program::{AbiSlot, AbiType};
use cairo_m_compiler_codegen::{CasmBuilder, CodeGenerator, CodegenError, FunctionLayout};
use cairo_m_compiler_mir::{DataLayout, MirType};
use thiserror::Error;
use womir::loader::dag::ValueOrigin;
use womir::loader::FunctionProcessingStage;

use context::DagToCasmContext;

#[derive(Error, Debug)]
pub enum DagToCasmError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
    #[error("Unsupported WASM operation {op:?} in function '{function_name}' at node {node_idx}: {suggestion}")]
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
    #[error("Value mapping error in function '{function_name}' at node {node_idx}: {reason} (available: {available_count} values)")]
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
    #[error("Loop structure error in function '{function_name}' at node {node_idx}: depth {requested_depth} exceeds available {available_depth}")]
    LoopDepthError {
        function_name: String,
        node_idx: usize,
        requested_depth: u32,
        available_depth: usize,
    },
    #[error("Code generation failed: {0}")]
    CodegenError(#[from] CodegenError),
}

/// Lower a whole WOMIR program to CASM CodeGenerator
pub fn lower_program_to_casm(module: &BlocklessDagModule) -> Result<CodeGenerator, DagToCasmError> {
    let mut codegen = CodeGenerator::new();
    let wasm_program = &module.0;

    // Process each function
    for (func_idx, _) in wasm_program.functions.iter().enumerate() {
        let builder = function_to_casm(module, func_idx)?;

        // Get function name for entrypoint tracking
        let func_name = wasm_program
            .m
            .exported_functions
            .get(&(func_idx as u32))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("func_{}", func_idx));

        // Get function type information for parameters and return types
        let func_type = wasm_program.m.get_func_type(func_idx as u32);

        // Handle param types with proper error handling
        let param_types: Vec<MirType> = func_type
            .ty
            .params()
            .iter()
            .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function parameters"))
            .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

        // Handle return types with proper error handling
        let return_types: Vec<MirType> = func_type
            .ty
            .results()
            .iter()
            .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function return types"))
            .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

        // Build entrypoint info
        let entrypoint_info = cairo_m_common::program::EntrypointInfo {
            pc: 0, // Will be updated by add_function_from_builder
            params: param_types
                .iter()
                .enumerate()
                .map(|(i, _)| AbiSlot {
                    name: format!("param_{}", i),
                    ty: AbiType::U32,
                })
                .collect(),
            returns: return_types
                .iter()
                .enumerate()
                .map(|(i, _)| AbiSlot {
                    name: format!("return_{}", i),
                    ty: AbiType::U32,
                })
                .collect(),
        };

        // Add function using the clean API
        let layout = builder.layout.clone();
        codegen.add_function_from_builder(func_name, builder, entrypoint_info, layout)?;
    }

    // Calculate memory layout for variable-sized instructions
    codegen.calculate_memory_layout()?;

    // Resolve labels (second pass)
    codegen.resolve_labels()?;

    Ok(codegen)
}

/// Convert WASM type to MIR type (limited support for now)
fn wasm_type_to_mir_type(
    wasm_type: &wasmparser::ValType,
    function_name: &str,
    context: &str,
) -> Result<MirType, DagToCasmError> {
    match wasm_type {
        wasmparser::ValType::I32 => Ok(MirType::U32),
        _ => Err(DagToCasmError::UnsupportedWasmType {
            wasm_type: *wasm_type,
            function_name: function_name.to_string(),
            context: context.to_string(),
        }),
    }
}

/// Convert a single WASM function to MIR using a two-pass algorithm
fn function_to_casm(
    module: &BlocklessDagModule,
    func_idx: usize,
) -> Result<CasmBuilder, DagToCasmError> {
    let program = &module.0;
    let func_name = program
        .m
        .exported_functions
        .get(&(func_idx as u32))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("func_{}", func_idx));

    // Get function type information for parameters and return types
    let func_type = program.m.get_func_type(func_idx as u32);

    // Handle param types with proper error handling
    let param_types: Vec<MirType> = func_type
        .ty
        .params()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function parameters"))
        .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

    // Handle return types with proper error handling
    let return_types: Vec<MirType> = func_type
        .ty
        .results()
        .iter()
        .map(|ty| wasm_type_to_mir_type(ty, &func_name, "function return types"))
        .collect::<Result<Vec<MirType>, DagToCasmError>>()?;

    // Construct layout
    let layout = FunctionLayout {
        name: func_name.clone(),
        value_layouts: HashMap::default(),
        frame_size: 0,
        num_parameters: param_types.len(),
        num_return_values: return_types.len(),
        num_return_slots: return_types.iter().map(DataLayout::memory_size_of).sum(),
    };

    // TODO : fix label counter
    let mut context = DagToCasmContext::new(layout, 0);

    // Allocate parameters at their proper parameter offsets
    allocate_function_parameters(&mut context, &param_types, &return_types)?;

    // Get the DAG for this function
    let dag = match program.functions.get(func_idx) {
        Some(FunctionProcessingStage::BlocklessDag(dag)) => dag,
        Some(_) => {
            return Err(DagToCasmError::InvalidControlFlow {
                function_name: func_name,
                reason: "Function not at BlocklessDag stage".to_string(),
                operation_context: "lowering function".to_string(),
            });
        }
        None => {
            return Err(DagToCasmError::ValueMappingError {
                function_name: func_name,
                node_idx: 0,
                reason: format!("Function {} not found", func_idx),
                available_count: program.functions.len(),
            });
        }
    };

    context.generate_instructions_from_dag(dag, module)?;

    Ok(context.casm_builder)
}

/// Allocates function parameters at their proper negative FP offsets
///
/// Parameters are stored at negative offsets relative to the frame pointer,
/// following the Cairo-M calling convention layout:
/// fp - M - K - CALLER_SAVE_SLOTS + cumulative_param_size
fn allocate_function_parameters(
    context: &mut DagToCasmContext,
    param_types: &[MirType],
    return_types: &[MirType],
) -> Result<(), DagToCasmError> {
    // This is very similar to FunctionLayout::allocate_parameters_with_sizes()
    // TODO : refactor FunctionLayout ABI to fix this
    use cairo_m_compiler_codegen::layout::ValueLayout;

    // Calculate total parameter and return slots for proper offset calculation
    let m_slots: usize = param_types.iter().map(DataLayout::memory_size_of).sum();
    let k_slots: usize = return_types.iter().map(DataLayout::memory_size_of).sum();
    const CALLER_SAVE_SLOTS: i32 = 2;

    // Allocate parameters at their proper parameter offsets
    let mut cumulative_param_size = 0;
    for (i, param_type) in param_types.iter().enumerate() {
        let param_id = context.new_value_id_for_type(param_type.clone());
        let size = DataLayout::memory_size_of(param_type);

        // Calculate offset using the frame layout formula:
        // fp - M - K - CALLER_SAVE_SLOTS + cumulative_param_size
        let offset =
            -(m_slots as i32) - (k_slots as i32) - CALLER_SAVE_SLOTS + cumulative_param_size as i32;

        // Manually insert the parameter layout
        if size == 1 {
            context
                .casm_builder
                .layout
                .value_layouts
                .insert(param_id, ValueLayout::Slot { offset });
        } else {
            context
                .casm_builder
                .layout
                .value_layouts
                .insert(param_id, ValueLayout::MultiSlot { offset, size });
        }

        context.insert_value(
            ValueOrigin {
                node: 0,
                output_idx: i as u32,
            },
            param_id,
        );

        cumulative_param_size += size;
    }

    Ok(())
}
