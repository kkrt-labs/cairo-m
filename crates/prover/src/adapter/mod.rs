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
use tracing::{Level, span};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{ExecutionBundleIterator, Memory};
use crate::adapter::merkle::{NodeData, TreeType, build_partial_merkle_tree};
use crate::poseidon2::{Poseidon2Hash, T};

pub type HashInput = [M31; T];

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProverInput {
    pub merkle_trees: MerkleTrees,
    pub memory: Memory,
    pub instructions: Instructions,
    pub public_address_ranges: PublicAddressRanges,
    pub poseidon2_inputs: Vec<HashInput>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MerkleTrees {
    pub initial_tree: Vec<NodeData>,
    pub initial_root: Option<M31>,
    pub final_tree: Vec<NodeData>,
    pub final_root: Option<M31>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: HashMap<u32, Vec<ExecutionBundle>>,
}

fn import_internal<TraceIter, MemoryIter>(
    trace_iter: TraceIter,
    memory_iter: MemoryIter,
    initial_memory: Vec<QM31>,
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

    // Process all execution bundles
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
    let mut memory = bundle_iter.into_memory();
    memory.update_multiplicities(&public_address_ranges);

    // Assert that the keys are the same for both initial_memory and final_memory
    // This is a sanity check that uses memory so it's desactivated in release builds.
    #[cfg(debug_assertions)]
    {
        let initial_keys: std::collections::HashSet<_> = memory.initial_memory.keys().collect();
        let final_keys: std::collections::HashSet<_> = memory.final_memory.keys().collect();
        assert_eq!(
            initial_keys, final_keys,
            "Initial and final memory keys do not match"
        );
    }

    // Build the partial merkle trees and add to the memory the intermediate nodes
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

    let mut poseidon2_inputs =
        Vec::<HashInput>::with_capacity(initial_tree.len() + final_tree.len());
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
        },
        poseidon2_inputs,
    })
}

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
        vec![],
        PublicAddressRanges::default(),
    )
}

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
