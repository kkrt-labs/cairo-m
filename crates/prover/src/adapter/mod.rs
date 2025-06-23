pub mod io;
pub mod memory;

use std::collections::HashMap;
use std::path::Path;

use cairo_m_common::opcode::Opcode;
use io::VmImportError;
use stwo_prover::core::fields::m31::M31;
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};
use crate::adapter::memory::{Memory, MemoryArg, MemoryEntry};

#[derive(Debug)]
pub struct ProverInput {
    pub memory_boundaries: Memory,
    pub instructions: Instructions,
}

#[derive(Debug, Default, Clone)]
pub struct VmRegisters {
    pub pc: M31,
    pub fp: M31,
}

impl From<crate::adapter::io::IoTraceEntry> for VmRegisters {
    fn from(entry: crate::adapter::io::IoTraceEntry) -> Self {
        Self {
            pc: entry.pc.into(),
            fp: entry.fp.into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: HashMap<Opcode, Vec<StateData>>,
}

#[derive(Debug, Default)]
pub struct StateData {
    pub registers: VmRegisters,
    pub memory_args: [MemoryArg; 4],
}

pub fn import_from_vm_output(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_vm_output").entered();

    let mut trace_iter = TraceFileIter::try_from(trace_path)?.peekable();
    let mut memory_iter = MemoryEntryFileIter::try_from(mem_path)?;

    let mut memory = Memory::default();
    // Initial memory uses clock = 0
    let mut clock = 1;
    let mut states_by_opcodes = HashMap::<Opcode, Vec<StateData>>::default();

    let initial_registers: VmRegisters = trace_iter
        .peek()
        .map(|&entry| entry.into())
        .ok_or(VmImportError::EmptyTrace)?;
    let mut final_registers = initial_registers.clone();

    for trace_entry in trace_iter {
        let mut memory_args: [MemoryArg; 4] = Default::default();

        let mut opcode_entry: MemoryEntry =
            memory_iter.next().ok_or(VmImportError::EmptyTrace)?.into();
        opcode_entry.clock = clock.into();
        memory_args[0] = memory.push(opcode_entry);

        let opcode: Opcode = Opcode::try_from(opcode_entry.value)?;

        memory_iter
            .by_ref()
            .take(opcode.info().memory_accesses)
            .enumerate()
            .for_each(|(i, entry)| {
                let mut entry: MemoryEntry = entry.into();
                entry.clock = clock.into();
                memory_args[i + 1] = memory.push(entry);
            });

        let state_data = StateData {
            registers: trace_entry.into(),
            memory_args,
        };

        states_by_opcodes
            .entry(opcode)
            .or_default()
            .push(state_data);

        final_registers = trace_entry.into();
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
