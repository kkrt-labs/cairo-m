//! Codegen Passes (post-builder)
//!
//! After generating CASM for all basic blocks of a function, we run a small
//! sequence of local transformation passes over the function’s instruction list.
//!
//! In particular, we enforce the following correctness requirement derived from
//! the prover components (see `store_fp_fp.rs` and `store_fp_imm.rs`):
//!
//! - Avoid two reads from the same fp-relative cell in a single instruction.
//!   Two reads (e.g., `[fp+o] + [fp+o]`) are invalid because the first read sets
//!   the cell’s clock to `clk`, making a second read require `clk < clk`.
//!   However, a read followed by a write to the same cell (dst == src) is fine.
//!   For 2-slot u32 values, treat each source as a 2-slot range: any overlap
//!   between the two source ranges is disallowed (two reads), but overlap between
//!   destination and a source is allowed (read then write).
//!
//! The `DeduplicateOperandsPass` rewrites offending instructions by inserting
//! temporary copies for one of the sources when both sources alias (or overlap
//! for u32). Destination aliasing is not rewritten since read-then-write is OK.
//!
//! We also include tiny peephole canonicalizations (e.g., `* 1` -> `+ 0`,
//! `* 0` -> `StoreImm(0)`/`U32StoreImm(0)`) that reduce unnecessary reads while
//! preserving semantics and the prover’s expected read/write patterns per
//! opcode.

use crate::{CasmBuilder, CodegenError, CodegenResult, InstructionBuilder, Label};
use cairo_m_common::Instruction as CasmInstr;
use stwo_prover::core::fields::m31::M31;

/// A transformation pass that runs over a function’s instruction list.
pub trait CodegenPass {
    fn name(&self) -> &str;
    fn run(&self, builder: &mut CasmBuilder) -> CodegenResult<()>;
}

/// Pass 1: Ensure no single instruction reuses the same memory cell twice.
struct DeduplicateOperandsPass;

impl CodegenPass for DeduplicateOperandsPass {
    fn name(&self) -> &str {
        "deduplicate-operands"
    }

    fn run(&self, builder: &mut CasmBuilder) -> CodegenResult<()> {
        // Build a fresh instruction list and remap labels accordingly.
        let old_instrs = builder.instructions().to_vec();
        let mut new_instrs: Vec<InstructionBuilder> = Vec::with_capacity(old_instrs.len());
        let mut index_mapping: Vec<Option<std::ops::Range<usize>>> =
            Vec::with_capacity(old_instrs.len());

        for instr in old_instrs.iter() {
            let start = new_instrs.len();
            let repl = rewrite_instruction(builder, instr)?;
            if repl.is_empty() {
                index_mapping.push(None);
            } else {
                let end = start + repl.len();
                new_instrs.extend(repl);
                index_mapping.push(Some(start..end));
            }
        }

        // Remap labels to point at first new instruction of a replaced range.
        for Label { address, .. } in builder.labels_mut().iter_mut() {
            if let Some(old_idx) = *address {
                if old_idx < index_mapping.len() {
                    if let Some(range) = &index_mapping[old_idx] {
                        *address = Some(range.start);
                    } else {
                        // If the labeled instruction vanished (shouldn't), point to next valid start.
                        let next = index_mapping
                            .iter()
                            .skip(old_idx + 1)
                            .find_map(|e| e.as_ref().map(|r| r.start));
                        *address = next;
                    }
                }
            }
        }

        *builder.instructions_mut() = new_instrs;
        Ok(())
    }
}

/// Pass 2: Canonicalize a few immediate arithmetic patterns.
/// - felt: `mul imm=1` -> `add imm=0`
/// - felt: `mul imm=0` -> `store_imm 0`
/// - u32:  `mul imm=1` -> `add imm=0`
/// - u32:  `mul imm=0` -> `u32_store_imm 0`
struct CanonicalizeImmediateOpsPass;

impl CodegenPass for CanonicalizeImmediateOpsPass {
    fn name(&self) -> &str {
        "canonicalize-imm-ops"
    }

    fn run(&self, builder: &mut CasmBuilder) -> CodegenResult<()> {
        let n = builder.instructions().len();
        for i in 0..n {
            // Build a replacement if needed; then swap in place.
            let old_label = builder.instructions()[i].get_label().map(|s| s.to_string());
            let new = match builder.instructions()[i].inner_instr() {
                // felt: mul by 1 -> add 0
                CasmInstr::StoreMulFpImm {
                    src_off,
                    imm,
                    dst_off,
                } if imm.0 == 1 => Some(
                    InstructionBuilder::from(CasmInstr::StoreAddFpImm {
                        src_off: *src_off,
                        imm: M31::from(0),
                        dst_off: *dst_off,
                    })
                    .with_comment(format!("[fp + {}] = [fp + {}] + 0", dst_off.0, src_off.0)),
                ),
                // felt: mul by 0 -> store imm 0
                CasmInstr::StoreMulFpImm { dst_off, imm, .. } if imm.0 == 0 => Some(
                    InstructionBuilder::from(CasmInstr::StoreImm {
                        imm: M31::from(0),
                        dst_off: *dst_off,
                    })
                    .with_comment(format!("[fp + {}] = 0", dst_off.0)),
                ),

                // u32: mul by 1 -> add 0
                CasmInstr::U32StoreMulFpImm {
                    src_off,
                    imm_lo,
                    imm_hi,
                    dst_off,
                } if imm_lo.0 == 1 && imm_hi.0 == 0 => Some(
                    InstructionBuilder::from(CasmInstr::U32StoreAddFpImm {
                        src_off: *src_off,
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: *dst_off,
                    })
                    .with_comment(format!(
                        "u32([fp + {}], [fp + {}]) = u32([fp + {}], [fp + {}]) + u32(0, 0)",
                        dst_off.0,
                        dst_off.0 + 1,
                        src_off.0,
                        src_off.0 + 1
                    )),
                ),
                // u32: mul by 0 -> store imm 0
                CasmInstr::U32StoreMulFpImm {
                    dst_off,
                    imm_lo,
                    imm_hi,
                    ..
                } if imm_lo.0 == 0 && imm_hi.0 == 0 => Some(
                    InstructionBuilder::from(CasmInstr::U32StoreImm {
                        imm_lo: M31::from(0),
                        imm_hi: M31::from(0),
                        dst_off: *dst_off,
                    })
                    .with_comment(format!(
                        "[fp + {}], [fp + {}] = u32(0)",
                        dst_off.0,
                        dst_off.0 + 1
                    )),
                ),

                _ => None,
            };

            if let Some(mut new_ib) = new {
                if let Some(lbl) = old_label {
                    new_ib = new_ib.with_label(lbl);
                }
                // Replace in place
                builder.instructions_mut()[i] = new_ib;
            }
        }

        Ok(())
    }
}

/// Run the default pass pipeline on a single function’s CASM.
pub fn run_all(builder: &mut CasmBuilder) -> CodegenResult<()> {
    let passes: [&dyn CodegenPass; 2] = [&DeduplicateOperandsPass, &CanonicalizeImmediateOpsPass];
    for p in passes.into_iter() {
        p.run(builder)?;
    }
    Ok(())
}

// ===== Helpers for DeduplicateOperandsPass =====

fn rewrite_instruction(
    builder: &mut CasmBuilder,
    instr: &InstructionBuilder,
) -> CodegenResult<Vec<InstructionBuilder>> {
    match instr.inner_instr() {
        // felt fp+fp
        CasmInstr::StoreAddFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::StoreSubFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::StoreMulFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::StoreDivFpFp {
            src0_off,
            src1_off,
            dst_off,
        } => rewrite_felt_fp_fp(builder, instr, *src0_off, *src1_off, *dst_off),

        // u32 fp+fp arithmetic
        CasmInstr::U32StoreAddFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::U32StoreSubFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::U32StoreMulFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::U32StoreDivRemFpFp {
            src0_off,
            src1_off,
            dst_off,
            dst_rem_off: _, // TODO: handle this
        } => rewrite_u32_fp_fp(builder, instr, *src0_off, *src1_off, *dst_off, false),

        // u32 fp+fp comparisons (felt result)
        CasmInstr::U32StoreEqFpFp {
            src0_off,
            src1_off,
            dst_off,
        }
        | CasmInstr::U32StoreLtFpFp {
            src0_off,
            src1_off,
            dst_off,
        } => rewrite_u32_fp_fp(builder, instr, *src0_off, *src1_off, *dst_off, true),

        _ => Ok(vec![instr.clone()]),
    }
}

fn rebuild_felt_fp_fp(orig: &CasmInstr, a: M31, b: M31, d: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::StoreAddFpFp { .. } => CasmInstr::StoreAddFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::StoreSubFpFp { .. } => CasmInstr::StoreSubFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::StoreMulFpFp { .. } => CasmInstr::StoreMulFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::StoreDivFpFp { .. } => CasmInstr::StoreDivFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        _ => {
            return Err(CodegenError::UnsupportedInstruction(
                "Expected felt fp+fp".into(),
            ))
        }
    })
}

fn rebuild_u32_fp_fp(orig: &CasmInstr, a: M31, b: M31, d: M31) -> CodegenResult<CasmInstr> {
    Ok(match orig {
        CasmInstr::U32StoreAddFpFp { .. } => CasmInstr::U32StoreAddFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::U32StoreSubFpFp { .. } => CasmInstr::U32StoreSubFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::U32StoreMulFpFp { .. } => CasmInstr::U32StoreMulFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::U32StoreDivRemFpFp { .. } => CasmInstr::U32StoreDivRemFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
            dst_rem_off: d, // TODO: handle this
        },
        CasmInstr::U32StoreEqFpFp { .. } => CasmInstr::U32StoreEqFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        CasmInstr::U32StoreLtFpFp { .. } => CasmInstr::U32StoreLtFpFp {
            src0_off: a,
            src1_off: b,
            dst_off: d,
        },
        _ => {
            return Err(CodegenError::UnsupportedInstruction(
                "Expected u32 fp+fp".into(),
            ))
        }
    })
}

const fn u32_fp_fp_op_name(orig: &CasmInstr) -> Option<&'static str> {
    match orig {
        CasmInstr::U32StoreAddFpFp { .. } => Some("U32Add"),
        CasmInstr::U32StoreSubFpFp { .. } => Some("U32Sub"),
        CasmInstr::U32StoreMulFpFp { .. } => Some("U32Mul"),
        CasmInstr::U32StoreDivRemFpFp { .. } => Some("U32Div"),
        CasmInstr::U32StoreEqFpFp { .. } => Some("U32Eq"),
        CasmInstr::U32StoreLtFpFp { .. } => Some("U32Less"),
        _ => None,
    }
}

fn rewrite_felt_fp_fp(
    builder: &mut CasmBuilder,
    orig: &InstructionBuilder,
    src0: M31,
    src1: M31,
    dst: M31,
) -> CodegenResult<Vec<InstructionBuilder>> {
    let o0 = src0.0;
    let o1 = src1.0;
    if o0 == o1 {
        // Two reads from the same cell: copy one source to a temp.
        let t0 = builder.layout_mut().reserve_stack(1);
        let copy = InstructionBuilder::from(CasmInstr::StoreAddFpImm {
            src_off: src0,
            imm: M31::from(0),
            dst_off: M31::from(t0),
        })
        .with_comment(format!("[fp + {t0}] = [fp + {o0}] + 0"));
        let op = InstructionBuilder::from(rebuild_felt_fp_fp(
            orig.inner_instr(),
            M31::from(t0),
            src1,
            dst,
        )?)
        .with_comment(format!(
            "[fp + {}] = [fp + {}] op [fp + {}]",
            dst.0, t0, src1.0
        ));
        Ok(vec![copy, op])
    } else {
        Ok(vec![orig.clone()])
    }
}

const fn u32_overlap(a: u32, b: u32) -> bool {
    // ranges [a,a+1] and [b,b+1] intersect?
    let a0 = a;
    let a1 = a + 1;
    let b0 = b;
    let b1 = b + 1;
    !(a1 < b0 || b1 < a0)
}

fn rewrite_u32_fp_fp(
    builder: &mut CasmBuilder,
    orig: &InstructionBuilder,
    src0: M31,
    src1: M31,
    dst: M31,
    _is_comparison: bool,
) -> CodegenResult<Vec<InstructionBuilder>> {
    let o0 = src0.0;
    let o1 = src1.0;
    if u32_overlap(o0, o1) {
        let t0 = builder.layout_mut().reserve_stack(2);
        let copy = InstructionBuilder::from(CasmInstr::U32StoreAddFpImm {
            src_off: src0,
            imm_lo: M31::from(0),
            imm_hi: M31::from(0),
            dst_off: M31::from(t0),
        })
        .with_comment(format!(
            "u32([fp + {t0}], [fp + {}]) = u32([fp + {o0}], [fp + {}]) + u32(0, 0)",
            t0 + 1,
            o0 + 1
        ));
        let op_instr = rebuild_u32_fp_fp(orig.inner_instr(), M31::from(t0), src1, dst)?;
        let op_name = u32_fp_fp_op_name(orig.inner_instr()).unwrap_or("op");
        let op = InstructionBuilder::from(op_instr).with_comment({
            let s0 = t0;
            let s1 = src1.0;
            let d = dst.0;
            // For comparisons (felt result) vs arithmetic (u32 result)
            let is_cmp = matches!(orig.inner_instr(), CasmInstr::U32StoreEqFpFp { .. } | CasmInstr::U32StoreLtFpFp { .. });
            if is_cmp {
                format!(
                    "[fp + {d}] = u32([fp + {s0}], [fp + {}]) {op_name} u32([fp + {s1}], [fp + {}])",
                    s0 + 1,
                    s1 + 1
                )
            } else {
                format!(
                    "u32([fp + {d}], [fp + {}]) = u32([fp + {s0}], [fp + {}]) {op_name} u32([fp + {s1}], [fp + {}])",
                    d + 1,
                    s0 + 1,
                    s1 + 1
                )
            }
        });
        Ok(vec![copy, op])
    } else {
        Ok(vec![orig.clone()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builder::CasmBuilder, layout::FunctionLayout};
    use cairo_m_common::Instruction as CasmInstr;

    fn run_dedup(instrs: Vec<InstructionBuilder>) -> (CasmBuilder, Vec<InstructionBuilder>) {
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        for i in instrs {
            b.emit_push(i);
        }
        DeduplicateOperandsPass.run(&mut b).unwrap();
        let out = b.instructions().to_vec();
        (b, out)
    }

    #[test]
    fn felt_fp_fp_all_same_offsets() {
        // [fp+5] = [fp+5] + [fp+5]
        let instr = InstructionBuilder::from(CasmInstr::StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(5),
            dst_off: M31::from(5),
        });
        let (_b, out) = run_dedup(vec![instr]);
        assert_eq!(out.len(), 2);
        match out[0].inner_instr() {
            CasmInstr::StoreAddFpImm { .. } => (),
            _ => panic!("copy"),
        }
        match out[1].inner_instr() {
            CasmInstr::StoreAddFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                assert_ne!(src0_off.0, 5);
                assert_eq!(src1_off.0, 5);
                assert_eq!(dst_off.0, 5);
            }
            _ => panic!("final op not felt fp+fp"),
        }
    }

    #[test]
    fn felt_fp_imm_in_place_ok() {
        // dst == src is allowed for felt fp+imm
        let instr = InstructionBuilder::from(CasmInstr::StoreMulFpImm {
            src_off: M31::from(7),
            imm: M31::from(9),
            dst_off: M31::from(7),
        });
        let (_b, out) = run_dedup(vec![instr]);
        assert_eq!(out.len(), 1);
        match out[0].inner_instr() {
            CasmInstr::StoreMulFpImm { .. } => (),
            _ => panic!("kept"),
        }
    }

    #[test]
    fn u32_fp_fp_overlap_sources() {
        // src0 = 5/6, src1 = 6/7: overlap on 6
        let instr = InstructionBuilder::from(CasmInstr::U32StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(6),
            dst_off: M31::from(20),
        });
        let (_b, out) = run_dedup(vec![instr]);
        assert_eq!(out.len(), 2);
        match out[0].inner_instr() {
            CasmInstr::U32StoreAddFpImm { .. } => (),
            _ => panic!("copy s0"),
        }
        match out[1].inner_instr() {
            CasmInstr::U32StoreAddFpFp { .. } => (),
            _ => panic!("final"),
        }
    }

    #[test]
    fn u32_fp_imm_overlap_with_dest_ok() {
        // Overlap with destination is allowed (single read)
        let instr = InstructionBuilder::from(CasmInstr::U32StoreMulFpImm {
            src_off: M31::from(10),
            imm_lo: M31::from(2),
            imm_hi: M31::from(0),
            dst_off: M31::from(11),
        });
        let (_b, out) = run_dedup(vec![instr]);
        assert_eq!(out.len(), 1);
        match out[0].inner_instr() {
            CasmInstr::U32StoreMulFpImm { .. } => (),
            _ => panic!("kept"),
        }
    }

    #[test]
    fn felt_fp_fp_dest_equals_src0_no_change() {
        // [fp+5] = [fp+5] op [fp+6] (dst == src0): should NOT rewrite
        let instr = InstructionBuilder::from(CasmInstr::StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(6),
            dst_off: M31::from(5),
        });
        let (_b, out) = run_dedup(vec![instr.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].inner_instr(), instr.inner_instr());
    }

    #[test]
    fn felt_fp_fp_dest_equals_src1_no_change() {
        // [fp+6] = [fp+5] op [fp+6] (dst == src1): should NOT rewrite
        let instr = InstructionBuilder::from(CasmInstr::StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(6),
            dst_off: M31::from(6),
        });
        let (_b, out) = run_dedup(vec![instr.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].inner_instr(), instr.inner_instr());
    }

    #[test]
    fn felt_fp_fp_srcs_equal_copy_src0_then_op() {
        // [fp+7] = [fp+5] op [fp+5] => copy src0 to temp, then op temp with src1
        let instr = InstructionBuilder::from(CasmInstr::StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(5),
            dst_off: M31::from(7),
        });
        let (_b, out) = run_dedup(vec![instr]);
        assert_eq!(out.len(), 2);
        let temp = match out[0].inner_instr() {
            CasmInstr::StoreAddFpImm {
                src_off,
                dst_off,
                imm,
            } => {
                assert_eq!(*src_off, M31::from(5));
                assert_eq!(*imm, M31::from(0));
                dst_off.0
            }
            _ => panic!("expected first copy into temp"),
        };
        match out[1].inner_instr() {
            CasmInstr::StoreAddFpFp {
                src0_off,
                src1_off,
                dst_off,
            } => {
                assert_eq!(*src0_off, M31::from(temp));
                assert_eq!(*src1_off, M31::from(5));
                assert_eq!(*dst_off, M31::from(7));
            }
            _ => panic!("expected final op with temp and src1"),
        }
    }

    #[test]
    fn u32_fp_fp_dest_overlaps_src0_no_change() {
        // src0:10/11, src1:20/21, dst:11/12 overlaps src0 only => allowed
        let instr = InstructionBuilder::from(CasmInstr::U32StoreAddFpFp {
            src0_off: M31::from(10),
            src1_off: M31::from(20),
            dst_off: M31::from(11),
        });
        let (_b, out) = run_dedup(vec![instr.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].inner_instr(), instr.inner_instr());
    }

    #[test]
    fn u32_fp_fp_dest_overlaps_src1_no_change() {
        // src0:10/11, src1:20/21, dst:21/22 overlaps src1 only => allowed
        let instr = InstructionBuilder::from(CasmInstr::U32StoreMulFpFp {
            src0_off: M31::from(10),
            src1_off: M31::from(20),
            dst_off: M31::from(21),
        });
        let (_b, out) = run_dedup(vec![instr.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].inner_instr(), instr.inner_instr());
    }

    #[test]
    fn labels_remapped_on_expansion() {
        // One harmless instr, then label, then an expanding instr
        let mut b = CasmBuilder::new(FunctionLayout::new_for_test(), 0);
        b.emit_push(InstructionBuilder::from(CasmInstr::StoreImm {
            imm: M31::from(1),
            dst_off: M31::from(0),
        }));
        let lbl = Label::new("L_test".to_string());
        b.emit_add_label(lbl);
        let dup = InstructionBuilder::from(CasmInstr::StoreAddFpFp {
            src0_off: M31::from(5),
            src1_off: M31::from(5),
            dst_off: M31::from(5),
        });
        b.emit_push(dup);
        DeduplicateOperandsPass.run(&mut b).unwrap();
        // Label should still point at the first instruction replacing the expanding op.
        let addr = b.labels()[0].address.expect("has addr");
        // After pass: [StoreImm], [copy], [copy], [final]
        // Label was on index 1 originally, should still be 1 now.
        assert_eq!(addr, 1);
        match b.instructions()[addr].inner_instr() {
            CasmInstr::StoreAddFpImm { .. } => (),
            _ => panic!("label not at first replacement"),
        }
    }
}
