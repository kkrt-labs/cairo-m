// Adapted from https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/debug_tools/relation_tracker.rs

use itertools::chain;
use num_traits::{One, Zero};
use stwo_constraint_framework::relation_tracker::{
    RelationSummary, RelationTrackerEntry, add_to_relation_entries,
};
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::{BackendForChannel, Column};
use stwo_prover::core::channel::MerkleChannel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::pcs::{CommitmentSchemeProver, TreeVec};
use stwo_prover::core::poly::circle::CanonicCoset;

use crate::components::Components;
use crate::public_data::PublicData;

pub fn track_and_summarize_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
    public_data: &PublicData,
) -> RelationSummary
where
    SimdBackend: BackendForChannel<MC>,
{
    let entries = track_relations(commitment_scheme, components, public_data);
    RelationSummary::summarize_relations(&entries).cleaned()
}

fn track_relations<MC: MerkleChannel>(
    commitment_scheme: &CommitmentSchemeProver<'_, SimdBackend, MC>,
    components: &Components,
    public_data: &PublicData,
) -> Vec<RelationTrackerEntry>
where
    SimdBackend: BackendForChannel<MC>,
{
    let evals = commitment_scheme.trace().polys.map(|tree| {
        tree.iter()
            .map(|poly| {
                poly.evaluate(CanonicCoset::new(poly.log_size()).circle_domain())
                    .values
                    .to_cpu()
            })
            .collect()
    });
    let evals = &evals.as_ref();
    let trace = &evals.into();

    let mut entries = relation_entries(components, trace);

    let initial_registers = public_data.initial_registers;
    let final_registers = public_data.final_registers;

    entries.push(RelationTrackerEntry {
        relation: "Registers".to_string(),
        mult: M31::one(),
        values: [initial_registers.pc, initial_registers.fp].to_vec(),
    });
    entries.push(RelationTrackerEntry {
        relation: "Registers".to_string(),
        mult: -M31::one(),
        values: [final_registers.pc, final_registers.fp].to_vec(),
    });
    entries.push(RelationTrackerEntry {
        relation: "Merkle".to_string(),
        mult: -M31::one(),
        values: [
            M31::zero(),
            M31::zero(),
            public_data.initial_root,
            public_data.initial_root,
        ]
        .to_vec(),
    });
    entries.push(RelationTrackerEntry {
        relation: "Merkle".to_string(),
        mult: -M31::one(),
        values: [
            M31::zero(),
            M31::zero(),
            public_data.final_root,
            public_data.final_root,
        ]
        .to_vec(),
    });

    // Add memory relation entries for public addresses
    for (addr, value, clock) in public_data.public_entries.iter().flatten() {
        let value_array = value.to_m31_array();
        entries.push(RelationTrackerEntry {
            relation: "Memory".to_string(),
            mult: -M31::one(),
            values: [
                *addr,
                *clock,
                value_array[0],
                value_array[1],
                value_array[2],
                value_array[3],
            ]
            .to_vec(),
        });
    }

    entries
}

fn relation_entries(
    components: &Components,
    trace: &TreeVec<Vec<&Vec<M31>>>,
) -> Vec<RelationTrackerEntry> {
    let Components {
        memory,
        merkle,
        range_check_20,
        opcodes,
    } = components;

    let entries: Vec<RelationTrackerEntry> = chain!(
        add_to_relation_entries(&opcodes.call_abs_imm, trace),
        add_to_relation_entries(&opcodes.jmp_abs_imm, trace),
        add_to_relation_entries(&opcodes.jmp_rel_imm, trace),
        add_to_relation_entries(&opcodes.jnz_fp_imm, trace),
        add_to_relation_entries(&opcodes.ret, trace),
        add_to_relation_entries(&opcodes.store_add_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_add_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_div_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_div_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_double_deref_fp, trace),
        add_to_relation_entries(&opcodes.store_imm, trace),
        add_to_relation_entries(&opcodes.store_mul_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_mul_fp_imm, trace),
        add_to_relation_entries(&opcodes.store_sub_fp_fp, trace),
        add_to_relation_entries(&opcodes.store_sub_fp_imm, trace),
        add_to_relation_entries(memory, trace),
        add_to_relation_entries(merkle, trace),
        add_to_relation_entries(range_check_20, trace),
    )
    .collect();

    entries
}
