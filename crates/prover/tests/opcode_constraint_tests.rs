//! Unit tests for individual opcode components constraints.
// There are no tests to check double memory accesses leading to clock-prev_clock-1 == -1.

use std::ops::Deref;

use cairo_m_common::Opcode;
use cairo_m_prover::adapter::memory::{DataAccess, ExecutionBundle, InstructionAccess};
use cairo_m_prover::components::Relations;
use cairo_m_prover::debug_tools::assert_constraints::MockCommitmentScheme;
use cairo_m_prover::preprocessed::PreProcessedTraceBuilder;
use stwo_prover::constraint_framework::{
    assert_constraints_on_trace, FrameworkComponent, FrameworkEval, TraceLocationAllocator,
    PREPROCESSED_TRACE_IDX,
};
use stwo_prover::core::channel::Blake2sChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

/// Helper macro to reduce boilerplate for testing opcode constraints.
/// Now includes clock validation to catch range check failures early.
macro_rules! test_opcode_constraints {
    ($execution_bundles:expr, $opcode_module:path) => {{
        use $opcode_module as opcode;

        let mut execution_bundles = $execution_bundles;

        let mut commitment_scheme = MockCommitmentScheme::default();

        // Preprocessed trace
        let preprocessed_trace = PreProcessedTraceBuilder::default().build();
        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(preprocessed_trace.gen_trace());
        tree_builder.finalize_interaction();

        // Write trace for the opcode
        let (claim, trace, interaction_claim_data) =
            opcode::Claim::write_trace::<Blake2sMerkleChannel>(&mut execution_bundles);

        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(trace.to_evals());
        tree_builder.finalize_interaction();

        // Interaction trace
        let mut dummy_channel = Blake2sChannel::default();
        let relations = Relations::draw(&mut dummy_channel);

        let (interaction_claim, interaction_trace) =
            opcode::InteractionClaim::write_interaction_trace(
                &relations.registers,
                &relations.memory,
                &relations.range_check_20,
                &interaction_claim_data,
            );

        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(interaction_trace);
        tree_builder.finalize_interaction();

        // Create component
        let mut tree_span_provider =
            TraceLocationAllocator::new_with_preproccessed_columns(&preprocessed_trace.ids());

        let eval = opcode::Eval {
            claim: claim.clone(),
            memory: relations.memory.clone(),
            registers: relations.registers.clone(),
            range_check_20: relations.range_check_20.clone(),
        };

        let component =
            FrameworkComponent::new(&mut tree_span_provider, eval, interaction_claim.claimed_sum);

        // Extract relevant trace columns
        let trace = commitment_scheme.trace_domain_evaluations();
        let mut component_trace = trace
            .sub_tree(component.trace_locations())
            .map(|tree| tree.into_iter().cloned().collect::<Vec<_>>());

        component_trace[PREPROCESSED_TRACE_IDX] = component
            .preproccessed_column_indices()
            .iter()
            .map(|idx| trace[PREPROCESSED_TRACE_IDX][*idx])
            .collect();

        let log_size = component.log_size();
        let component_eval = component.deref();

        // Assert constraints
        assert_constraints_on_trace(
            &component_trace,
            log_size,
            |eval| {
                component_eval.evaluate(eval);
            },
            component.claimed_sum(),
        );
    }};
}

// ============================================================================
// Arithmetic Operations Tests
// ============================================================================

#[test]
fn test_store_add_fp_fp_constraints() {
    // Test case 1: Simple addition [fp + 4] = [fp + 1] + [fp + 2]
    let execution_bundles = vec![
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    1, // off0
                    2, // off1
                    4, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(104), // fp + 4
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(30), // 10 + 20
                }),
            ],
        },
        // Test case 2: Addition with negative offsets
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpFp as u32,
                    M31::from(-2).0, // off0
                    M31::from(-1).0, // off1
                    0,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(198), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5),
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(200), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(12), // 5 + 7
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_fp
    );
}

#[test]
fn test_store_add_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Add positive immediate
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpImm as u32,
                    1,  // off0 (source)
                    42, // off1 (immediate value)
                    3,  // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                Some(DataAccess {
                    address: M31::from(103), // fp + 3
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(52), // 10 + 42
                }),
                None,
            ],
        },
        // Test case 2: Add negative immediate (M31 field arithmetic)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(100),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreAddFpImm as u32,
                    0,               // off0
                    M31::from(-5).0, // off1 (immediate -5)
                    1,               // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(101), // fp + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(15), // 20 + (-5) = 15
                }),
                None,
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_imm
    );
}

#[test]
fn test_store_sub_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case: [fp + 2] = [fp + 0] - [fp + 1]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreSubFpFp as u32,
                    0, // off0
                    1, // off1
                    2, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50),
                    value: M31::from(50),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(20),
                    value: M31::from(20),
                }),
                Some(DataAccess {
                    address: M31::from(102),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(30), // 50 - 20
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_sub_fp_fp
    );
}

#[test]
fn test_store_sub_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreSubFpImm as u32,
                0,  // off0
                15, // off1 (immediate)
                1,  // off2
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(40),
                value: M31::from(40),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(25), // 40 - 15
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_sub_fp_imm
    );
}

#[test]
fn test_store_mul_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: Simple multiplication
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreMulFpFp as u32,
                    0, // off0
                    1, // off1
                    2, // off2
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7),
                    value: M31::from(7),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(8),
                    value: M31::from(8),
                }),
                Some(DataAccess {
                    address: M31::from(102),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(56), // 7 * 8
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_fp
    );
}

#[test]
fn test_store_mul_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreMulFpImm as u32,
                0,  // off0
                11, // off1 (immediate multiplier)
                1,  // off2
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(5),
                value: M31::from(5),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(55), // 5 * 11
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_mul_fp_imm
    );
}

#[test]
fn test_store_div_fp_fp_constraints() {
    // Division in M31 is multiplication by multiplicative inverse
    let divisor = M31::from(7);
    let dividend = M31::from(35);
    let quotient = dividend * divisor.inverse();

    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreDivFpFp as u32,
                0, // off0 (dividend)
                1, // off1 (divisor)
                2, // off2 (quotient)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: dividend,
                value: dividend,
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: divisor,
                value: divisor,
            }),
            Some(DataAccess {
                address: M31::from(102),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: quotient, // 35 / 7 = 5 in M31
            }),
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_fp
    );
}

#[test]
fn test_store_div_fp_imm_constraints() {
    let dividend = M31::from(42);
    let divisor = M31::from(6);
    let quotient = dividend * divisor.inverse();

    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreDivFpImm as u32,
                0,         // off0
                divisor.0, // off1 (immediate divisor)
                1,         // off2
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: dividend,
                value: dividend,
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: quotient, // 42 / 6 = 7 in M31
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_div_fp_imm
    );
}

// ============================================================================
// Memory Operations Tests
// ============================================================================

#[test]
fn test_store_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Store positive immediate
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreImm as u32,
                    42, // off0 (immediate value)
                    0,  // off1 (unused)
                    2,  // off2 (offset from fp)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(42),
                }),
                None,
                None,
            ],
        },
        // Test case 2: Store to negative offset
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(1),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreImm as u32,
                    100,             // off0 (immediate value)
                    0,               // off1 (unused)
                    M31::from(-1).0, // off2 (negative offset)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(100),
                }),
                None,
                None,
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_imm
    );
}

#[test]
fn test_store_deref_fp_constraints() {
    let execution_bundles = vec![
        // Test: [fp + 2] = [fp + 0]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDerefFp as u32,
                    0, // off0 (source)
                    0, // off1 (unused)
                    2, // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(77),
                    value: M31::from(77),
                }),
                Some(DataAccess {
                    address: M31::from(102), // fp + 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(77), // Copy the value
                }),
                None,
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_deref_fp
    );
}

#[test]
fn test_store_double_deref_fp_constraints() {
    let execution_bundles = vec![
        // Test: [fp + 3] = [[fp + 0] + 1]
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(0),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::StoreDoubleDerefFp as u32,
                    0, // off0 (first deref)
                    1, // off1 (offset for second deref)
                    3, // off2 (destination)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100), // fp + 0
                    prev_clock: M31::from(0),
                    prev_value: M31::from(200), // Points to address 200
                    value: M31::from(200),
                }),
                Some(DataAccess {
                    address: M31::from(201), // [fp + 0] + 1 = 200 + 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(88),
                    value: M31::from(88),
                }),
                Some(DataAccess {
                    address: M31::from(103), // fp + 3
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0),
                    value: M31::from(88), // Value from double deref
                }),
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_double_deref_fp
    );
}

// ============================================================================
// Jump Operations Tests
// ============================================================================

#[test]
fn test_jmp_abs_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Jump to address 50
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpAbsImm as u32,
                    50, // off0 (jump target)
                    0,  // off1 (unused)
                    0,  // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
        // Test case 2: Jump to address 0 (start of program)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(100),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpAbsImm as u32,
                    0, // off0 (jump to start)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_imm
    );
}

#[test]
fn test_jmp_abs_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsDerefFp as u32,
                1, // off0 (offset to deref)
                0, // off1 (unused)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(101), // fp + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(75), // Jump target
                value: M31::from(75),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_deref_fp
    );
}

#[test]
fn test_jmp_abs_double_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsDoubleDerefFp as u32,
                0, // off0
                2, // off1 (offset for second deref)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(200), // First pointer
                value: M31::from(200),
            }),
            Some(DataAccess {
                address: M31::from(202), // 200 + 2
                prev_clock: M31::from(0),
                prev_value: M31::from(80), // Jump target
                value: M31::from(80),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_double_deref_fp
    );
}

#[test]
fn test_jmp_abs_add_fp_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsAddFpFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(30),
                value: M31::from(30),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(20),
                value: M31::from(20),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_add_fp_fp
    );
}

#[test]
fn test_jmp_abs_add_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsAddFpImm as u32,
                0,  // off0
                25, // off1 (immediate to add)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(30),
                value: M31::from(30),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_add_fp_imm
    );
}

#[test]
fn test_jmp_abs_mul_fp_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsMulFpFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(10),
                value: M31::from(10),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(8),
                value: M31::from(8),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_mul_fp_fp
    );
}

#[test]
fn test_jmp_abs_mul_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpAbsMulFpImm as u32,
                0, // off0
                4, // off1 (immediate multiplier)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(20),
                value: M31::from(20),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_abs_mul_fp_imm
    );
}

#[test]
fn test_jmp_rel_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Jump forward
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpRelImm as u32,
                    5, // off0 (relative jump +5)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
        // Test case 2: Jump backward
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(50),
                fp: M31::from(100),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JmpRelImm as u32,
                    M31::from(-10).0, // off0 (relative jump -10)
                    0,                // off1 (unused)
                    0,                // off2 (unused)
                ),
            },
            operands: [None, None, None],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_imm
    );
}

#[test]
fn test_jmp_rel_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelDerefFp as u32,
                1, // off0
                0, // off1 (unused)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(10), // Relative offset
                value: M31::from(10),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_deref_fp
    );
}

#[test]
fn test_jmp_rel_double_deref_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelDoubleDerefFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(200),
                value: M31::from(200),
            }),
            Some(DataAccess {
                address: M31::from(201),
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Relative offset
                value: M31::from(15),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_double_deref_fp
    );
}

#[test]
fn test_jmp_rel_add_fp_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelAddFpFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(5),
                value: M31::from(5),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(3),
                value: M31::from(3),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_add_fp_fp
    );
}

#[test]
fn test_jmp_rel_add_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelAddFpImm as u32,
                0, // off0
                7, // off1 (immediate)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(3),
                value: M31::from(3),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_add_fp_imm
    );
}

#[test]
fn test_jmp_rel_mul_fp_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelMulFpFp as u32,
                0, // off0
                1, // off1
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(3),
                value: M31::from(3),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(4),
                value: M31::from(4),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_mul_fp_fp
    );
}

#[test]
fn test_jmp_rel_mul_fp_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JmpRelMulFpImm as u32,
                0, // off0
                2, // off1 (immediate multiplier)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(6),
                value: M31::from(6),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jmp_rel_mul_fp_imm
    );
}

// ============================================================================
// Conditional Jump Tests
// ============================================================================

#[test]
fn test_jnz_fp_fp_constraints() {
    let execution_bundles = vec![
        // Test case 1: Condition is non-zero (will jump)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpFp as u32,
                    0, // off0 (condition)
                    1, // off1 (jump offset)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(5), // Non-zero
                    value: M31::from(5),
                }),
                Some(DataAccess {
                    address: M31::from(101),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(7), // Jump offset
                    value: M31::from(7),
                }),
                None,
            ],
        },
        // Test case 2: Condition is zero (won't jump)
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(20),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpFp as u32,
                    0, // off0 (condition)
                    1, // off1 (jump offset)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0), // Zero condition
                    value: M31::from(0),
                }),
                Some(DataAccess {
                    address: M31::from(201),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(10),
                    value: M31::from(10),
                }),
                None,
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_fp
    );
}

#[test]
fn test_jnz_fp_fp_taken_constraints() {
    // This tests the taken branch component
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JnzFpFp as u32,
                0, // off0 (condition)
                1, // off1 (jump offset)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(10), // Non-zero - will jump
                value: M31::from(10),
            }),
            Some(DataAccess {
                address: M31::from(101),
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Jump offset
                value: M31::from(15),
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_fp_taken
    );
}

#[test]
fn test_jnz_fp_imm_constraints() {
    let execution_bundles = vec![
        // Test case 1: Non-zero condition
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(10),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpImm as u32,
                    0,  // off0 (condition)
                    20, // off1 (immediate jump offset)
                    0,  // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(100),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(1), // Non-zero
                    value: M31::from(1),
                }),
                None,
                None,
            ],
        },
        // Test case 2: Zero condition
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(30),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::JnzFpImm as u32,
                    0,  // off0 (condition)
                    15, // off1 (immediate jump offset)
                    0,  // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(200),
                    prev_clock: M31::from(0),
                    prev_value: M31::from(0), // Zero
                    value: M31::from(0),
                }),
                None,
                None,
            ],
        },
    ];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_imm
    );
}

#[test]
fn test_jnz_fp_imm_taken_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::JnzFpImm as u32,
                0,  // off0 (condition)
                25, // off1 (immediate jump offset)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100),
                prev_clock: M31::from(0),
                prev_value: M31::from(42), // Non-zero - will jump
                value: M31::from(42),
            }),
            None,
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::jnz_fp_imm_taken
    );
}

// ============================================================================
// Call and Return Operations Tests
// ============================================================================

#[test]
fn test_call_abs_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(10),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallAbsImm as u32,
                50, // off0 (call target)
                2,  // off1 (new fp offset)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(102), // fp + 2 (new fp location)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(103), // fp + 2 + 1 (return address)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(11), // PC + 1
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_abs_imm
    );
}

#[test]
fn test_call_abs_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(20),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallAbsFp as u32,
                1, // off0 (offset to call target)
                3, // off1 (new fp offset)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(101), // fp + 1 (call target)
                prev_clock: M31::from(0),
                prev_value: M31::from(80),
                value: M31::from(80),
            }),
            Some(DataAccess {
                address: M31::from(103), // fp + 3 (new fp location)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(104), // fp + 3 + 1 (return address)
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(21), // PC + 1
            }),
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_abs_fp
    );
}

#[test]
fn test_call_rel_imm_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(30),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallRelImm as u32,
                10, // off0 (relative call offset)
                2,  // off1 (new fp offset)
                0,  // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(102), // fp + 2
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(103), // fp + 2 + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(31), // PC + 1
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_rel_imm
    );
}

#[test]
fn test_call_rel_fp_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(40),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::CallRelFp as u32,
                0, // off0 (offset to relative call value)
                4, // off1 (new fp offset)
                0, // off2 (unused)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(15), // Relative offset
                value: M31::from(15),
            }),
            Some(DataAccess {
                address: M31::from(104), // fp + 4
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(100), // Store old fp
            }),
            Some(DataAccess {
                address: M31::from(105), // fp + 4 + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(0),
                value: M31::from(41), // PC + 1
            }),
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::call_rel_fp
    );
}

#[test]
fn test_ret_constraints() {
    let execution_bundles = vec![
        // Test case 1: Return from function
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(25),
                fp: M31::from(100),
            },
            clock: M31::from(1),
            instruction: InstructionAccess {
                prev_clock: M31::from(0),
                value: QM31::from_u32_unchecked(
                    Opcode::Ret as u32,
                    0, // off0 (unused)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(98), // fp - 2 (old fp location)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(50), // Previous fp value
                    value: M31::from(50),
                }),
                Some(DataAccess {
                    address: M31::from(99), // fp - 1 (return address)
                    prev_clock: M31::from(0),
                    prev_value: M31::from(15), // Return address
                    value: M31::from(15),
                }),
                None,
            ],
        },
        // Test case 2: Return from nested function
        ExecutionBundle {
            registers: cairo_m_common::State {
                pc: M31::from(80),
                fp: M31::from(200),
            },
            clock: M31::from(2),
            instruction: InstructionAccess {
                prev_clock: M31::from(1),
                value: QM31::from_u32_unchecked(
                    Opcode::Ret as u32,
                    0, // off0 (unused)
                    0, // off1 (unused)
                    0, // off2 (unused)
                ),
            },
            operands: [
                Some(DataAccess {
                    address: M31::from(198), // fp - 2
                    prev_clock: M31::from(0),
                    prev_value: M31::from(150), // Previous fp
                    value: M31::from(150),
                }),
                Some(DataAccess {
                    address: M31::from(199), // fp - 1
                    prev_clock: M31::from(0),
                    prev_value: M31::from(60), // Return address
                    value: M31::from(60),
                }),
                None,
            ],
        },
    ];

    test_opcode_constraints!(execution_bundles, cairo_m_prover::components::opcodes::ret);
}

// ============================================================================
// In-place Store Operations Tests (special variants)
// ============================================================================

#[test]
fn test_store_add_fp_fp_inplace_constraints() {
    // In-place operations modify the destination directly
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreAddFpFp as u32, // Note: opcode is same, but component handles in-place
                0,                           // off0
                1,                           // off1
                0,                           // off2 (same as off0 for in-place)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(10),
                value: M31::from(10),
            }),
            Some(DataAccess {
                address: M31::from(101), // fp + 1
                prev_clock: M31::from(0),
                prev_value: M31::from(5),
                value: M31::from(5),
            }),
            Some(DataAccess {
                address: M31::from(100), // fp + 0 (in-place update)
                prev_clock: M31::from(1),
                prev_value: M31::from(10),
                value: M31::from(15), // 10 + 5
            }),
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_fp_inplace
    );
}

#[test]
fn test_store_add_fp_imm_inplace_constraints() {
    let execution_bundles = vec![ExecutionBundle {
        registers: cairo_m_common::State {
            pc: M31::from(0),
            fp: M31::from(100),
        },
        clock: M31::from(1),
        instruction: InstructionAccess {
            prev_clock: M31::from(0),
            value: QM31::from_u32_unchecked(
                Opcode::StoreAddFpImm as u32,
                0,  // off0
                20, // off1 (immediate)
                0,  // off2 (same as off0 for in-place)
            ),
        },
        operands: [
            Some(DataAccess {
                address: M31::from(100), // fp + 0
                prev_clock: M31::from(0),
                prev_value: M31::from(30),
                value: M31::from(30),
            }),
            Some(DataAccess {
                address: M31::from(100), // fp + 0 (in-place update)
                prev_clock: M31::from(1),
                prev_value: M31::from(30),
                value: M31::from(50), // 30 + 20
            }),
            None,
        ],
    }];

    test_opcode_constraints!(
        execution_bundles,
        cairo_m_prover::components::opcodes::store_add_fp_imm_inplace
    );
}
