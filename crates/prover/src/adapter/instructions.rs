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
