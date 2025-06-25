pub mod io;
pub mod memory;

use std::collections::HashMap;
use std::path::Path;

use cairo_m_common::opcode::Opcode;
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use cairo_m_runner::RunnerOutput;
use io::VmImportError;
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{Memory, MemoryArg, MemoryEntry};

#[derive(Debug, PartialEq, Eq)]
pub struct ProverInput {
    pub memory_boundaries: Memory,
    pub instructions: Instructions,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: HashMap<Opcode, Vec<StateData>>,
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct StateData {
    pub registers: VmRegisters,
    pub memory_args: [MemoryArg; 4],
}

fn import_internal<TraceIter, MemoryIter>(
    trace_iter: TraceIter,
    mut memory_iter: MemoryIter,
) -> Result<ProverInput, VmImportError>
where
    TraceIter: Iterator<Item = VmRegisters>,
    MemoryIter: Iterator<Item = RunnerMemoryEntry>,
{
    let mut trace_iter = trace_iter.peekable();
    let mut memory = Memory::default();
    // Initial memory uses clock = 0
    let mut clock = 1;
    let mut states_by_opcodes = HashMap::<Opcode, Vec<StateData>>::default();

    let initial_registers: VmRegisters = *trace_iter.peek().ok_or(VmImportError::EmptyTrace)?;
    let mut final_registers = initial_registers;

    for registers in trace_iter {
        let mut memory_args: [MemoryArg; 4] = Default::default();

        let memory_trace_entry = memory_iter.next().ok_or(VmImportError::EmptyTrace)?;
        let opcode_entry = MemoryEntry {
            address: memory_trace_entry.addr,
            value: memory_trace_entry.value,
            clock: clock.into(),
        };

        memory_args[0] = memory.push(opcode_entry);

        let opcode: Opcode = Opcode::try_from(opcode_entry.value)?;

        memory_iter
            .by_ref()
            .take(opcode.info().memory_accesses)
            .enumerate()
            .for_each(|(i, memory_trace_entry)| {
                let entry = MemoryEntry {
                    address: memory_trace_entry.addr,
                    value: memory_trace_entry.value,
                    clock: clock.into(),
                };
                memory_args[i + 1] = memory.push(entry);
            });

        let state_data = StateData {
            registers,
            memory_args,
        };

        states_by_opcodes
            .entry(opcode)
            .or_default()
            .push(state_data);

        final_registers = registers;
        clock += 1;
    }

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
