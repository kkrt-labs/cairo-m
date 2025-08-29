//! Centralized opcode selection for normalized operations.
//! Keeps a single source of truth for opcode numbers by shape.

use crate::{CodegenError, CodegenResult};
use cairo_m_common::instruction::*;
use cairo_m_compiler_mir::BinaryOp;

// Felt op selection
pub fn felt_fp_fp(op: BinaryOp) -> CodegenResult<u32> {
    match op {
        BinaryOp::Add => Ok(STORE_ADD_FP_FP),
        BinaryOp::Sub => Ok(STORE_SUB_FP_FP),
        BinaryOp::Mul => Ok(STORE_MUL_FP_FP),
        BinaryOp::Div => Ok(STORE_DIV_FP_FP),
        _ => Err(CodegenError::UnsupportedInstruction(format!(
            "Invalid felt fp-fp op: {op:?}"
        ))),
    }
}

pub fn felt_fp_imm(op: BinaryOp) -> CodegenResult<u32> {
    match op {
        BinaryOp::Add => Ok(STORE_ADD_FP_IMM),
        BinaryOp::Sub => Ok(STORE_ADD_FP_IMM), // Sub compiled as add with neg imm
        BinaryOp::Mul => Ok(STORE_MUL_FP_IMM),
        BinaryOp::Div => Ok(STORE_MUL_FP_IMM), // Div by imm compiled as mul by inverse
        _ => Err(CodegenError::UnsupportedInstruction(format!(
            "Invalid felt fp-imm op: {op:?}"
        ))),
    }
}

// U32 op selection
pub fn u32_fp_fp(op: BinaryOp) -> CodegenResult<u32> {
    match op {
        BinaryOp::U32Add => Ok(U32_STORE_ADD_FP_FP),
        BinaryOp::U32Sub => Ok(U32_STORE_SUB_FP_FP),
        BinaryOp::U32Mul => Ok(U32_STORE_MUL_FP_FP),
        BinaryOp::U32Div => Ok(U32_STORE_DIV_FP_FP),
        BinaryOp::U32Eq => Ok(U32_STORE_EQ_FP_FP),
        BinaryOp::U32Less => Ok(U32_STORE_LT_FP_FP),
        BinaryOp::U32BitwiseAnd => Ok(U32_STORE_AND_FP_FP),
        BinaryOp::U32BitwiseOr => Ok(U32_STORE_OR_FP_FP),
        BinaryOp::U32BitwiseXor => Ok(U32_STORE_XOR_FP_FP),
        _ => Err(CodegenError::UnsupportedInstruction(format!(
            "Invalid u32 fp-fp op: {op:?}"
        ))),
    }
}

pub fn u32_fp_imm(op: BinaryOp) -> CodegenResult<u32> {
    match op {
        BinaryOp::U32Add => Ok(U32_STORE_ADD_FP_IMM),
        BinaryOp::U32Sub => Ok(U32_STORE_ADD_FP_IMM), // two's complement imm
        BinaryOp::U32Mul => Ok(U32_STORE_MUL_FP_IMM),
        BinaryOp::U32Div => Ok(U32_STORE_DIV_FP_IMM),
        BinaryOp::U32Eq => Ok(U32_STORE_EQ_FP_IMM),
        BinaryOp::U32Neq => Ok(U32_STORE_EQ_FP_IMM), // complement after
        BinaryOp::U32Greater => Ok(U32_STORE_LT_FP_IMM), // use lt with biased imm
        BinaryOp::U32GreaterEqual => Ok(U32_STORE_LT_FP_IMM), // complement of lt
        BinaryOp::U32Less => Ok(U32_STORE_LT_FP_IMM),
        BinaryOp::U32LessEqual => Ok(U32_STORE_LT_FP_IMM), // lt with bias
        BinaryOp::U32BitwiseAnd => Ok(U32_STORE_AND_FP_IMM),
        BinaryOp::U32BitwiseOr => Ok(U32_STORE_OR_FP_IMM),
        BinaryOp::U32BitwiseXor => Ok(U32_STORE_XOR_FP_IMM),
        _ => Err(CodegenError::UnsupportedInstruction(format!(
            "Invalid u32 fp-imm op: {op:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairo_m_compiler_mir::BinaryOp;

    #[test]
    fn test_felt_opcode_selection() {
        assert_eq!(felt_fp_fp(BinaryOp::Add).unwrap(), STORE_ADD_FP_FP);
        assert_eq!(felt_fp_fp(BinaryOp::Sub).unwrap(), STORE_SUB_FP_FP);
        assert_eq!(felt_fp_fp(BinaryOp::Mul).unwrap(), STORE_MUL_FP_FP);
        assert_eq!(felt_fp_fp(BinaryOp::Div).unwrap(), STORE_DIV_FP_FP);

        assert_eq!(felt_fp_imm(BinaryOp::Add).unwrap(), STORE_ADD_FP_IMM);
        assert_eq!(felt_fp_imm(BinaryOp::Sub).unwrap(), STORE_ADD_FP_IMM); // neg imm
        assert_eq!(felt_fp_imm(BinaryOp::Mul).unwrap(), STORE_MUL_FP_IMM);
        assert_eq!(felt_fp_imm(BinaryOp::Div).unwrap(), STORE_MUL_FP_IMM); // inverse imm
    }

    #[test]
    fn test_u32_opcode_selection() {
        assert_eq!(u32_fp_fp(BinaryOp::U32Add).unwrap(), U32_STORE_ADD_FP_FP);
        assert_eq!(u32_fp_fp(BinaryOp::U32Sub).unwrap(), U32_STORE_SUB_FP_FP);
        assert_eq!(u32_fp_fp(BinaryOp::U32Mul).unwrap(), U32_STORE_MUL_FP_FP);
        assert_eq!(u32_fp_fp(BinaryOp::U32Div).unwrap(), U32_STORE_DIV_FP_FP);
        assert_eq!(u32_fp_fp(BinaryOp::U32Eq).unwrap(), U32_STORE_EQ_FP_FP);
        assert_eq!(u32_fp_fp(BinaryOp::U32Less).unwrap(), U32_STORE_LT_FP_FP);
        // Only Eq/Lt are valid comparisons for fp-fp in builder path

        assert_eq!(u32_fp_imm(BinaryOp::U32Add).unwrap(), U32_STORE_ADD_FP_IMM);
        assert_eq!(u32_fp_imm(BinaryOp::U32Sub).unwrap(), U32_STORE_ADD_FP_IMM); // two's complement
        assert_eq!(u32_fp_imm(BinaryOp::U32Mul).unwrap(), U32_STORE_MUL_FP_IMM);
        assert_eq!(u32_fp_imm(BinaryOp::U32Div).unwrap(), U32_STORE_DIV_FP_IMM);
        assert_eq!(u32_fp_imm(BinaryOp::U32Eq).unwrap(), U32_STORE_EQ_FP_IMM);
        assert_eq!(u32_fp_imm(BinaryOp::U32Less).unwrap(), U32_STORE_LT_FP_IMM);
        // Only Eq/Lt are valid comparisons for fp-imm in builder path
    }
}
