pub mod io;
pub mod memory;
pub mod partial_merkle;

use std::collections::HashMap;
use std::path::Path;

use cairo_m_common::opcode::Opcode;
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use cairo_m_runner::RunnerOutput;
use io::VmImportError;
pub use memory::ExecutionBundle;
use stwo_prover::core::fields::qm31::QM31;
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{ExecutionBundleIterator, Memory};
use crate::adapter::partial_merkle::{build_partial_merkle_tree, MockHasher, NodeData};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProverInput {
    pub merkle_trees: MerkleTrees,
    pub memory: Memory,
    pub instructions: Instructions,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MerkleTrees {
    pub initial_merkle_tree: Vec<NodeData>,
    pub final_merkle_tree: Vec<NodeData>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: HashMap<Opcode, Vec<ExecutionBundle>>,
}

fn import_internal<TraceIter, MemoryIter>(
    trace_iter: TraceIter,
    memory_iter: MemoryIter,
    initial_memory: Vec<QM31>,
) -> Result<ProverInput, VmImportError>
where
    TraceIter: Iterator<Item = VmRegisters>,
    MemoryIter: Iterator<Item = RunnerMemoryEntry>,
{
    let mut bundle_iter = ExecutionBundleIterator::new(trace_iter, memory_iter, initial_memory);
    let mut states_by_opcodes = HashMap::<Opcode, Vec<ExecutionBundle>>::default();

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
        let opcode = Opcode::try_from(bundle.instruction.value)?;

        // Store bundle by opcode
        states_by_opcodes.entry(opcode).or_default().push(bundle);
    }

    // Get the final registers from the last trace entry that wasn't processed
    final_registers = bundle_iter.get_final_registers().unwrap_or(final_registers);

    // Get the memory state from the iterator
    let memory = bundle_iter.into_memory();
    // Assert that the keys are the same for both initial_memory and final_memory
    let initial_keys: std::collections::HashSet<_> = memory.initial_memory.keys().collect();
    let final_keys: std::collections::HashSet<_> = memory.final_memory.keys().collect();
    assert_eq!(
        initial_keys, final_keys,
        "Initial and final memory keys do not match"
    );

    // Build the partial merkle trees
    let initial_merkle_tree = build_partial_merkle_tree::<MockHasher>(&memory.initial_memory);
    let final_merkle_tree = build_partial_merkle_tree::<MockHasher>(&memory.final_memory);

    Ok(ProverInput {
        merkle_trees: MerkleTrees {
            initial_merkle_tree,
            final_merkle_tree,
        },
        memory,
        instructions: Instructions {
            initial_registers,
            final_registers,
            states_by_opcodes,
        },
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

    unimplemented!("serialize the initial memory");
    import_internal(trace_iter, memory_iter, vec![])
}

pub fn import_from_runner_output(
    runner_output: RunnerOutput,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_runner_output").entered();

    let trace_iter = runner_output.vm.trace.into_iter();
    let memory_iter = runner_output.vm.memory.trace.into_inner().into_iter();

    import_internal(
        trace_iter,
        memory_iter,
        runner_output.vm.initial_memory.data,
    )
}
