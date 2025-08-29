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
        match ib.opcode {
            // Felt
            STORE_IMM => {
                let imm = ib.op0().unwrap() as u32;
                let dst = ib.op1().unwrap();
                mem.set(dst, M31::from(imm));
            }
            STORE_ADD_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let val = mem.get(a) + mem.get(b);
                mem.set(dst, val);
            }
            STORE_ADD_FP_IMM => {
                let src = ib.op0().unwrap();
                let imm = ib.op1().unwrap() as u32;
                let dst = ib.op2().unwrap();
                let val = mem.get(src) + M31::from(imm);
                mem.set(dst, val);
            }
            STORE_SUB_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let val = mem.get(a) - mem.get(b);
                mem.set(dst, val);
            }
            STORE_MUL_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let val = mem.get(a) * mem.get(b);
                mem.set(dst, val);
            }
            STORE_DIV_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let divisor = mem.get(b);
                if divisor.0 == 0 {
                    return Err(ExecutionError::DivisionByZero);
                }
                let val = mem.get(a) * divisor.inverse();
                mem.set(dst, val);
            }
            STORE_MUL_FP_IMM => {
                let src = ib.op0().unwrap();
                let imm = ib.op1().unwrap() as u32;
                let dst = ib.op2().unwrap();
                let val = mem.get(src) * M31::from(imm);
                mem.set(dst, val);
            }

            // U32 immediates
            U32_STORE_IMM => {
                let lo = ib.op0().unwrap() as u32;
                let hi = ib.op1().unwrap() as u32;
                let dst = ib.op2().unwrap();
                mem.set(dst, M31::from(lo));
                mem.set(dst + 1, M31::from(hi));
            }

            // U32 fp-imm
            U32_STORE_ADD_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                mem.set_u32(dst, x.wrapping_add(imm));
            }
            U32_STORE_MUL_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                mem.set_u32(dst, x.wrapping_mul(imm));
            }
            U32_STORE_DIV_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                if imm == 0 {
                    return Err(ExecutionError::DivisionByZero);
                }
                let y = x.wrapping_div(imm);
                mem.set_u32(dst, y);
            }

            U32_STORE_AND_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                mem.set_u32(dst, x & imm);
            }
            U32_STORE_OR_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                mem.set_u32(dst, x | imm);
            }
            U32_STORE_XOR_FP_IMM => {
                let src = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let x = mem.get_u32(src);
                mem.set_u32(dst, x ^ imm);
            }

            // U32 fp-fp arithmetic
            U32_STORE_ADD_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x.wrapping_add(y));
            }
            U32_STORE_SUB_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x.wrapping_sub(y));
            }
            U32_STORE_MUL_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x.wrapping_mul(y));
            }
            U32_STORE_DIV_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                if y == 0 {
                    return Err(ExecutionError::DivisionByZero);
                }
                let z = x.wrapping_div(y);
                mem.set_u32(dst, z);
            }

            U32_STORE_AND_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x & y);
            }
            U32_STORE_OR_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x | y);
            }
            U32_STORE_XOR_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let x = mem.get_u32(a);
                let y = mem.get_u32(b);
                mem.set_u32(dst, x ^ y);
            }

            // U32 comparisons (felt result in one slot)
            U32_STORE_EQ_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let truth = (mem.get_u32(a) == mem.get_u32(b)) as u32;
                mem.set(dst, M31::from(truth));
            }
            U32_STORE_EQ_FP_IMM => {
                let a = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let truth = (mem.get_u32(a) == imm) as u32;
                mem.set(dst, M31::from(truth));
            }
            U32_STORE_LT_FP_FP => {
                let a = ib.op0().unwrap();
                let b = ib.op1().unwrap();
                let dst = ib.op2().unwrap();
                let truth = (mem.get_u32(a) < mem.get_u32(b)) as u32;
                mem.set(dst, M31::from(truth));
            }
            U32_STORE_LT_FP_IMM => {
                let a = ib.op0().unwrap();
                let lo = ib.op1().unwrap() as u32;
                let hi = ib.op2().unwrap() as u32;
                let dst = ib.op3();
                let imm = hi << 16 | (lo & 0xFFFF);
                let truth = (mem.get_u32(a) < imm) as u32;
                mem.set(dst, M31::from(truth));
            }

            // Others used in comparison complement sequences handled above
            _ => {
                // For unsupported opcodes in these tests, panic to catch misses early
                panic!("Unsupported opcode in test interpreter: {}", ib.opcode);
            }
        }
    }
    Ok(())
}

// Convenience: access 4th operand
trait Op3Ext {
    fn op3(&self) -> i32;
}
impl Op3Ext for InstructionBuilder {
    fn op3(&self) -> i32 {
        match self.operands.get(3) {
            Some(crate::Operand::Literal(v)) => *v,
            _ => panic!("expected 4 operands with last literal"),
        }
    }
}
