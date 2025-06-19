pub mod instructions;
pub mod io;
pub mod memory;

use std::path::Path;

use instructions::Instructions;
use io::VmImportError;
use memory::{MemoryBoundaries, MemoryCache, MemoryEntry, TraceEntry};
use tracing::{span, Level};

use crate::adapter::io::{MemoryEntryFileIter, TraceFileIter};

#[derive(Debug)]
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
    instructions.final_registers = first.into();
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

#[cfg(test)]
mod tests {
    use stwo_prover::core::fields::m31::M31;
    use stwo_prover::core::fields::qm31::QM31;

    use super::*;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_adapt_from_iter_user_original_spec() {
        // program_length = 1
        // mem_iter = [(0,(6,10,0,0)), (0,(6,10,0,0)), (1,(10,0,0,0))]
        // trace_iter = [(0,1)]

        let memory_entries = vec![
            MemoryEntry {
                address: 0,
                value: [6, 10, 0, 0],
            }, // Initial program (a single instruction here) written
            MemoryEntry {
                address: 0,
                value: [6, 10, 0, 0],
            }, // Instruction read
            MemoryEntry {
                address: 1,
                value: [10, 0, 0, 0],
            }, // Memory operand written for store_imm
        ];

        let trace_entries = vec![
            TraceEntry { pc: 0, fp: 1 }, // Single trace entry
        ];

        let program_length = 1;

        let result = adapt_from_iter(memory_entries, trace_entries, program_length);

        assert!(result.is_ok());

        let prover_input = result.unwrap();

        // Memory boundaries checks
        assert_eq!(prover_input.memory_boundaries.initial_memory.len(), 2);
        assert_eq!(prover_input.memory_boundaries.final_memory.len(), 2);

        // Register checks
        assert_eq!(prover_input.instructions.initial_registers.pc.0, 0);
        assert_eq!(prover_input.instructions.initial_registers.fp.0, 1);
        assert_eq!(prover_input.instructions.final_registers.pc.0, 0);
        assert_eq!(prover_input.instructions.final_registers.fp.0, 1);

        // Check store_imm has exactly one state
        assert_eq!(
            prover_input.instructions.states_by_opcodes.store_imm.len(),
            1
        );
        let store_imm_state = &prover_input.instructions.states_by_opcodes.store_imm[0];

        // Check registers in store_imm state
        assert_eq!(store_imm_state.registers.pc, M31(0));
        assert_eq!(store_imm_state.registers.fp, M31(1));

        // Check memory_args in store_imm state
        let memory_args = &store_imm_state.memory_args;

        // First memory arg (instruction read)
        assert_eq!(memory_args[0].address, M31(0));
        assert_eq!(
            memory_args[0].prev_val,
            QM31::from_u32_unchecked(6, 10, 0, 0)
        );
        assert_eq!(memory_args[0].value, QM31::from_u32_unchecked(6, 10, 0, 0));
        assert_eq!(memory_args[0].prev_clock, M31(1));
        assert_eq!(memory_args[0].clock, M31(2));

        // Second memory arg (memory operand written)
        assert_eq!(memory_args[1].address, M31(1));
        assert_eq!(
            memory_args[1].prev_val,
            QM31::from_u32_unchecked(0, 0, 0, 0)
        );
        assert_eq!(memory_args[1].value, QM31::from_u32_unchecked(10, 0, 0, 0));
        assert_eq!(memory_args[1].prev_clock, M31(0));
        assert_eq!(memory_args[1].clock, M31(2));

        // Third and fourth memory args should be zero (not used in store_imm)
        assert_eq!(memory_args[2].address, M31(0));
        assert_eq!(
            memory_args[2].prev_val,
            QM31::from_u32_unchecked(0, 0, 0, 0)
        );
        assert_eq!(memory_args[2].value, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(memory_args[2].prev_clock, M31(0));
        assert_eq!(memory_args[2].clock, M31(0));

        assert_eq!(memory_args[3].address, M31(0));
        assert_eq!(
            memory_args[3].prev_val,
            QM31::from_u32_unchecked(0, 0, 0, 0)
        );
        assert_eq!(memory_args[3].value, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(memory_args[3].prev_clock, M31(0));
        assert_eq!(memory_args[3].clock, M31(0));
    }

    #[test]
    fn test_adapt_from_iter_empty_trace() {
        let memory_entries = vec![MemoryEntry {
            address: 0,
            value: [6, 10, 0, 0],
        }];
        let trace_entries: Vec<TraceEntry> = vec![];
        let program_length = 1;

        let result = adapt_from_iter(memory_entries, trace_entries, program_length);

        // Should fail with EmptyTrace error
        assert!(matches!(result, Err(VmImportError::EmptyTrace)));
    }
}
