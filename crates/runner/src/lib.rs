pub mod memory;
pub mod vm;

use cairo_m_common::abi_codec::m31_from_i64;
use cairo_m_common::program::{AbiSlot, AbiType};
use cairo_m_common::{AbiCodecError, CairoMValue, InputValue, Program, PublicAddressRanges};
use memory::MemoryError;
use stwo_prover::core::fields::m31::M31;
use vm::{VM, VmError};

/// Result type for runner operations
pub type Result<T> = std::result::Result<T, RunnerError>;

// Current limitation is that the maximum clock difference must be < 2^20
const DEFAULT_MAX_STEPS: usize = (1 << 20) - 1;

// Maximum value for the lower/upper 16-bit parts of a U32
const U16_MAX: u32 = 0xFFFF;

/// Errors that can occur during program execution
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("Entry point '{0}' not found. Available entry points: {1:?}")]
    EntryPointNotFound(String, Vec<String>),

    #[error("VM error: {0}")]
    VmError(#[from] VmError),

    #[error("Failed to read return value: {0}")]
    ReturnValueError(#[from] MemoryError),

    #[error("Argument count mismatch: expected {expected}, provided {provided}")]
    ArgumentCountMismatch { expected: usize, provided: usize },

    #[error("ABI encode/decode error: {0}")]
    AbiError(#[from] AbiCodecError),
}

/// Options for running a Cairo program
#[derive(Debug, Clone)]
pub struct RunnerOptions {
    /// The maximum number of steps to execute, DEFAULT_max_steps by default.
    pub max_steps: usize,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
        }
    }
}

/// Result of running a Cairo program
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The decoded return values of the program
    pub return_values: Vec<CairoMValue>,
    /// The final VM
    pub vm: VM,
    /// The public address ranges for structured access to program, input, and output data
    pub public_address_ranges: PublicAddressRanges,
}

/// Calculates the total number of memory cells needed for materializing array data
/// when passing arguments to a function.
///
/// Arrays are passed as pointers in the call ABI, but their contents must be
/// materialized in memory. This function recursively calculates the total space needed
/// for all arrays (including nested arrays within tuples/structs).
///
/// ## Returns
/// Number of M31 cells needed for materialized array contents
fn calculate_array_materialization_size(ty: &AbiType) -> usize {
    match ty {
        AbiType::FixedSizeArray { element, size } => {
            let element_call_size = AbiType::call_slot_size(element);
            let nested_array_size = calculate_array_materialization_size(element);
            (*size as usize) * (element_call_size + nested_array_size)
        }
        AbiType::Tuple(elements) => elements
            .iter()
            .map(calculate_array_materialization_size)
            .sum(),
        AbiType::Struct { fields, .. } => fields
            .iter()
            .map(|(_, field_type)| calculate_array_materialization_size(field_type))
            .sum(),
        // Scalar types don't require array materialization
        _ => 0,
    }
}

/// Value-aware materialization size calculator for arguments.
/// Handles dynamic pointers by using the provided values to determine the
/// number of cells to allocate inline.
fn calculate_array_materialization_size_with_value(ty: &AbiType, val: &InputValue) -> usize {
    match (ty, val) {
        // Static arrays: compute based on declared size and recurse into element type for nested arrays
        (AbiType::FixedSizeArray { element, size }, InputValue::List(values)) => {
            assert_eq!(*size, values.len() as u32);
            let element_call_size = AbiType::call_slot_size(element);
            let nested_array_size = calculate_array_materialization_size(element);
            (*size as usize) * (element_call_size + nested_array_size)
        }
        // Dynamic pointer: count elements and account for nested arrays in element
        (AbiType::Pointer { element, len }, InputValue::List(values)) => {
            let element_call_size = AbiType::call_slot_size(element);
            let nested_array_size = calculate_array_materialization_size(element);
            let count = len.unwrap_or(values.len() as u32) as usize;
            count * (element_call_size + nested_array_size)
        }
        // Numeric value for pointer means a single element
        (AbiType::Pointer { element, len }, InputValue::Number(_)) => {
            let element_call_size = AbiType::call_slot_size(element);
            let nested_array_size = calculate_array_materialization_size(element);
            let count = len.unwrap_or(1) as usize;
            count * (element_call_size + nested_array_size)
        }
        // Aggregates: recurse element-wise
        (AbiType::Tuple(types), InputValue::List(values)) => types
            .iter()
            .zip(values.iter())
            .map(|(t, v)| calculate_array_materialization_size_with_value(t, v))
            .sum(),
        (AbiType::Struct { fields, .. }, InputValue::Struct(values)) => fields
            .iter()
            .zip(values.iter())
            .map(|((_, t), v)| calculate_array_materialization_size_with_value(t, v))
            .sum(),
        // No materialization for numeric pointers (raw addresses) or scalars
        _ => 0,
    }
}

/// Validates and converts an M31 value to a boolean.
///
/// ## Arguments
/// * `value` - The raw M31 value (must be 0 or 1)
///
/// ## Returns
/// * `Ok(bool)` - true if value is 1, false if value is 0
/// * `Err` - if value is neither 0 nor 1
fn m31_to_bool(value: u32) -> Result<bool> {
    if value != 0 && value != 1 {
        return Err(AbiCodecError::TypeMismatch(format!(
            "Invalid boolean value: expected 0 or 1, got {}",
            value
        ))
        .into());
    }
    Ok(value == 1)
}

/// Reconstructs a U32 from its low and high 16-bit parts stored as M31 values.
///
/// ## Arguments
/// * `low_part` - Lower 16 bits (must be <= 0xFFFF)
/// * `high_part` - Upper 16 bits (must be <= 0xFFFF)
///
/// ## Returns
/// * `Ok(u32)` - The reconstructed 32-bit value
/// * `Err` - if either part exceeds 16-bit range
fn reconstruct_u32_from_parts(low_part: u32, high_part: u32) -> Result<u32> {
    if low_part > U16_MAX || high_part > U16_MAX {
        return Err(AbiCodecError::TypeMismatch(format!(
            "Invalid U32 parts: low={}, high={} (each must be <= {})",
            low_part, high_part, U16_MAX
        ))
        .into());
    }
    Ok(low_part | (high_part << 16))
}

/// Reads and decodes an array's elements from VM memory.
///
/// ## Arguments
/// * `element_type` - The ABI type of each array element
/// * `array_size` - Number of elements in the array
/// * `memory_base` - Starting address in VM memory
/// * `vm` - The VM instance to read from
///
/// ## Returns
/// Vector of decoded values
fn read_array_from_memory(
    element_type: &AbiType,
    array_size: u32,
    memory_base: M31,
    vm: &VM,
) -> Result<Vec<CairoMValue>> {
    let mut decoded_elements = Vec::with_capacity(array_size as usize);
    let mut memory_offset = 0usize;

    for _ in 0..array_size {
        let current_address = memory_base + M31::from(memory_offset as u32);
        let (decoded_value, cells_consumed) =
            decode_value_from_memory(element_type, vm, current_address)?;

        memory_offset += cells_consumed;
        decoded_elements.push(decoded_value);
    }

    Ok(decoded_elements)
}

/// Generic decoder that reads values using a provided memory reader function.
///
/// This function abstracts the decoding logic to work with different memory sources
/// (VM memory, return frame slots, etc.) by accepting a reader function.
///
/// ## Arguments
/// * `ty` - The ABI type to decode
/// * `vm` - VM instance for following array pointers
/// * `read` - Function that reads M31 values at relative offsets
/// * `base_off` - Base offset for the reader function
///
/// ## Returns
/// Tuple of (decoded value, number of M31 cells consumed)
fn decode_value_with_custom_reader<F>(
    ty: &AbiType,
    vm: &VM,
    read: &mut F,
    base_off: usize,
) -> Result<(CairoMValue, usize)>
where
    F: FnMut(usize) -> Result<M31>,
{
    match ty {
        AbiType::Felt => {
            let m31_value = read(base_off)?;
            Ok((CairoMValue::Felt(m31_value), 1))
        }
        AbiType::Pointer { element, len } => {
            let m31_value = read(base_off)?;
            if let Some(count) = len {
                let arr = read_array_from_memory(element, *count, m31_value, vm)?;
                Ok((CairoMValue::Array(arr), 1))
            } else {
                Ok((CairoMValue::Pointer(m31_value), 1))
            }
        }
        AbiType::Bool => {
            let m31_value = read(base_off)?;
            let bool_value = m31_to_bool(m31_value.0)?;
            Ok((CairoMValue::Bool(bool_value), 1))
        }
        AbiType::U32 => {
            let low_word = read(base_off)?;
            let high_word = read(base_off + 1)?;
            let u32_value = reconstruct_u32_from_parts(low_word.0, high_word.0)?;
            Ok((CairoMValue::U32(u32_value), 2))
        }
        AbiType::Tuple(element_types) => {
            let mut offset = 0usize;
            let mut tuple_values = Vec::with_capacity(element_types.len());

            for element_type in element_types {
                let (decoded_element, cells_used) =
                    decode_value_with_custom_reader(element_type, vm, read, base_off + offset)?;
                offset += cells_used;
                tuple_values.push(decoded_element);
            }
            Ok((CairoMValue::Tuple(tuple_values), offset))
        }
        AbiType::Struct { fields, .. } => {
            let mut offset = 0usize;
            let mut struct_fields = Vec::with_capacity(fields.len());

            for (field_name, field_type) in fields {
                let (decoded_field, cells_used) =
                    decode_value_with_custom_reader(field_type, vm, read, base_off + offset)?;
                offset += cells_used;
                struct_fields.push((field_name.clone(), decoded_field));
            }
            Ok((CairoMValue::Struct(struct_fields), offset))
        }
        AbiType::FixedSizeArray { element, size } => {
            // Arrays are stored as pointers in the call ABI
            let array_pointer = read(base_off)?;
            let array_elements = read_array_from_memory(element, *size, array_pointer, vm)?;
            Ok((CairoMValue::Array(array_elements), 1))
        }
        AbiType::Unit => Ok((CairoMValue::Unit, 0)),
    }
}

/// Decodes a value from VM memory starting at the specified address.
///
/// ## Arguments
/// * `ty` - The ABI type to decode
/// * `vm` - The VM instance to read from
/// * `memory_address` - Starting address in VM memory
///
/// ## Returns
/// Tuple of (decoded value, number of M31 cells consumed)
fn decode_value_from_memory(
    ty: &AbiType,
    vm: &VM,
    memory_address: M31,
) -> Result<(CairoMValue, usize)> {
    let mut memory_reader = |offset: usize| -> Result<M31> {
        Ok(vm
            .memory
            .get_data(memory_address + M31::from(offset as u32))?)
    };
    decode_value_with_custom_reader(ty, vm, &mut memory_reader, 0)
}

/// Decodes a value from the return frame slots.
///
/// ## Arguments
/// * `ty` - The ABI type to decode
/// * `return_frame` - Array of M31 values from the return frame
/// * `slot_index` - Starting index in the return frame
/// * `vm` - VM instance for following array pointers
///
/// ## Returns
/// Tuple of (decoded value, next slot index to read)
fn decode_value_from_return_slots(
    ty: &AbiType,
    return_frame: &[M31],
    slot_index: usize,
    vm: &VM,
) -> Result<(CairoMValue, usize)> {
    let mut slot_reader = |offset: usize| -> Result<M31> {
        let absolute_index = slot_index + offset;
        if absolute_index < return_frame.len() {
            Ok(return_frame[absolute_index])
        } else {
            Err(AbiCodecError::InsufficientData.into())
        }
    };

    let (decoded_value, cells_consumed) =
        decode_value_with_custom_reader(ty, vm, &mut slot_reader, 0)?;
    Ok((decoded_value, slot_index + cells_consumed))
}

/// Decodes all return values from the function's return frame.
///
/// ## Arguments
/// * `return_specs` - ABI specifications for return values
/// * `return_frame` - Raw M31 values from the return frame
/// * `vm` - VM instance for following array pointers
///
/// ## Returns
/// Vector of decoded return values
///
/// ## Errors
/// Returns error if frame size doesn't match expected return slots
fn decode_all_return_values(
    return_specs: &[AbiSlot],
    return_frame: &[M31],
    vm: &VM,
) -> Result<Vec<CairoMValue>> {
    let mut slot_position = 0usize;
    let mut decoded_returns = Vec::with_capacity(return_specs.len());

    for return_spec in return_specs {
        let (decoded_value, next_position) =
            decode_value_from_return_slots(&return_spec.ty, return_frame, slot_position, vm)?;
        slot_position = next_position;
        decoded_returns.push(decoded_value);
    }

    // Ensure we consumed exactly the right number of slots
    if slot_position != return_frame.len() {
        return Err(AbiCodecError::TrailingOrInsufficientData.into());
    }

    Ok(decoded_returns)
}

/// Executes a Cairo-M program with the specified entrypoint and arguments.
///
/// ## Arguments
/// * `program` - The compiled Cairo-M program
/// * `entrypoint` - Name of the function to execute
/// * `args` - Input arguments for the function
/// * `options` - Execution options (e.g., max steps)
///
/// ## Returns
/// `RunnerOutput` containing return values, final VM state, and memory ranges
pub fn run_cairo_program(
    program: &Program,
    entrypoint: &str,
    args: &[InputValue],
    options: RunnerOptions,
) -> Result<RunnerOutput> {
    let entrypoint_info = program.get_entrypoint(entrypoint).ok_or_else(|| {
        RunnerError::EntryPointNotFound(
            entrypoint.to_string(),
            program.entrypoints.keys().cloned().collect(),
        )
    })?;

    if entrypoint_info.params.len() != args.len() {
        return Err(RunnerError::ArgumentCountMismatch {
            expected: entrypoint_info.params.len(),
            provided: args.len(),
        });
    }

    let mut vm = VM::try_from(program)?;

    // Calculate memory layout for function call frame
    // The frame consists of:
    // 1. Space for materialized array data (below arguments)
    // 2. Argument slots (arrays stored as pointers)
    // 3. Return value slots (arrays stored as pointers)
    // 4. Frame pointer overhead (2 cells: old_fp, return_pc)
    let argument_slot_count: usize = entrypoint_info
        .params
        .iter()
        .map(|param| AbiType::call_slot_size(&param.ty))
        .sum();

    let array_materialization_size: usize = entrypoint_info
        .params
        .iter()
        .zip(args.iter())
        .map(|(param, arg)| calculate_array_materialization_size_with_value(&param.ty, arg))
        .sum();

    let return_slot_count: usize = entrypoint_info
        .returns
        .iter()
        .map(|ret| AbiType::call_slot_size(&ret.ty))
        .sum();

    let initial_frame_pointer = vm.state.fp;
    let total_frame_offset =
        array_materialization_size + argument_slot_count + return_slot_count + 2;

    // Array data is materialized starting from the current frame pointer
    let mut array_memory_cursor = initial_frame_pointer;

    let mut encoded_arguments: Vec<M31> = Vec::with_capacity(argument_slot_count);
    for (param_spec, input_value) in entrypoint_info.params.iter().zip(args.iter()) {
        encode_value_for_call(
            &mut vm,
            &mut array_memory_cursor,
            &param_spec.ty,
            input_value,
            &mut encoded_arguments,
        )?;
    }

    vm.run_from_entrypoint(
        entrypoint_info.pc as u32,
        total_frame_offset as u32,
        &encoded_arguments,
        return_slot_count,
        &options,
    )?;

    // Extract raw return values from the return frame
    let mut raw_return_frame = Vec::with_capacity(return_slot_count);
    for slot_index in 0..return_slot_count {
        let return_slot_address =
            vm.state.fp - M31::from((return_slot_count + 2 - slot_index) as u32);
        let slot_value = vm.memory.get_data(return_slot_address)?;
        raw_return_frame.push(slot_value);
    }

    let decoded_returns =
        decode_all_return_values(&entrypoint_info.returns, &raw_return_frame, &vm)?;

    // Create public address ranges for proof generation
    let public_address_ranges = PublicAddressRanges::new(
        vm.program_length.0,
        encoded_arguments.len(),
        return_slot_count,
    );

    Ok(RunnerOutput {
        return_values: decoded_returns,
        vm,
        public_address_ranges,
    })
}

/// Encode a single value for the call frame, materializing arrays in memory and pushing pointers.
///
/// For arguments, the ABI is:
/// - Felt/Bool/U32/Pointer/Tuple/Struct: flattened directly into `dst` according to their call-argument slot sizes,
///   recursing through tuples/structs.
/// - FixedSizeArray: materialize the array elements inline in memory at `array_cursor` using the *argument ABI* of the element,
///   then push a single pointer to the base of that materialization into `dst`.
///
/// For composite elements (tuple/struct) inside arrays, their in-memory layout is the *argument ABI* flattening
/// (e.g., `U32` = two M31 words, etc.). Nested arrays are handled recursively: they are materialized first and
/// their pointers are included in the flattened element representation written inline.
fn encode_value_for_call(
    vm: &mut VM,
    array_cursor: &mut M31,
    ty: &AbiType,
    val: &InputValue,
    dst: &mut Vec<M31>,
) -> Result<()> {
    match (ty, val) {
        (AbiType::Felt, InputValue::Number(n)) => dst.push(m31_from_i64(*n)),
        (AbiType::Pointer { element, len }, InputValue::List(values)) => {
            // Encode elements using argument ABI, materialize to memory and push base pointer
            let element_slot_size = AbiType::call_slot_size(element);
            let expected_capacity = values.len() * element_slot_size;
            let mut elements_m31: Vec<M31> = Vec::with_capacity(expected_capacity);
            if let Some(expected) = len {
                if *expected as usize != values.len() {
                    return Err(AbiCodecError::TypeMismatch(format!(
                        "pointer length mismatch: expected {} got {}",
                        expected,
                        values.len()
                    ))
                    .into());
                }
            }
            for v in values {
                encode_value_for_call(vm, array_cursor, element, v, &mut elements_m31)?;
            }

            let base = *array_cursor;
            for (i, m) in elements_m31.iter().enumerate() {
                vm.memory
                    .insert_no_trace(base + M31::from(i as u32), (*m).into())
                    .map_err(VmError::from)?;
            }
            let total_cells = elements_m31.len() as u32;
            dst.push(base);
            *array_cursor = base + M31::from(total_cells);
        }
        (AbiType::Bool, InputValue::Number(n)) => match *n {
            0 => dst.push(M31::from(0u32)),
            1 => dst.push(M31::from(1u32)),
            _ => {
                return Err(
                    AbiCodecError::TypeMismatch(format!("bool expects 0 or 1, got {}", n)).into(),
                );
            }
        },
        (AbiType::Bool, InputValue::Bool(b)) => dst.push(M31::from(if *b { 1u32 } else { 0u32 })),
        (AbiType::U32, InputValue::Number(n)) => {
            if *n < 0 || (*n as i128) > u32::MAX as i128 {
                return Err(AbiCodecError::TypeMismatch(format!("u32 out of range: {}", n)).into());
            }
            let u = *n as u32;
            let lo = M31::from(u & U16_MAX);
            let hi = M31::from(u >> 16);
            dst.extend_from_slice(&[lo, hi]);
        }
        (AbiType::Tuple(types), InputValue::List(values)) => {
            if types.len() != values.len() {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "tuple arity mismatch: expected {} got {}",
                    types.len(),
                    values.len()
                ))
                .into());
            }
            for (t, v) in types.iter().zip(values.iter()) {
                encode_value_for_call(vm, array_cursor, t, v, dst)?;
            }
        }
        (AbiType::Struct { fields, .. }, InputValue::Struct(values)) => {
            if fields.len() != values.len() {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "struct field count mismatch: expected {} got {}",
                    fields.len(),
                    values.len()
                ))
                .into());
            }
            for ((_, fty), v) in fields.iter().zip(values.iter()) {
                encode_value_for_call(vm, array_cursor, fty, v, dst)?;
            }
        }
        (AbiType::FixedSizeArray { element, size }, InputValue::List(values)) => {
            if *size as usize != values.len() {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "array size mismatch: expected {} got {}",
                    size,
                    values.len()
                ))
                .into());
            }

            // First, encode each element into a temporary buffer using the *argument ABI*.
            // This will recursively materialize any nested arrays and push their pointers.
            let element_slot_size = AbiType::call_slot_size(element);
            let expected_capacity = values.len() * element_slot_size;
            let mut elements_m31: Vec<M31> = Vec::with_capacity(expected_capacity);
            for v in values {
                encode_value_for_call(vm, array_cursor, element, v, &mut elements_m31)?;
            }

            // Inline-allocate array elements starting from the array_cursor, ascending
            let len = elements_m31.len() as u32;
            let base = *array_cursor;
            for (i, m) in elements_m31.iter().enumerate() {
                vm.memory
                    .insert_no_trace(base + M31::from(i as u32), (*m).into())
                    .map_err(VmError::from)?;
            }

            // Pass pointer to elements (base) as argument value
            dst.push(base);

            *array_cursor = base + M31::from(len);
        }
        (AbiType::Unit, InputValue::Unit) => {}
        _ => {
            return Err(AbiCodecError::TypeMismatch(format!(
                "incompatible type/value pair: {:?}/{:?}",
                ty, val
            ))
            .into());
        }
    }

    Ok(())
}
