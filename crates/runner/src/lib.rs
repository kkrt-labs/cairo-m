pub mod memory;
pub mod vm;

use cairo_m_common::abi_codec::m31_from_i64;
use cairo_m_common::program::{AbiSlot, AbiType};
use cairo_m_common::{AbiCodecError, CairoMValue, InputValue};
use cairo_m_common::{Program, PublicAddressRanges};
use memory::MemoryError;
use stwo_prover::core::fields::m31::M31;
use vm::{VmError, VM};

/// Result type for runner operations
pub type Result<T> = std::result::Result<T, RunnerError>;

// Current limitation is that the maximum clock difference must be < 2^20
const DEFAULT_MAX_STEPS: usize = (1 << 20) - 1;

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

/// Total number of *inline memory cells* we must allocate below the args area
/// when passing arguments according to the call ABI (arrays as pointers).
/// This accounts for arrays at any nesting depth. Each array contributes:
///   size * (arg_slot_size(element) + inline_data_slots_for_args(element))
fn inline_data_slots_for_args(ty: &AbiType) -> usize {
    match ty {
        AbiType::FixedSizeArray { element, size } => {
            let el_arg = AbiType::call_slot_size(element);
            let el_inline = inline_data_slots_for_args(element);
            (*size as usize) * (el_arg + el_inline)
        }
        AbiType::Tuple(ts) => ts.iter().map(inline_data_slots_for_args).sum(),
        AbiType::Struct { fields, .. } => fields
            .iter()
            .map(|(_, t)| inline_data_slots_for_args(t))
            .sum(),
        // Non-arrays do not require extra inline materialization.
        _ => 0,
    }
}

/// Decode a value from VM memory using the *call ABI* layout starting at `base`.
/// Returns the decoded value and the number of M31 cells consumed in that memory region.
/// Arrays consume 1 cell in their containing region (a pointer), while their contents are
/// read by following the pointer and decoding recursively.
fn decode_value_from_call_memory(ty: &AbiType, vm: &VM, base: M31) -> Result<(CairoMValue, usize)> {
    match ty {
        AbiType::Felt => {
            let m = vm.memory.get_data(base)?;
            Ok((CairoMValue::Felt(m), 1))
        }
        AbiType::Bool => {
            let m = vm.memory.get_data(base)?;
            let v = m.0;
            if v != 0 && v != 1 {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid boolean value: expected 0 or 1, got {}",
                    v
                ))
                .into());
            }
            Ok((CairoMValue::Bool(v == 1), 1))
        }
        AbiType::U32 => {
            let lo = vm.memory.get_data(base)?;
            let hi = vm.memory.get_data(base + M31::from(1u32))?;
            if lo.0 >= (1 << 16) || hi.0 >= (1 << 16) {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid U32 value: lo={}, hi={} (each must be < 65536)",
                    lo.0, hi.0
                ))
                .into());
            }
            Ok((CairoMValue::U32(lo.0 | (hi.0 << 16)), 2))
        }
        AbiType::Pointer(_) => {
            let p = vm.memory.get_data(base)?;
            Ok((CairoMValue::Pointer(p), 1))
        }
        AbiType::Tuple(elems) => {
            let mut cur = 0usize;
            let mut out = Vec::with_capacity(elems.len());
            for t in elems {
                let (v, used) = decode_value_from_call_memory(t, vm, base + M31::from(cur as u32))?;
                cur += used;
                out.push(v);
            }
            Ok((CairoMValue::Tuple(out), cur))
        }
        AbiType::Struct { fields, .. } => {
            let mut cur = 0usize;
            let mut out = Vec::with_capacity(fields.len());
            for (name, fty) in fields {
                let (v, used) =
                    decode_value_from_call_memory(fty, vm, base + M31::from(cur as u32))?;
                cur += used;
                out.push((name.clone(), v));
            }
            Ok((CairoMValue::Struct(out), cur))
        }
        AbiType::FixedSizeArray { element, size } => {
            // In call ABI, an array is represented by a single pointer cell.
            let ptr = vm.memory.get_data(base)?;
            let mut items = Vec::with_capacity(*size as usize);
            let mut off = 0usize;
            for _ in 0..(*size as usize) {
                let (v, used) =
                    decode_value_from_call_memory(element, vm, ptr + M31::from(off as u32))?;
                off += used;
                items.push(v);
            }
            Ok((CairoMValue::Array(items), 1))
        }
        AbiType::Unit => Ok((CairoMValue::Unit, 0)),
    }
}

/// Decode a value from the *return frame slots* using the call ABI (arrays are pointers).
/// For arrays, follow the pointer into memory and decode recursively.
fn decode_value_from_call_slots(
    ty: &AbiType,
    frame: &[M31],
    start: usize,
    vm: &VM,
) -> Result<(CairoMValue, usize)> {
    match ty {
        AbiType::Felt => {
            if start + 1 > frame.len() {
                return Err(AbiCodecError::InsufficientData.into());
            }
            Ok((CairoMValue::Felt(frame[start]), start + 1))
        }
        AbiType::Bool => {
            if start + 1 > frame.len() {
                return Err(AbiCodecError::InsufficientData.into());
            }
            let v = frame[start].0;
            if v != 0 && v != 1 {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid boolean value: expected 0 or 1, got {}",
                    v
                ))
                .into());
            }
            Ok((CairoMValue::Bool(v == 1), start + 1))
        }
        AbiType::U32 => {
            if start + 2 > frame.len() {
                return Err(AbiCodecError::InsufficientData.into());
            }
            let lo = frame[start].0;
            let hi = frame[start + 1].0;
            if lo >= (1 << 16) || hi >= (1 << 16) {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid U32 value: lo={}, hi={} (each must be < 65536)",
                    lo, hi
                ))
                .into());
            }
            Ok((CairoMValue::U32(lo | (hi << 16)), start + 2))
        }
        AbiType::Pointer(_) => {
            if start + 1 > frame.len() {
                return Err(AbiCodecError::InsufficientData.into());
            }
            Ok((CairoMValue::Pointer(frame[start]), start + 1))
        }
        AbiType::Tuple(elems) => {
            let mut cur = start;
            let mut out = Vec::with_capacity(elems.len());
            for t in elems {
                let (v, next) = decode_value_from_call_slots(t, frame, cur, vm)?;
                cur = next;
                out.push(v);
            }
            Ok((CairoMValue::Tuple(out), cur))
        }
        AbiType::Struct { fields, .. } => {
            let mut cur = start;
            let mut out = Vec::with_capacity(fields.len());
            for (name, fty) in fields {
                let (v, next) = decode_value_from_call_slots(fty, frame, cur, vm)?;
                cur = next;
                out.push((name.clone(), v));
            }
            Ok((CairoMValue::Struct(out), cur))
        }
        AbiType::FixedSizeArray { element, size } => {
            // Frame contains a single pointer; follow it into memory.
            if start + 1 > frame.len() {
                return Err(AbiCodecError::InsufficientData.into());
            }
            let base = frame[start];
            let mut items = Vec::with_capacity(*size as usize);
            let mut off = 0usize;
            for _ in 0..(*size as usize) {
                let (v, used) =
                    decode_value_from_call_memory(element, vm, base + M31::from(off as u32))?;
                off += used;
                items.push(v);
            }
            Ok((CairoMValue::Array(items), start + 1))
        }
        AbiType::Unit => Ok((CairoMValue::Unit, start)),
    }
}

/// Decode all returns from the frame slots using the call ABI.
/// Validates that the number of consumed frame slots matches exactly.
fn decode_returns_from_call(
    returns: &[AbiSlot],
    frame: &[M31],
    vm: &VM,
) -> Result<Vec<CairoMValue>> {
    let mut cur = 0usize;
    let mut out = Vec::with_capacity(returns.len());
    for slot in returns {
        let (v, next) = decode_value_from_call_slots(&slot.ty, frame, cur, vm)?;
        cur = next;
        out.push(v);
    }
    if cur != frame.len() {
        return Err(AbiCodecError::TrailingOrInsufficientData.into());
    }
    Ok(out)
}

/// Runs a compiled Cairo-M program using input values.
/// Encodes `args` according to the *call ABI* (arrays as pointers, with inline
/// materialization below the args area), runs the program, then decodes the return
/// values using *value encoding* (arrays inline).
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

    // -------------------------------
    // Frame layout:
    //
    // fp_offset =
    //  inline_arg_data            (materialized array contents for args)
    //   arg_slots                 (arguments area; arrays are pointers)
    // + ret_slots                 (return area; arrays inline)
    // + 2                         (implicit slots)
    // -------------------------------

    // Argument area size (arrays as pointers):
    let arg_slots: usize = entrypoint_info
        .params
        .iter()
        .map(|p| AbiType::call_slot_size(&p.ty))
        .sum();

    // Inline memory we must allocate for materialized arrays in arguments:
    let inline_arg_data: usize = entrypoint_info
        .params
        .iter()
        .map(|p| inline_data_slots_for_args(&p.ty))
        .sum();

    // Return area size (call ABI; arrays are pointers):
    let ret_slots: usize = entrypoint_info
        .returns
        .iter()
        .map(|r| AbiType::call_slot_size(&r.ty))
        .sum();

    // Pre-compute where the argument area starts so we can inline-place arrays below it
    let initial_fp = vm.state.fp;
    let fp_offset = inline_arg_data + arg_slots + ret_slots + 2;

    // Array cursor points to the beginning of the *inline materialized area*.
    // Arrays must be placed BELOW where arguments will be written.
    let mut array_cursor = initial_fp;

    // Encode arguments according to call ABI: arrays => materialize inline + pass pointer
    let mut flat_args: Vec<M31> = Vec::new();
    for (slot, val) in entrypoint_info.params.iter().zip(args.iter()) {
        encode_value_for_call(&mut vm, &mut array_cursor, &slot.ty, val, &mut flat_args)?;
    }

    // Execute from entrypoint
    vm.run_from_entrypoint(
        entrypoint_info.pc as u32,
        fp_offset as u32,
        &flat_args,
        ret_slots,
        &options,
    )?;

    // Read raw return slots (call ABI area)
    let mut return_values_raw = Vec::with_capacity(ret_slots);
    for i in 0..ret_slots {
        let return_address = vm.state.fp - M31::from((ret_slots + 2 - i) as u32);
        let value = vm.memory.get_data(return_address)?;
        return_values_raw.push(value);
    }

    // Decode return values following the call ABI:
    // arrays are returned as pointers; we dereference and decode recursively.
    let decoded = decode_returns_from_call(&entrypoint_info.returns, &return_values_raw, &vm)?;

    // Define public ranges based on actual flattened args and return slot counts
    let public_address_ranges =
        PublicAddressRanges::new(vm.program_length.0, flat_args.len(), ret_slots);

    Ok(RunnerOutput {
        return_values: decoded,
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
        (AbiType::Bool, InputValue::Number(n)) => match *n {
            0 => dst.push(M31::from(0u32)),
            1 => dst.push(M31::from(1u32)),
            _ => {
                return Err(
                    AbiCodecError::TypeMismatch(format!("bool expects 0 or 1, got {}", n)).into(),
                )
            }
        },
        (AbiType::Bool, InputValue::Bool(b)) => dst.push(M31::from(if *b { 1u32 } else { 0u32 })),
        (AbiType::U32, InputValue::Number(n)) => {
            if *n < 0 || (*n as i128) > u32::MAX as i128 {
                return Err(AbiCodecError::TypeMismatch(format!("u32 out of range: {}", n)).into());
            }
            let u = *n as u32;
            let lo = M31::from(u & 0xFFFF);
            let hi = M31::from(u >> 16);
            dst.extend_from_slice(&[lo, hi]);
        }
        (AbiType::Pointer(_), _) => {
            // Pointers cannot be provided directly as inputs in runner
            return Err(AbiCodecError::TypeMismatch(
                "Pointer types are internal-only and cannot be provided as input.".to_string(),
            )
            .into());
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
            let mut elements_m31: Vec<M31> = Vec::new();
            for v in values {
                encode_value_for_call(vm, array_cursor, element, v, &mut elements_m31)?;
            }

            // Inline-allocate array elements starting from the array_cursor, ascending
            let len = elements_m31.len() as u32;
            let base = *array_cursor;

            // Write elements to memory
            for (i, m) in elements_m31.iter().enumerate() {
                vm.memory
                    .insert_no_trace(base + M31::from(i as u32), (*m).into())
                    .map_err(VmError::from)?;
            }

            // Pass pointer to elements (base) as argument value
            dst.push(base);

            // Move cursor above this block
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
