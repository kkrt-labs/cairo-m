use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

use super::memory::{MemoryCache, MemoryEntry, TraceEntry};

#[derive(Debug, Error)]
pub enum InstructionError {
    #[error("Unexpected end of memory iterator while processing instruction")]
    UnexpectedEndOfMemory,
    #[error("Unknown opcode: {0}")]
    UnknownOpcode(u32),
}

/// Opcode wrapper that provides type-safe access to opcode IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opcode {
    id: u32,
}

impl Opcode {
    /// Get the opcode ID as a u32
    pub fn id(self) -> u32 {
        self.id
    }
}

impl From<[u32; 4]> for Opcode {
    /// Extract opcode from instruction value array
    /// The opcode is stored in the first element of the array
    fn from(value: [u32; 4]) -> Self {
        Self { id: value[0] }
    }
}

impl From<&[u32; 4]> for Opcode {
    /// Extract opcode from reference to instruction value array
    fn from(value: &[u32; 4]) -> Self {
        Self { id: value[0] }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VmRegisters {
    pub pc: M31,
    pub fp: M31,
}

impl From<TraceEntry> for VmRegisters {
    fn from(entry: TraceEntry) -> Self {
        Self {
            pc: entry.pc.into(),
            fp: entry.fp.into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct MemoryArg {
    pub address: M31,
    pub prev_val: QM31,
    pub value: QM31,
    pub prev_clock: M31,
    pub clock: M31,
}

#[derive(Debug, Default)]
pub struct StateData {
    pub registers: VmRegisters,
    pub memory_args: [MemoryArg; 4],
}

#[derive(Debug, Default)]
pub struct StatesByOpcodes {
    pub store_add_fp_fp: Vec<StateData>, // [fp + off2] = [fp + off0] + [fp + off1]
    pub store_add_fp_imm: Vec<StateData>, // [fp + off2] = [fp + off0] + imm
    pub store_sub_fp_fp: Vec<StateData>, // [fp + off2] = [fp + off0] - [fp + off1]
    pub store_sub_fp_imm: Vec<StateData>, // [fp + off2] = [fp + off0] - imm
    pub store_deref_fp: Vec<StateData>,  // [fp + off2] = [fp + off0]
    pub store_double_deref_fp: Vec<StateData>, // [fp + off2] = [[fp + off0] + off1]
    pub store_imm: Vec<StateData>,       // [fp + off2] = imm
    pub store_mul_fp_fp: Vec<StateData>, // [fp + off2] = [fp + off0] * [fp + off1]
    pub store_mul_fp_imm: Vec<StateData>, // [fp + off2] = [fp + off0] * imm
    pub store_div_fp_fp: Vec<StateData>, // [fp + off2] = [fp + off0] / [fp + off1]
    pub store_div_fp_imm: Vec<StateData>, // [fp + off2] = [fp + off0] / imm
    pub call_abs_fp: Vec<StateData>,     // call abs [fp + off0]
    pub call_abs_imm: Vec<StateData>,    // call abs imm
    pub call_rel_fp: Vec<StateData>,     // call rel [fp + off0]
    pub call_rel_imm: Vec<StateData>,    // call rel imm
    pub ret: Vec<StateData>,             // ret
    pub jmp_abs_add_fp_fp: Vec<StateData>, // jmp abs [fp + off0] + [fp + off1]
    pub jmp_abs_add_fp_imm: Vec<StateData>, // jmp abs [fp + off0] + imm
    pub jmp_abs_deref_fp: Vec<StateData>, // jmp abs [fp + off0]
    pub jmp_abs_double_deref_fp: Vec<StateData>, // jmp abs [[fp + off0] + off1]
    pub jmp_abs_imm: Vec<StateData>,     // jmp abs imm
    pub jmp_abs_mul_fp_fp: Vec<StateData>, // jmp abs [fp + off0] * [fp + off1]
    pub jmp_abs_mul_fp_imm: Vec<StateData>, // jmp abs [fp + off0] * imm
    pub jmp_rel_add_fp_fp: Vec<StateData>, // jmp rel [fp + off0] + [fp + off1]
    pub jmp_rel_add_fp_imm: Vec<StateData>, // jmp rel [fp + off0] + imm
    pub jmp_rel_deref_fp: Vec<StateData>, // jmp rel [fp + off0]
    pub jmp_rel_double_deref_fp: Vec<StateData>, // jmp rel [[fp + off0] + off1]
    pub jmp_rel_imm: Vec<StateData>,     // jmp rel imm
    pub jmp_rel_mul_fp_fp: Vec<StateData>, // jmp rel [fp + off0] * [fp + off1]
    pub jmp_rel_mul_fp_imm: Vec<StateData>, // jmp rel [fp + off0] * imm
    pub jnz_fp_fp: Vec<StateData>,       // jmp rel [fp + off1] if [fp + off0] != 0
    pub jnz_fp_imm: Vec<StateData>,
}

#[derive(Debug, Default)]
pub struct Instructions {
    pub initial_registers: VmRegisters,
    pub final_registers: VmRegisters,
    pub states_by_opcodes: StatesByOpcodes,
}

impl Instructions {
    pub fn push_instr<I>(
        &mut self,
        mut memory: I,
        registers: VmRegisters,
        clock: u32,
        memory_cache: &mut MemoryCache,
    ) -> Result<(), InstructionError>
    where
        I: Iterator<Item = MemoryEntry>,
    {
        let mut state_data = StateData {
            registers,
            memory_args: Default::default(),
        };

        let instruction = memory
            .next()
            .ok_or(InstructionError::UnexpectedEndOfMemory)?;
        let opcode_id = Opcode::from(instruction.value).id();
        state_data.memory_args[0] = memory_cache.push(instruction, clock);

        match opcode_id {
            0 => {
                // store_add_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_add_fp_fp.push(state_data);
            }
            1 => {
                // store_add_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_add_fp_imm.push(state_data);
            }
            2 => {
                // store_sub_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_sub_fp_fp.push(state_data);
            }
            3 => {
                // store_sub_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_sub_fp_imm.push(state_data);
            }
            4 => {
                // store_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_deref_fp.push(state_data);
            }
            5 => {
                // store_double_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes
                    .store_double_deref_fp
                    .push(state_data);
            }
            6 => {
                // store_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.store_imm.push(state_data);
            }
            7 => {
                // store_mul_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_mul_fp_fp.push(state_data);
            }
            8 => {
                // store_mul_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_mul_fp_imm.push(state_data);
            }
            9 => {
                // store_div_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_div_fp_fp.push(state_data);
            }
            10 => {
                // store_div_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_div_fp_imm.push(state_data);
            }
            11 => {
                // call_abs_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.call_abs_fp.push(state_data);
            }
            12 => {
                // call_abs_imm
                self.states_by_opcodes.call_abs_imm.push(state_data);
            }
            13 => {
                // call_rel_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.call_rel_fp.push(state_data);
            }
            14 => {
                // call_rel_imm
                self.states_by_opcodes.call_rel_imm.push(state_data);
            }
            15 => {
                // ret
                self.states_by_opcodes.ret.push(state_data);
            }
            16 => {
                // jmp_abs_add_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_abs_add_fp_fp.push(state_data);
            }
            17 => {
                // jmp_abs_add_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_add_fp_imm.push(state_data);
            }
            18 => {
                // jmp_abs_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_deref_fp.push(state_data);
            }
            19 => {
                // jmp_abs_double_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes
                    .jmp_abs_double_deref_fp
                    .push(state_data);
            }
            20 => {
                // jmp_abs_imm
                self.states_by_opcodes.jmp_abs_imm.push(state_data);
            }
            21 => {
                // jmp_abs_mul_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_abs_mul_fp_fp.push(state_data);
            }
            22 => {
                // jmp_abs_mul_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_mul_fp_imm.push(state_data);
            }
            23 => {
                // jmp_rel_add_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_rel_add_fp_fp.push(state_data);
            }
            24 => {
                // jmp_rel_add_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_add_fp_imm.push(state_data);
            }
            25 => {
                // jmp_rel_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_deref_fp.push(state_data);
            }
            26 => {
                // jmp_rel_double_deref_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes
                    .jmp_rel_double_deref_fp
                    .push(state_data);
            }
            27 => {
                // jmp_rel_imm
                self.states_by_opcodes.jmp_rel_imm.push(state_data);
            }
            28 => {
                // jmp_rel_mul_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_rel_mul_fp_fp.push(state_data);
            }
            29 => {
                // jmp_rel_mul_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_mul_fp_imm.push(state_data);
            }
            30 => {
                // jnz_fp_fp
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jnz_fp_fp.push(state_data);
            }
            31 => {
                // jnz_fp_imm
                let mem1 = memory
                    .next()
                    .ok_or(InstructionError::UnexpectedEndOfMemory)?;
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jnz_fp_imm.push(state_data);
            }
            _ => return Err(InstructionError::UnknownOpcode(opcode_id)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::memory::{MemoryCache, MemoryEntry};
    use super::*;

    fn create_mem_entry(address: u32, value: [u32; 4]) -> MemoryEntry {
        MemoryEntry { address, value }
    }

    fn create_memory_iterator(entries: Vec<MemoryEntry>) -> impl Iterator<Item = MemoryEntry> {
        entries.into_iter()
    }

    #[test]
    fn test_push_instr_store_add_fp_fp() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(10),
            fp: M31::from(20),
        };
        let clock = 5;

        // Create memory entries for store_add_fp_fp (opcode 0)
        let memory_entries = vec![
            create_mem_entry(0, [0, 0, 0, 0]),    // instruction with opcode 0
            create_mem_entry(1, [1, 2, 3, 4]),    // first operand
            create_mem_entry(2, [5, 6, 7, 8]),    // second operand
            create_mem_entry(3, [9, 10, 11, 12]), // result location
        ];

        instructions
            .push_instr(
                create_memory_iterator(memory_entries),
                state,
                clock,
                &mut memory_cache,
            )
            .unwrap();

        // Check that the instruction was added to the correct opcode vector
        assert_eq!(instructions.states_by_opcodes.store_add_fp_fp.len(), 1);
        assert!(instructions.states_by_opcodes.store_add_fp_imm.is_empty());

        let state_data = &instructions.states_by_opcodes.store_add_fp_fp[0];
        assert_eq!(state_data.registers.pc, M31::from(10));
        assert_eq!(state_data.registers.fp, M31::from(20));

        // Check that memory_args are properly set
        assert_eq!(state_data.memory_args[0].address, M31::from(0)); // instruction address
        assert_eq!(state_data.memory_args[1].address, M31::from(1)); // first operand address
        assert_eq!(state_data.memory_args[2].address, M31::from(2)); // second operand address
        assert_eq!(state_data.memory_args[3].address, M31::from(3)); // result address
    }

    #[test]
    fn test_push_instr_store_add_fp_imm() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(15),
            fp: M31::from(25),
        };
        let clock = 7;

        // Create memory entries for store_add_fp_imm (opcode 1)
        let memory_entries = vec![
            create_mem_entry(0, [1, 0, 0, 0]), // instruction with opcode 1
            create_mem_entry(1, [1, 2, 3, 4]), // operand
            create_mem_entry(2, [5, 6, 7, 8]), // result location
        ];

        instructions
            .push_instr(
                create_memory_iterator(memory_entries),
                state,
                clock,
                &mut memory_cache,
            )
            .unwrap();

        assert_eq!(instructions.states_by_opcodes.store_add_fp_imm.len(), 1);
        assert!(instructions.states_by_opcodes.store_add_fp_fp.is_empty());

        let state_data = &instructions.states_by_opcodes.store_add_fp_imm[0];
        assert_eq!(state_data.registers.pc, M31::from(15));
        assert_eq!(state_data.registers.fp, M31::from(25));

        // For store_add_fp_imm, only 3 memory entries are used
        assert_eq!(state_data.memory_args[0].address, M31::from(0)); // instruction
        assert_eq!(state_data.memory_args[1].address, M31::from(1)); // operand
        assert_eq!(state_data.memory_args[2].address, M31::from(2)); // result
        assert_eq!(state_data.memory_args[3].address, M31::from(0)); // unused, should be default
    }

    #[test]
    fn test_push_instr_store_imm() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(5),
            fp: M31::from(10),
        };
        let clock = 3;

        // Create memory entries for store_imm (opcode 6)
        let memory_entries = vec![
            create_mem_entry(0, [6, 0, 0, 0]),  // instruction with opcode 6
            create_mem_entry(1, [42, 0, 0, 0]), // result location
        ];

        instructions
            .push_instr(
                create_memory_iterator(memory_entries),
                state,
                clock,
                &mut memory_cache,
            )
            .unwrap();

        assert_eq!(instructions.states_by_opcodes.store_imm.len(), 1);

        let state_data = &instructions.states_by_opcodes.store_imm[0];
        assert_eq!(state_data.registers.pc, M31::from(5));
        assert_eq!(state_data.registers.fp, M31::from(10));

        // For store_imm, only 2 memory entries are used
        assert_eq!(state_data.memory_args[0].address, M31::from(0)); // instruction
        assert_eq!(state_data.memory_args[1].address, M31::from(1)); // result
        assert_eq!(state_data.memory_args[2].address, M31::from(0)); // unused
        assert_eq!(state_data.memory_args[3].address, M31::from(0)); // unused
    }

    #[test]
    fn test_push_instr_call_abs_imm() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(100),
            fp: M31::from(200),
        };
        let clock = 15;

        // Create memory entries for call_abs_imm (opcode 12)
        let memory_entries = vec![
            create_mem_entry(0, [12, 0, 0, 0]), // instruction with opcode 12
        ];

        instructions
            .push_instr(
                create_memory_iterator(memory_entries),
                state,
                clock,
                &mut memory_cache,
            )
            .unwrap();

        assert_eq!(instructions.states_by_opcodes.call_abs_imm.len(), 1);

        let state_data = &instructions.states_by_opcodes.call_abs_imm[0];
        assert_eq!(state_data.registers.pc, M31::from(100));
        assert_eq!(state_data.registers.fp, M31::from(200));

        // For call_abs_imm, only 1 memory entry is used
        assert_eq!(state_data.memory_args[0].address, M31::from(0)); // instruction
        assert_eq!(state_data.memory_args[1].address, M31::from(0)); // unused
        assert_eq!(state_data.memory_args[2].address, M31::from(0)); // unused
        assert_eq!(state_data.memory_args[3].address, M31::from(0)); // unused
    }

    #[test]
    fn test_push_instr_unknown_opcode() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(0),
            fp: M31::from(0),
        };
        let clock = 1;

        // Create memory entry with invalid opcode 32
        let memory_entries = vec![
            create_mem_entry(0, [32, 0, 0, 0]), // invalid opcode
        ];

        let result = instructions.push_instr(
            create_memory_iterator(memory_entries),
            state,
            clock,
            &mut memory_cache,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            InstructionError::UnknownOpcode(opcode) => assert_eq!(opcode, 32),
            _ => panic!("Expected UnknownOpcode error"),
        }
    }

    #[test]
    fn test_push_instr_multiple_instructions() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let clock = 1;

        // Push store_imm instruction
        let state1 = VmRegisters {
            pc: M31::from(0),
            fp: M31::from(100),
        };
        let memory_entries1 = vec![
            create_mem_entry(0, [6, 0, 0, 0]),  // store_imm opcode
            create_mem_entry(1, [42, 0, 0, 0]), // result
        ];
        instructions
            .push_instr(
                create_memory_iterator(memory_entries1),
                state1,
                clock,
                &mut memory_cache,
            )
            .unwrap();

        // Push call_abs_imm instruction
        let state2 = VmRegisters {
            pc: M31::from(1),
            fp: M31::from(100),
        };
        let memory_entries2 = vec![
            create_mem_entry(2, [12, 0, 0, 0]), // call_abs_imm opcode
        ];
        instructions
            .push_instr(
                create_memory_iterator(memory_entries2),
                state2,
                clock + 1,
                &mut memory_cache,
            )
            .unwrap();

        // Push ret instruction
        let state3 = VmRegisters {
            pc: M31::from(2),
            fp: M31::from(100),
        };
        let memory_entries3 = vec![
            create_mem_entry(3, [15, 0, 0, 0]), // ret opcode
        ];
        instructions
            .push_instr(
                create_memory_iterator(memory_entries3),
                state3,
                clock + 2,
                &mut memory_cache,
            )
            .unwrap();

        // Verify all instructions were stored in correct vectors
        assert_eq!(instructions.states_by_opcodes.store_imm.len(), 1);
        assert_eq!(instructions.states_by_opcodes.call_abs_imm.len(), 1);
        assert_eq!(instructions.states_by_opcodes.ret.len(), 1);

        // Verify states are correct
        assert_eq!(
            instructions.states_by_opcodes.store_imm[0].registers.pc,
            M31::from(0)
        );
        assert_eq!(
            instructions.states_by_opcodes.call_abs_imm[0].registers.pc,
            M31::from(1)
        );
        assert_eq!(
            instructions.states_by_opcodes.ret[0].registers.pc,
            M31::from(2)
        );
    }

    #[test]
    fn test_clock_progression() {
        let mut instructions = Instructions::default();
        let mut memory_cache = MemoryCache::default();
        let state = VmRegisters {
            pc: M31::from(0),
            fp: M31::from(0),
        };

        // Push multiple instructions with different clocks
        for i in 0..3 {
            let memory_entries = vec![
                create_mem_entry(i, [6, 0, 0, 0]),
                create_mem_entry(i + 10, [42, 0, 0, 0]),
            ];
            instructions
                .push_instr(
                    create_memory_iterator(memory_entries),
                    state,
                    i + 1,
                    &mut memory_cache,
                )
                .unwrap();
        }

        assert_eq!(instructions.states_by_opcodes.store_imm.len(), 3);

        // Verify clocks are correct
        for (i, state_data) in instructions.states_by_opcodes.store_imm.iter().enumerate() {
            assert_eq!(state_data.memory_args[0].clock, M31::from((i + 1) as u32)); // instruction clock
            assert_eq!(state_data.memory_args[1].clock, M31::from((i + 1) as u32));
            // result clock
        }
    }

    #[test]
    fn test_opcode_from_array() {
        let instruction_value = [42, 1, 2, 3];
        let opcode = Opcode::from(instruction_value);
        assert_eq!(opcode.id(), 42);
    }

    #[test]
    fn test_opcode_from_array_ref() {
        let instruction_value = [17, 4, 5, 6];
        let opcode = Opcode::from(&instruction_value);
        assert_eq!(opcode.id(), 17);
    }

    #[test]
    fn test_opcode_from_zero() {
        let instruction_value = [0, 1, 2, 3];
        let opcode = Opcode::from(instruction_value);
        assert_eq!(opcode.id(), 0);
    }

    #[test]
    fn test_opcode_equality() {
        let opcode1 = Opcode::from([5, 0, 0, 0]);
        let opcode2 = Opcode::from([5, 1, 2, 3]);
        let opcode3 = Opcode::from([6, 0, 0, 0]);

        assert_eq!(opcode1, opcode2); // Same opcode ID, different other values
        assert_ne!(opcode1, opcode3); // Different opcode ID
    }

    #[test]
    fn test_opcode_debug() {
        let opcode = Opcode::from([12, 0, 0, 0]);
        let debug_str = format!("{:?}", opcode);
        assert!(debug_str.contains("12"));
    }
}
