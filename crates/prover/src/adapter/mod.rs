pub mod io;
pub mod memory;
pub mod merkle;

use std::collections::HashMap;
use std::path::Path;

use cairo_m_common::execution::Segment;
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::{PublicAddressRanges, State as VmRegisters};
use io::VmImportError;
pub use memory::ExecutionBundle;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{DataAccess, ExecutionBundleIterator, Memory};
use crate::adapter::merkle::{build_partial_merkle_tree, NodeData, TreeType};
use crate::components::sha256::MESSAGE_SIZE;
use crate::poseidon2::{Poseidon2Hash, T};

/// Hash input type for the merkle tree component (T M31 elements)
pub type PoseidonHashInput = [M31; T];
pub type SHA256HashInput = [M31; MESSAGE_SIZE];

/// Input data structure for proof generation.
/// Contains all the hints for witness generation and the public data.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProverInput {
    /// Merkle tree commitments for initial and final memory states
    pub merkle_trees: MerkleTrees,
    /// Boundary memory states and clock update data
    pub memory: Memory,
    /// Execution bundles organized by opcode for opcode witness generation
    pub instructions: Instructions,
    /// List of public memory addresses (program/inputs/outputs)
    pub public_address_ranges: PublicAddressRanges,
    /// Hash inputs for Poseidon2 computations in Merkle trees
    pub poseidon2_inputs: Vec<PoseidonHashInput>,
    /// Hash inputs for SHA256 computations in Merkle trees
    pub sha256_inputs: Vec<SHA256HashInput>,
}

/// Merkle tree commitments for initial and final memory states for continuation.
///
/// ## For which components ?
/// MERKLE COMPONENT: only component using these hints.
///
/// Each merkle tree contains a vec of tree nodes. The root is also stored.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MerkleTrees {
    /// Vec of nodes for the initial memory state
    pub initial_tree: Vec<NodeData>,
    /// Root hash of the initial memory state (None if empty)
    pub initial_root: Option<M31>,
    /// Vec of nodes for the final memory state
    pub final_tree: Vec<NodeData>,
    /// Root hash of the final memory state (None if empty)
    pub final_root: Option<M31>,
}

/// Opcode related data.
///
/// ## For which component ?
/// OPCODE COMPONENTS: a row of an opcode component's trace requires only the execution bundle for that opcode.
/// PUBLIC DATA (not a component): initial and final registers are emitted/consumed by the public data.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Instructions {
    /// VM register state at the start of execution (PC, FP)
    pub initial_registers: VmRegisters,
    /// VM register state at the end of execution (PC, FP)
    pub final_registers: VmRegisters,
    /// Execution bundles grouped by opcode value for opcode witness generation
    pub states_by_opcodes: HashMap<u32, Vec<ExecutionBundle>>,
    /// Global data memory access log for all steps
    pub data_accesses: Vec<DataAccess>,
}

/// Internal function to convert runner output to prover input format.
///
/// This is the core transformation logic that processes execution traces and
/// memory accesses to create the structured data needed for proof generation.
///
/// ## Process Overview
/// 1. **Bundle Generation** - Convert raw traces to execution bundles
/// 2. **Opcode Grouping** - Organize bundles by opcode for components
/// 3. **Public Address Handling** - Adjust multiplicities for public data
/// 4. **Merkle Tree Construction** - Merkle tree data used to prove merkle tree construction
/// 5. **Hash Collection** - Poseidon2 inputs used to prove hash computation
///
/// ## Arguments
/// * `trace_iter` - Iterator over VM register states
/// * `memory_iter` - Iterator over memory access entries
/// * `initial_memory` - Initial memory state as QM31 values
/// * `public_addresses` - List of public addresses
///
/// ## Returns
/// * `Ok(ProverInput)` - Complete prover input data
/// * `Err(VmImportError)` - Import failed due to invalid trace data
fn import_internal<TraceIter, MemoryIter>(
    trace_iter: TraceIter,
    memory_iter: MemoryIter,
    initial_memory: HashMap<M31, (QM31, M31, M31)>,
    public_address_ranges: PublicAddressRanges,
) -> Result<ProverInput, VmImportError>
where
    TraceIter: Iterator<Item = VmRegisters>,
    MemoryIter: Iterator<Item = RunnerMemoryEntry>,
{
    let mut bundle_iter = ExecutionBundleIterator::new(trace_iter, memory_iter, initial_memory);
    let mut states_by_opcodes = HashMap::<u32, Vec<ExecutionBundle>>::default();

    // Get initial registers by peeking at the trace
    let initial_registers = bundle_iter
        .peek_initial_registers()
        .copied()
        .ok_or(VmImportError::EmptyTrace)?;
    let mut final_registers = initial_registers;

    // Iterate through the trace and memory log to build the execution bundles
    #[allow(clippy::while_let_on_iterator)]
    while let Some(bundle_result) = bundle_iter.next() {
        let bundle = bundle_result?;

        // Track final registers
        final_registers = bundle.registers;

        // Extract opcode from instruction
        let opcode = bundle.instruction.instruction.opcode_value();

        // Store bundle by opcode
        states_by_opcodes.entry(opcode).or_default().push(bundle);
    }

    // Get the final registers from the last trace entry that wasn't processed
    final_registers = bundle_iter.get_final_registers().unwrap_or(final_registers);

    // Get the memory state from the iterator
    let (mut memory, data_accesses) = bundle_iter.into_memory_and_data_accesses();
    // Extract global access log for opcode components consumption
    memory.update_multiplicities(&public_address_ranges);

    // Assert that the keys are the same for both initial_memory and final_memory
    // This is a sanity check that uses memory so it's deactivated in release builds.
    #[cfg(debug_assertions)]
    {
        let initial_keys: std::collections::HashSet<_> = memory.initial_memory.keys().collect();
        let final_keys: std::collections::HashSet<_> = memory.final_memory.keys().collect();
        assert_eq!(
            initial_keys, final_keys,
            "Initial and final memory keys do not match"
        );
    }

    // Build partial Merkle trees for memory commitments.
    // The memory is passed as mut since the merkle tree construction adds intermediate nodes to the memory map.
    let (initial_tree, initial_root) = build_partial_merkle_tree::<Poseidon2Hash>(
        &memory.initial_memory,
        TreeType::Initial,
        &public_address_ranges,
    );
    let (final_tree, final_root) = build_partial_merkle_tree::<Poseidon2Hash>(
        &memory.final_memory,
        TreeType::Final,
        &public_address_ranges,
    );

    // Extract Poseidon2 inputs from merkle trees.
    // This data is used for the Poseidon2 component
    let mut poseidon2_inputs =
        Vec::<PoseidonHashInput>::with_capacity(initial_tree.len() + final_tree.len());
    initial_tree.iter().for_each(|node| {
        poseidon2_inputs.push(node.to_hash_input());
    });
    final_tree.iter().for_each(|node| {
        poseidon2_inputs.push(node.to_hash_input());
    });

    Ok(ProverInput {
        merkle_trees: MerkleTrees {
            initial_tree,
            final_tree,
            initial_root,
            final_root,
        },
        memory,
        public_address_ranges,
        instructions: Instructions {
            initial_registers,
            final_registers,
            states_by_opcodes,
            data_accesses,
        },
        poseidon2_inputs,
        sha256_inputs: vec![],
    })
}

/// Imports prover input from runner artifact files.
///
/// This function reads execution trace and memory trace files produced by the
/// Cairo-M runner and converts them into prover input format. Currently
/// incomplete - needs serialization of initial memory and public addresses.
///
/// ## Arguments
/// * `trace_path` - Path to the execution trace file
/// * `mem_path` - Path to the memory trace file
///
/// ## Returns
/// * `Ok(ProverInput)` - Successfully imported prover data
/// * `Err(VmImportError)` - Failed to read or process files
///
/// ## Status
/// Currently unimplemented - requires serialization format for:
/// - Initial memory state
/// - Public address lists
#[allow(unreachable_code)]
#[allow(unused_variables)]
pub fn import_from_runner_artifacts(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_runner_artifacts").entered();

    let trace_iter = TraceFileIter::try_from(trace_path)?.map(Into::into);

    let memory_file_iter = MemoryEntryFileIter::try_from(mem_path)?;
    let memory_iter = memory_file_iter.map(Into::into);

    unimplemented!("serialize the initial memory and public addresses");
    import_internal(
        trace_iter,
        memory_iter,
        HashMap::new(),
        PublicAddressRanges::default(),
    )
}

/// Imports prover input directly from runner execution segment.
///
/// This is the primary entry point for converting runner output into prover input.
/// It processes a complete execution segment containing the trace, memory accesses,
/// and initial memory state.
///
/// ## Arguments
/// * `segment` - Execution segment from the Cairo-M runner containing:
///   - `trace`: Vector of VM register states
///   - `memory_trace`: Memory access trace
///   - `initial_memory`: Initial memory state as QM31 values
/// * `public_addresses` - List of public input/output memory addresses
///
/// ## Returns
/// * `Ok(ProverInput)` - Complete prover input ready for proof generation
/// * `Err(VmImportError)` - Conversion failed due to invalid segment data
pub fn import_from_runner_output(
    segment: Segment,
    public_address_ranges: PublicAddressRanges,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_runner_output").entered();

    let trace_iter = segment.trace.into_iter();
    let memory_iter = segment.memory_trace.into_inner().into_iter();

    import_internal(
        trace_iter,
        memory_iter,
        segment.initial_memory,
        public_address_ranges,
    )
}
