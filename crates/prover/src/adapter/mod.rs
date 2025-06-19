pub mod instructions;
pub mod io;
pub mod memory;

use std::path::Path;

use instructions::Instructions;
use io::VmImportError;
use memory::{MemoryBoundaries, MemoryCache, MemoryEntry, TraceEntry};
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};

pub struct ProverInput {
    pub memory_boundaries: MemoryBoundaries,
    pub instructions: Instructions,
}

pub fn import_from_vm_output(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_vm_output").entered();

    let memory_iter = MemoryEntryFileIter::try_from(mem_path)?;
    let program_length = memory_iter.program_length();
    let memory_entries = memory_iter.map(|e| e.into());
    let trace_entries = TraceFileIter::try_from(trace_path)?.map(|e| e.into());

    adapt_from_iter(memory_entries, trace_entries, program_length)
}

pub fn adapt_from_iter<I: IntoIterator<Item = MemoryEntry>, J: IntoIterator<Item = TraceEntry>>(
    mem_iter: I,
    trace_iter: J,
    program_length: u32,
) -> Result<ProverInput, VmImportError> {
    let mut instructions = Instructions::default();
    let mut memory = mem_iter.into_iter();
    let mut trace = trace_iter.into_iter();
    let mut clock = 1;
    let mut memory_cache = MemoryCache::default();

    let Some(first) = trace.next() else {
        return Err(VmImportError::EmptyTrace);
    };

    // Push program
    for _ in 0..program_length {
        let program_entry = memory.next().ok_or(VmImportError::EmptyTrace)?;
        memory_cache.push(program_entry, clock);
        clock += 1;
    }

    // Push first instruction execution
    instructions.initial_registers = first.into();
    instructions
        .push_instr(&mut memory, first.into(), clock, &mut memory_cache)
        .map_err(VmImportError::InitialInstructionError)?;
    clock += 1;

    // Push remaining instructions executions
    for entry in trace {
        instructions.final_registers = entry.into();
        instructions
            .push_instr(&mut memory, entry.into(), clock, &mut memory_cache)
            .map_err(VmImportError::InstructionError)?;
        clock += 1;
    }

    Ok(ProverInput {
        memory_boundaries: memory_cache.get_memory_boundaries(),
        instructions,
    })
}
