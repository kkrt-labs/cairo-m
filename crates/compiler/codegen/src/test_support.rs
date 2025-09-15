use crate::InstructionBuilder;
use cairo_m_common::instruction::*;
use stwo_prover::core::fields::m31::M31;

/// Simple frame memory model for tests (fp-relative, non-negative offsets only).
pub struct Mem {
    slots: Vec<M31>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionError {
    InvalidOperands,
    DivisionByZero,
}

impl Mem {
    pub fn new(size: usize) -> Self {
        Self {
            slots: vec![M31::from(0u32); size],
        }
    }
    #[inline]
    pub(crate) fn get(&self, off: i32) -> M31 {
        self.slots[off as usize]
    }
    #[inline]
    pub(crate) fn set(&mut self, off: i32, val: M31) {
        self.slots[off as usize] = val;
    }

    // u32 is stored in 2 consecutive slots: lo,hi (16-bit each)
    pub(crate) fn set_u32(&mut self, off: i32, val: u32) {
        let lo = val & 0xFFFF;
        let hi = (val >> 16) & 0xFFFF;
        self.set(off, M31::from(lo));
        self.set(off + 1, M31::from(hi));
    }
    pub(crate) fn get_u32(&self, off: i32) -> u32 {
        let lo = self.get(off).0 & 0xFFFF;
        let hi = self.get(off + 1).0 & 0xFFFF;
        (hi << 16) | lo
    }
}

/// Execute a limited subset of InstructionBuilders on a simple memory.
/// Only supports opcodes used by u32 arithmetic/compare paths and simple felt ops used around them.
pub fn exec(mem: &mut Mem, instrs: &[InstructionBuilder]) -> Result<(), ExecutionError> {
    for ib in instrs {
        let instr = ib.inner_instr();
        match instr {
            Instruction::StoreImm { imm, dst_off } => {
                mem.set(dst_off.0 as i32, M31::from(imm.0));
            }
            Instruction::StoreAddFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get(src0_off.0 as i32);
                let b = mem.get(src1_off.0 as i32);
                let val = a + b;
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreSubFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get(src0_off.0 as i32);
                let b = mem.get(src1_off.0 as i32);
                let val = a - b;
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreMulFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get(src0_off.0 as i32);
                let b = mem.get(src1_off.0 as i32);
                let val = a * b;
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreDivFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get(src0_off.0 as i32);
                let b = mem.get(src1_off.0 as i32);
                if b == M31::from(0) {
                    return Err(ExecutionError::DivisionByZero);
                }
                let val = a / b;
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreAddFpImm {
                src_off,
                imm,
                dst_off,
            } => {
                let a = mem.get(src_off.0 as i32);
                let val = a + M31::from(imm.0);
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreMulFpImm {
                src_off,
                imm,
                dst_off,
            } => {
                let a = mem.get(src_off.0 as i32);
                let val = a * M31::from(imm.0);
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreLowerThanFpImm {
                src_off,
                imm,
                dst_off,
            } => {
                let a = mem.get(src_off.0 as i32);
                let val = a < M31::from(imm.0);
                mem.set(dst_off.0 as i32, M31::from(val as u32));
            }
            Instruction::AssertEqFpImm { src_off, imm } => {
                let a = mem.get(src_off.0 as i32);
                let val = a == M31::from(imm.0);
                assert!(val);
            }
            Instruction::StoreDoubleDerefFp {
                base_off,
                imm,
                dst_off,
            } => {
                let a = mem.get(base_off.0 as i32);
                let val = mem.get(a.0 as i32 + imm.0 as i32);
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreDoubleDerefFpFp {
                base_off,
                offset_off,
                dst_off,
            } => {
                let a = mem.get(base_off.0 as i32);
                let b = mem.get(offset_off.0 as i32);
                let val = mem.get(a.0 as i32 + b.0 as i32);
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::StoreFramePointer { imm, dst_off } => {
                let val = mem.get(imm.0 as i32);
                mem.set(dst_off.0 as i32, val);
            }
            Instruction::U32StoreAddFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a.wrapping_add(b);
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreSubFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a.wrapping_sub(b);
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreMulFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a.wrapping_mul(b);
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreDivRemFpFp {
                src0_off,
                src1_off,
                dst_off,
                dst_rem_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                if b == 0 {
                    return Err(ExecutionError::DivisionByZero);
                }
                let val = a.wrapping_div(b);
                mem.set_u32(dst_off.0 as i32, val);
                let rem = a.wrapping_rem(b);
                mem.set_u32(dst_rem_off.0 as i32, rem);
            }
            Instruction::U32StoreAddFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let a = mem.get_u32(src_off.0 as i32);
                let result = a.wrapping_add(imm);
                mem.set_u32(dst_off.0 as i32, result);
            }
            Instruction::U32StoreMulFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let result = a.wrapping_mul(imm);
                mem.set_u32(dst_off.0 as i32, result);
            }
            Instruction::U32StoreDivRemFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
                dst_rem_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                if imm == 0 {
                    return Err(ExecutionError::DivisionByZero);
                }
                let q = a.wrapping_div(imm);
                let r = a.wrapping_rem(imm);
                mem.set_u32(dst_off.0 as i32, q);
                mem.set_u32(dst_rem_off.0 as i32, r);
            }
            Instruction::U32StoreImm {
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                mem.set_u32(dst_off.0 as i32, imm);
            }
            Instruction::U32StoreEqFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a == b;
                mem.set(dst_off.0 as i32, M31::from(val as u32));
            }
            Instruction::U32StoreLtFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a < b;
                mem.set(dst_off.0 as i32, M31::from(val as u32));
            }
            Instruction::U32StoreEqFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let val = a == imm;
                mem.set(dst_off.0 as i32, M31::from(val as u32));
            }
            Instruction::U32StoreLtFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let val = a < imm;
                mem.set(dst_off.0 as i32, M31::from(val as u32));
            }

            Instruction::U32StoreAndFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a & b;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreOrFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a | b;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreXorFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                let a = mem.get_u32(src0_off.0 as i32);
                let b = mem.get_u32(src1_off.0 as i32);
                let val = a ^ b;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreAndFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let val = a & imm;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreOrFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let val = a | imm;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::U32StoreXorFpImm {
                src_off,
                imm_lo,
                imm_hi,
                dst_off,
            } => {
                let a = mem.get_u32(src_off.0 as i32);
                let imm = imm_lo.0 | (imm_hi.0 << 16);
                let val = a ^ imm;
                mem.set_u32(dst_off.0 as i32, val);
            }
            Instruction::StoreToDoubleDerefFpImm {
                src_off,
                imm,
                base_off,
            } => {
                let a = mem.get(src_off.0 as i32);
                let val = mem.get(a.0 as i32 + imm.0 as i32);
                mem.set(base_off.0 as i32, val);
            }
            Instruction::StoreToDoubleDerefFpFp {
                src_off,
                base_off,
                offset_off,
            } => {
                let a = mem.get(src_off.0 as i32);
                let b = mem.get(base_off.0 as i32);
                let val = mem.get(a.0 as i32 + b.0 as i32);
                mem.set(offset_off.0 as i32, val);
            }
            _ => {
                panic!("Unsupported instruction: {:?} in test interpreter", instr);
            }
        }
    }
    Ok(())
}
