use std::sync::atomic::{AtomicU32, Ordering};

use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use serde::{Deserialize, Serialize};
use stwo_constraint_framework::logup::LogupTraceGenerator;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
};
use stwo_prover::core::backend::simd::column::BaseColumn;
use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
use stwo_prover::core::backend::simd::qm31::PackedQM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::fields::qm31::{SecureField, SECURE_EXTENSION_DEGREE};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

use crate::preprocessed::PreProcessedColumn;

// Constants for bitwise operations
/// Number of bits in each operand for bitwise lookups
pub const BITWISE_OPERAND_BITS: u32 = 8;
/// Total bits for the lookup table (operand1 bits + operand2 bits)
pub const BITWISE_LOOKUP_BITS: u32 = BITWISE_OPERAND_BITS * 2;
/// Mask for extracting the lower operand
pub const BITWISE_OPERAND_MASK: u32 = (1 << BITWISE_OPERAND_BITS) - 1;
/// Number of bitwise operations (AND, OR, XOR)
pub const BITWISE_NUM_OPS: u32 = 3;
/// Total log size for stacked bitwise operations
pub const BITWISE_STACKED_LOG_SIZE: u32 = BITWISE_LOOKUP_BITS + 2; // 16 + 2 = 18 (for 3 ops, need next power of 2)

pub struct InteractionClaimData {
    pub bitwise: Vec<[PackedM31; 5]>, // operation_id, input1, input2, result, multiplicity
}

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; 1];
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    /// Writes the preprocessed bitwise trace for all operations (AND, OR, XOR)
    ///
    /// lookup_data contains all bitwise operations made in other components during main trace generation
    /// Each entry is [operation_id, input1, input2] where operation_id is 0 (AND), 1 (OR), or 2 (XOR)
    ///
    /// write_trace creates columns for:
    /// - All operations stacked: 3 * 2^BITWISE_LOOKUP_BITS entries
    pub fn write_trace<'a, MC: MerkleChannel>(
        lookup_data: impl ParallelIterator<Item = &'a [[PackedM31; 3]]>,
    ) -> (
        Self,
        [CircleEvaluation<SimdBackend, M31, BitReversedOrder>; 1],
        InteractionClaimData,
    )
    where
        SimdBackend: BackendForChannel<MC>,
    {
        let total_size = 1 << BITWISE_STACKED_LOG_SIZE;
        let op_size = 1 << BITWISE_LOOKUP_BITS;

        // Initialize multiplicities for all combinations (3 ops * 65536 combinations each)
        let mults_atomic: Vec<AtomicU32> = (0..total_size).map(|_| AtomicU32::new(0)).collect();

        // Count occurrences of each (operation_id, input1, input2) triple
        lookup_data.for_each(|entries| {
            for entry in entries.iter() {
                // entry[0] contains operation_id values (0=AND, 1=OR, 2=XOR)
                // entry[1] contains packed input1 values
                // entry[2] contains packed input2 values
                for i in 0..N_LANES {
                    let op_id = entry[0].to_array()[i];
                    let input1 = entry[1].to_array()[i];
                    let input2 = entry[2].to_array()[i];
                    // Compute index: op_id * 65536 + (input1 << 8) + input2
                    let index = (op_id.0 as usize) * op_size
                        + ((input1.0 as usize) << BITWISE_OPERAND_BITS)
                        + (input2.0 as usize);
                    if index < total_size {
                        mults_atomic[index].fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        // Convert atomic multiplicities to M31
        let mults: Vec<M31> = mults_atomic
            .into_par_iter()
            .map(|atomic| M31(atomic.into_inner()))
            .collect();

        // Generate all columns
        let mut op_id_col = Vec::with_capacity(total_size);
        let mut input1_col = Vec::with_capacity(total_size);
        let mut input2_col = Vec::with_capacity(total_size);
        let mut result_col = Vec::with_capacity(total_size);

        // Stack three operations: AND (0), OR (1), XOR (2)
        for op_id in 0..3 {
            for i in 0..op_size {
                let input1 = (i >> BITWISE_OPERAND_BITS) as u32;
                let input2 = (i & BITWISE_OPERAND_MASK as usize) as u32;
                let result = match op_id {
                    0 => input1 & input2, // AND
                    1 => input1 | input2, // OR
                    2 => input1 ^ input2, // XOR
                    _ => unreachable!(),
                };

                op_id_col.push(M31(op_id));
                input1_col.push(M31(input1));
                input2_col.push(M31(input2));
                result_col.push(M31(result));
            }
        }

        // Pad with zeros to reach power of 2
        while op_id_col.len() < total_size {
            op_id_col.push(M31(0));
            input1_col.push(M31(0));
            input2_col.push(M31(0));
            result_col.push(M31(0));
        }

        // Pack data for interaction
        let packed_data: Vec<[PackedM31; 5]> = (0..total_size)
            .collect::<Vec<_>>()
            .par_chunks(N_LANES)
            .enumerate()
            .map(|(chunk_idx, _chunk)| {
                let base_idx = chunk_idx * N_LANES;
                [
                    PackedM31::from_array(std::array::from_fn(|i| {
                        op_id_col.get(base_idx + i).copied().unwrap_or(M31(0))
                    })),
                    PackedM31::from_array(std::array::from_fn(|i| {
                        input1_col.get(base_idx + i).copied().unwrap_or(M31(0))
                    })),
                    PackedM31::from_array(std::array::from_fn(|i| {
                        input2_col.get(base_idx + i).copied().unwrap_or(M31(0))
                    })),
                    PackedM31::from_array(std::array::from_fn(|i| {
                        result_col.get(base_idx + i).copied().unwrap_or(M31(0))
                    })),
                    PackedM31::from_array(std::array::from_fn(|i| {
                        mults.get(base_idx + i).copied().unwrap_or(M31(0))
                    })),
                ]
            })
            .collect();

        let domain = CanonicCoset::new(BITWISE_STACKED_LOG_SIZE).circle_domain();

        (
            Self {
                log_size: BITWISE_STACKED_LOG_SIZE,
            },
            [CircleEvaluation::<SimdBackend, M31, BitReversedOrder>::new(
                domain,
                BaseColumn::from_iter(mults),
            )],
            InteractionClaimData {
                bitwise: packed_data,
            },
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }

    pub fn write_interaction_trace(
        bitwise: &crate::relations::Bitwise,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    ) {
        let log_size = interaction_claim_data.bitwise.len().ilog2() + LOG_N_LANES;
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        (col.par_iter_mut(), &interaction_claim_data.bitwise)
            .into_par_iter()
            .for_each(|(writer, value)| {
                let denom: PackedQM31 = bitwise.combine(&[value[0], value[1], value[2], value[3]]);
                writer.write_frac(value[4].into(), denom);
            });
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        let interaction_claim = Self { claimed_sum };
        (interaction_claim, trace)
    }
}

#[derive(Clone)]
pub struct Eval {
    pub claim: Claim,
    pub relation: crate::relations::Bitwise,
    pub claimed_sum: SecureField,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.claim.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size() + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        // Read the 5 trace columns
        let operation_id =
            eval.get_preprocessed_column(BitwiseStacked::new(0, BITWISE_OPERAND_BITS).id());
        let input1 =
            eval.get_preprocessed_column(BitwiseStacked::new(1, BITWISE_OPERAND_BITS).id());
        let input2 =
            eval.get_preprocessed_column(BitwiseStacked::new(2, BITWISE_OPERAND_BITS).id());
        let result =
            eval.get_preprocessed_column(BitwiseStacked::new(3, BITWISE_OPERAND_BITS).id());
        let multiplicity = eval.next_trace_mask();

        // Add lookups to the relation
        eval.add_to_relation(RelationEntry::new(
            &self.relation,
            E::EF::from(multiplicity),
            &[operation_id, input1, input2, result],
        ));

        eval.finalize_logup();
        eval
    }
}

pub type Component = FrameworkComponent<Eval>;

/// Bitwise preprocessed columns container
/// Represents all columns needed for bitwise operations (operation_id, input1, input2, result)
pub struct Bitwise {
    operand_bits: u32,
}

impl Bitwise {
    pub const fn new(operand_bits: u32) -> Self {
        // Could support other sizes, but for now we only use 8-bit
        Self { operand_bits }
    }

    /// Returns the 4 columns needed for bitwise operations
    pub const fn columns(&self) -> [BitwiseStacked; 4] {
        [
            BitwiseStacked::new(0, self.operand_bits),
            BitwiseStacked::new(1, self.operand_bits),
            BitwiseStacked::new(2, self.operand_bits),
            BitwiseStacked::new(3, self.operand_bits),
        ]
    }
}

/// Stacked bitwise preprocessed column
/// Stacks AND, OR, XOR operations into a single column
pub struct BitwiseStacked {
    col_index: usize, // 0: operation_id, 1: input1, 2: input2, 3: result
    operand_bits: u32,
}

impl BitwiseStacked {
    pub const fn new(col_index: usize, operand_bits: u32) -> Self {
        assert!(col_index < 4, "col_index must be in range 0..=3");
        Self {
            col_index,
            operand_bits,
        }
    }
}

impl PreProcessedColumn for BitwiseStacked {
    fn log_size(&self) -> u32 {
        // For dynamic sizing: 2 * operand_bits for the lookup table + 2 bits for 3 operations
        self.operand_bits * 2 + 2
    }

    fn gen_column_simd(&self) -> CircleEvaluation<SimdBackend, BaseField, BitReversedOrder> {
        let lookup_bits = self.operand_bits * 2;
        let total_size = 1 << self.log_size();
        let op_size = 1 << lookup_bits;
        let operand_mask = (1 << self.operand_bits) - 1;

        let mut values = Vec::with_capacity(total_size);

        // Stack three operations: AND (0), OR (1), XOR (2)
        for op_id in 0..3 {
            for i in 0..op_size {
                let input1 = i >> self.operand_bits;
                let input2 = i & operand_mask;

                let value = match self.col_index {
                    0 => M31(op_id),  // operation_id
                    1 => M31(input1), // input1
                    2 => M31(input2), // input2
                    3 => {
                        // result based on operation
                        let result = match op_id {
                            0 => input1 & input2, // AND
                            1 => input1 | input2, // OR
                            2 => input1 ^ input2, // XOR
                            _ => unreachable!(),
                        };
                        M31(result)
                    }
                    _ => unreachable!(),
                };
                values.push(value);
            }
        }

        // Pad with zeros to reach power of 2
        while values.len() < total_size {
            values.push(M31(0));
        }

        CircleEvaluation::new(
            CanonicCoset::new(self.log_size()).circle_domain(),
            BaseColumn::from_iter(values),
        )
    }

    fn id(&self) -> PreProcessedColumnId {
        PreProcessedColumnId {
            id: format!("bitwise_stacked_col_{}", self.col_index),
        }
    }
}
