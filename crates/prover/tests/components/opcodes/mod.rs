//! Opcode constraint tests organized by opcode type.

// Common test utilities
mod common;

// Store operations
mod store_deref_fp;
mod store_double_deref_fp;
mod store_imm;

// Arithmetic operations
mod store_add_fp_fp;
mod store_add_fp_fp_inplace;
mod store_add_fp_imm;
mod store_add_fp_imm_inplace;
mod store_div_fp_fp;
mod store_div_fp_imm;
mod store_mul_fp_fp;
mod store_mul_fp_imm;
mod store_sub_fp_fp;
mod store_sub_fp_imm;

// Absolute jump operations
mod jmp_abs_add_fp_fp;
mod jmp_abs_add_fp_imm;
mod jmp_abs_deref_fp;
mod jmp_abs_double_deref_fp;
mod jmp_abs_imm;
mod jmp_abs_mul_fp_fp;
mod jmp_abs_mul_fp_imm;

// Relative jump operations
mod jmp_rel_add_fp_fp;
mod jmp_rel_add_fp_imm;
mod jmp_rel_deref_fp;
mod jmp_rel_double_deref_fp;
mod jmp_rel_imm;
mod jmp_rel_mul_fp_fp;
mod jmp_rel_mul_fp_imm;

// Conditional jump operations
mod jnz_fp_fp;
mod jnz_fp_fp_taken;
mod jnz_fp_imm;
mod jnz_fp_imm_taken;

// Call operations
mod call_abs_fp;
mod call_abs_imm;
mod call_rel_fp;
mod call_rel_imm;

// Return operation
mod ret;
