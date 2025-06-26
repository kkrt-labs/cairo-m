pub mod io;
pub mod memory;

use std::collections::HashMap;
use std::path::Path;

use cairo_m_common::opcode::Opcode;
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use cairo_m_runner::RunnerOutput;
use io::VmImportError;
pub use memory::ExecutionBundle;
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{ExecutionBundleIterator, Memory};

#[derive(Debug, PartialEq, Eq)]
pub struct ProverInput {
    pub memory_boundaries: Memory,
    pub instructions: Instructions,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: HashMap<Opcode, Vec<ExecutionBundle>>,
}

fn import_internal<TraceIter, MemoryIter>(
    trace_iter: TraceIter,
    memory_iter: MemoryIter,
) -> Result<ProverInput, VmImportError>
where
    TraceIter: Iterator<Item = VmRegisters>,
    MemoryIter: Iterator<Item = RunnerMemoryEntry>,
{
    let mut bundle_iter = ExecutionBundleIterator::new(trace_iter, memory_iter);
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

    // Get the memory state from the iterator
    let memory = bundle_iter.into_memory();

    Ok(ProverInput {
        memory_boundaries: memory,
        instructions: Instructions {
            initial_registers,
            final_registers,
            states_by_opcodes,
        },
    })
}

pub fn import_from_runner_artifacts(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_runner_artifacts").entered();

    let trace_iter = TraceFileIter::try_from(trace_path)?.map(Into::into);

    let memory_file_iter = MemoryEntryFileIter::try_from(mem_path)?;
    let memory_iter = memory_file_iter.map(Into::into);

    import_internal(trace_iter, memory_iter)
}

pub fn import_from_runner_output(
    runner_output: &RunnerOutput,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_runner_output").entered();

    let vm = &runner_output.vm;
    let trace_iter = vm.trace.iter().copied();
    let memory_trace = vm.memory.trace.borrow();
    let memory_iter = memory_trace.iter().copied();

    import_internal(trace_iter, memory_iter)
}
