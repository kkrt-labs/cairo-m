use stwo_prover::core::fields::qm31::QM31;

use super::io::{MemEntry, TraceEntry};
use super::memory::MemoryCache;

#[derive(Clone, Copy, Debug, Default)]
pub struct VmState {
    pub pc: u32,
    pub fp: u32,
}

impl From<TraceEntry> for VmState {
    fn from(entry: TraceEntry) -> Self {
        Self {
            pc: entry.pc,
            fp: entry.fp,
        }
    }
}

//TODO: add prev value
#[derive(Debug, Default)]
pub struct StateData {
    pub state: VmState,
    pub memory_args: [(u32, QM31, QM31, u32, u32); 4], // (addr, val, prev_val, prev_clock, clock)
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
    pub initial_state: VmState,
    pub final_state: VmState,
    pub states_by_opcodes: StatesByOpcodes,
}

impl Instructions {
    pub fn push_instr<I>(
        &mut self,
        mut memory: I,
        state: VmState,
        clock: u32,
        memory_cache: &mut MemoryCache,
    ) where
        I: Iterator<Item = MemEntry>,
    {
        let mut state_data = StateData {
            state,
            memory_args: Default::default(),
        };

        let instruction = memory.next().unwrap();
        let opcode_id = instruction.val[0];
        state_data.memory_args[0] = memory_cache.push(instruction, clock);

        match opcode_id {
            0 => {
                // store_add_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory.next().unwrap();
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_add_fp_fp.push(state_data);
            }
            1 => {
                // store_add_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_add_fp_imm.push(state_data);
            }
            2 => {
                // store_sub_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory.next().unwrap();
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_sub_fp_fp.push(state_data);
            }
            3 => {
                // store_sub_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_sub_fp_imm.push(state_data);
            }
            4 => {
                // store_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_deref_fp.push(state_data);
            }
            5 => {
                // store_double_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory.next().unwrap();
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes
                    .store_double_deref_fp
                    .push(state_data);
            }
            6 => {
                // store_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.store_imm.push(state_data);
            }
            7 => {
                // store_mul_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory.next().unwrap();
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_mul_fp_fp.push(state_data);
            }
            8 => {
                // store_mul_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_mul_fp_imm.push(state_data);
            }
            9 => {
                // store_div_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                let mem3 = memory.next().unwrap();
                state_data.memory_args[3] = memory_cache.push(mem3, clock);
                self.states_by_opcodes.store_div_fp_fp.push(state_data);
            }
            10 => {
                // store_div_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.store_div_fp_imm.push(state_data);
            }
            11 => {
                // call_abs_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.call_abs_fp.push(state_data);
            }
            12 => {
                // call_abs_imm
                self.states_by_opcodes.call_abs_imm.push(state_data);
            }
            13 => {
                // call_rel_fp
                let mem1 = memory.next().unwrap();
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
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_abs_add_fp_fp.push(state_data);
            }
            17 => {
                // jmp_abs_add_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_add_fp_imm.push(state_data);
            }
            18 => {
                // jmp_abs_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_deref_fp.push(state_data);
            }
            19 => {
                // jmp_abs_double_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
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
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_abs_mul_fp_fp.push(state_data);
            }
            22 => {
                // jmp_abs_mul_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_abs_mul_fp_imm.push(state_data);
            }
            23 => {
                // jmp_rel_add_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_rel_add_fp_fp.push(state_data);
            }
            24 => {
                // jmp_rel_add_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_add_fp_imm.push(state_data);
            }
            25 => {
                // jmp_rel_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_deref_fp.push(state_data);
            }
            26 => {
                // jmp_rel_double_deref_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
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
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jmp_rel_mul_fp_fp.push(state_data);
            }
            29 => {
                // jmp_rel_mul_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jmp_rel_mul_fp_imm.push(state_data);
            }
            30 => {
                // jnz_fp_fp
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                let mem2 = memory.next().unwrap();
                state_data.memory_args[2] = memory_cache.push(mem2, clock);
                self.states_by_opcodes.jnz_fp_fp.push(state_data);
            }
            31 => {
                // jnz_fp_imm
                let mem1 = memory.next().unwrap();
                state_data.memory_args[1] = memory_cache.push(mem1, clock);
                self.states_by_opcodes.jnz_fp_imm.push(state_data);
            }
            _ => panic!("Unknown opcode: {opcode_id}"),
        }
    }
}
