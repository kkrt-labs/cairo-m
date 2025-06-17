pub mod instructions;
pub mod io;
pub mod memory;

use std::path::Path;

use instructions::Instructions;
use io::{read_memory_and_trace_from_paths, VmImportError};
use memory::{MemoryBoundaries, MemoryCache, MemoryEntry, TraceEntry};
use tracing::{span, Level};

pub struct ProverInput {
    pub memory_boundaries: MemoryBoundaries,
    pub instructions: Instructions,
}

pub fn import_from_vm_output(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<ProverInput, VmImportError> {
    let _span = span!(Level::INFO, "import_from_vm_output").entered();

    let (memory_entries, trace_entries) = read_memory_and_trace_from_paths(trace_path, mem_path)?;

    let memory_entries: Vec<MemoryEntry> = memory_entries.into_iter().map(|e| e.into()).collect();
    let trace_entries: Vec<TraceEntry> = trace_entries.into_iter().map(|e| e.into()).collect();

    Ok(adapt_from_iter(memory_entries, trace_entries))
}

pub fn adapt_from_iter<I: IntoIterator<Item = MemoryEntry>, J: IntoIterator<Item = TraceEntry>>(
    mem_iter: I,
    trace_iter: J,
) -> ProverInput {
    let mut instructions = Instructions::default();
    let mut memory = mem_iter.into_iter();
    let mut trace = trace_iter.into_iter();
    let mut clock = 1;
    let mut memory_cache = MemoryCache::default();

    let Some(first) = trace.next() else {
        let memory: Vec<MemoryEntry> = memory.collect();
        return ProverInput {
            memory_boundaries: MemoryBoundaries {
                initial_memory: memory.clone(),
                final_memory: memory,
            },
            instructions,
        };
    };

    instructions.initial_registers = first.into();
    instructions
        .push_instr(&mut memory, first.into(), clock, &mut memory_cache)
        .expect("Failed to push initial instruction");
    clock += 1;

    for entry in trace {
        instructions.final_registers = entry.into();
        instructions
            .push_instr(&mut memory, entry.into(), clock, &mut memory_cache)
            .expect("Failed to push instruction");
        clock += 1;
    }

    ProverInput {
        memory_boundaries: memory_cache.get_memory_boundaries(),
        instructions,
    }
}
