//! Macros for defining opcode components with reduced boilerplate.

/// Defines an opcode component with all the necessary boilerplate code.
///
/// This macro generates:
/// - Constants for trace columns and lookups
/// - LookupData and InteractionClaimData structs
/// - Claim struct with all required implementations
/// - InteractionClaim struct with trace writing logic
/// - Eval struct with constraint evaluation
///
/// # Arguments
///
/// * `name` - The name of the opcode component (e.g., store_add_fp_fp)
/// * `opcode_id` - The opcode variant from the Opcode enum
/// * `columns` - List of trace column names in order
/// * `lookups` - Map of lookup relation names to their count
/// * `write_trace` - Closure that computes trace values and fills lookup data
/// * `evaluate` - Closure that defines constraints
#[macro_export]
macro_rules! define_opcode_component {
    {
        name: $name:ident,
        opcode_id: $opcode_id:ident,
        columns: [$($col:ident),* $(,)?],
        lookups: {
            registers: $registers_lookup_count:expr,
            memory: $memory_lookup_count:expr,
            range_check_20: $range_check_20_lookup_count:expr $(,)?
        },
        write_trace: |$input:ident, $lookup_data:ident, $enabler:ident, $one:ident, $zero:ident| $write_trace:block,
        evaluate: |$eval_ident:ident, $cols_ident:ident, $one_ident:ident, $self_ident:ident| $evaluate_block:block
    } => {
        use cairo_m_common::Opcode;
        use num_traits::{One, Zero};
        use rayon::iter::{
            IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
        };
        use serde::{Deserialize, Serialize};
        use stwo_air_utils::trace::component_trace::ComponentTrace;
        use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
        use stwo_prover::constraint_framework::logup::LogupTraceGenerator;
        use stwo_prover::constraint_framework::{
            EvalAtRow, FrameworkComponent, FrameworkEval, Relation, RelationEntry,
        };
        use stwo_prover::core::backend::simd::conversion::Pack;
        use stwo_prover::core::backend::simd::m31::{PackedM31, LOG_N_LANES, N_LANES};
        use stwo_prover::core::backend::simd::qm31::PackedQM31;
        use stwo_prover::core::backend::simd::SimdBackend;
        use stwo_prover::core::backend::BackendForChannel;
        use stwo_prover::core::channel::{Channel, MerkleChannel};
        use stwo_prover::core::fields::m31::{BaseField, M31};
        use stwo_prover::core::fields::qm31::SecureField;
        use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
        use stwo_prover::core::pcs::TreeVec;
        use stwo_prover::core::poly::circle::CircleEvaluation;
        use stwo_prover::core::poly::BitReversedOrder;

        use $crate::adapter::StateData;
        use $crate::relations;
        use $crate::utils::{Enabler, PackedStateData};

        const N_TRACE_COLUMNS: usize = 0 $(+ { let _ = stringify!($col); 1 })*;

        const N_REGISTERS_LOOKUPS: usize = $registers_lookup_count;
        const N_MEMORY_LOOKUPS: usize = $memory_lookup_count;
        const N_RANGE_CHECK_20_LOOKUPS: usize = $range_check_20_lookup_count;

        const N_LOOKUPS_COLUMNS: usize = SECURE_EXTENSION_DEGREE
            * (
                N_REGISTERS_LOOKUPS + N_MEMORY_LOOKUPS + N_RANGE_CHECK_20_LOOKUPS
            ).div_ceil(2);

        pub struct InteractionClaimData {
            pub lookup_data: LookupData,
            pub non_padded_length: usize,
        }

        #[derive(Uninitialized, IterMut, ParIterMut)]
        pub struct LookupData {
            pub registers: [Vec<[PackedM31; 2]>; N_REGISTERS_LOOKUPS],
            pub memory: [Vec<[PackedM31; 6]>; N_MEMORY_LOOKUPS],
            pub range_check_20: [Vec<PackedM31>; N_RANGE_CHECK_20_LOOKUPS],
        }

        #[derive(Clone, Default, Serialize, Deserialize)]
        pub struct Claim {
            pub log_size: u32,
        }

        impl Claim {
            pub fn mix_into(&self, channel: &mut impl Channel) {
                channel.mix_u64(self.log_size as u64);
            }

            pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
                let trace = vec![self.log_size; N_TRACE_COLUMNS];
                let interaction_trace = vec![self.log_size; N_LOOKUPS_COLUMNS];
                TreeVec::new(vec![vec![], trace, interaction_trace])
            }

            pub fn write_trace<MC: MerkleChannel>(
                inputs: &mut Vec<StateData>,
            ) -> (Self, ComponentTrace<N_TRACE_COLUMNS>, InteractionClaimData)
            where
                SimdBackend: BackendForChannel<MC>,
            {
                let non_padded_length = inputs.len();
                let log_size = std::cmp::max(LOG_N_LANES, inputs.len().next_power_of_two().ilog2());

                let (mut trace, mut lookup_data) = unsafe {
                    (
                        ComponentTrace::<N_TRACE_COLUMNS>::uninitialized(log_size),
                        LookupData::uninitialized(log_size - LOG_N_LANES),
                    )
                };
                inputs.resize(1 << log_size, StateData::default());
                let packed_inputs: Vec<PackedStateData> = inputs
                    .chunks(N_LANES)
                    .map(|chunk| {
                        let array: [StateData; N_LANES] = chunk.try_into().unwrap();
                        Pack::pack(array)
                    })
                    .collect();

                let $zero = PackedM31::from(M31::zero());
                let $one = PackedM31::from(M31::one());
                let enabler_col = Enabler::new(non_padded_length);
                (
                    trace.par_iter_mut(),
                    packed_inputs.par_iter(),
                    lookup_data.par_iter_mut(),
                )
                    .into_par_iter()
                    .enumerate()
                    .for_each(|(row_index, (mut row, $input, $lookup_data))| {
                        let $enabler = enabler_col.packed_at(row_index);

                        // User-provided trace writing logic
                        let ($($col),*) = $write_trace;

                        // Assign columns in order
                        let mut _col_idx = 0;
                        $(
                            *row[_col_idx] = $col;
                            #[allow(unused_assignments)]
                            {_col_idx += 1;}
                        )*
                    });

                (
                    Self { log_size },
                    trace,
                    InteractionClaimData {
                        lookup_data,
                        non_padded_length,
                    },
                )
            }
        }

        #[derive(Clone, Serialize, Deserialize)]
        pub struct InteractionClaim {
            pub claimed_sum: SecureField,
        }

        impl InteractionClaim {
            pub fn mix_into(&self, channel: &mut impl Channel) {
                channel.mix_felts(&[self.claimed_sum]);
            }

            pub fn write_interaction_trace(
                registers_relation: &relations::Registers,
                memory_relation: &relations::Memory,
                range_check_20_relation: &relations::RangeCheck_20,
                interaction_claim_data: &InteractionClaimData,
            ) -> (
                Self,
                impl IntoIterator<Item = CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
            ) {
                let log_size = {
                    if N_REGISTERS_LOOKUPS > 0 {
                        interaction_claim_data.lookup_data.registers[0].len().ilog2() + LOG_N_LANES
                    } else if N_MEMORY_LOOKUPS > 0 {
                        interaction_claim_data.lookup_data.memory[0].len().ilog2() + LOG_N_LANES
                    } else if N_RANGE_CHECK_20_LOOKUPS > 0 {
                        interaction_claim_data.lookup_data.range_check_20[0].len().ilog2() + LOG_N_LANES
                    } else {
                        // Default log_size if no lookups, though this is unlikely for opcodes.
                        // It will likely be padded to LOG_N_LANES anyway.
                        0
                    }
                };

                let mut interaction_trace = LogupTraceGenerator::new(log_size);
                let enabler_col = Enabler::new(interaction_claim_data.non_padded_length);

                if N_REGISTERS_LOOKUPS > 0 {
                    $crate::components::opcodes::macros::define_opcode_component!(@generate_interaction_columns
                        registers,
                        registers_relation,
                        interaction_claim_data,
                        enabler_col,
                        interaction_trace
                    );
                }
                if N_MEMORY_LOOKUPS > 0 {
                    $crate::components::opcodes::macros::define_opcode_component!(@generate_interaction_columns
                        memory,
                        memory_relation,
                        interaction_claim_data,
                        enabler_col,
                        interaction_trace
                    );
                }
                if N_RANGE_CHECK_20_LOOKUPS > 0 {
                    $crate::components::opcodes::macros::define_opcode_component!(@generate_interaction_columns
                        range_check_20,
                        range_check_20_relation,
                        interaction_claim_data,
                        enabler_col,
                        interaction_trace
                    );
                }

                let (trace, claimed_sum) = interaction_trace.finalize_last();
                (Self { claimed_sum }, trace)
            }
        }

        pub struct Eval {
            pub claim: Claim,
            pub registers: relations::Registers,
            pub memory: relations::Memory,
            pub range_check_20: relations::RangeCheck_20,
        }

        impl FrameworkEval for Eval {
            fn log_size(&self) -> u32 {
                self.claim.log_size
            }

            fn max_constraint_log_degree_bound(&self) -> u32 {
                self.log_size() + 1
            }

            fn evaluate<E: EvalAtRow>(&self, mut $eval_ident: E) -> E {
                let $one_ident = E::F::from(M31::one());
                let expected_opcode_id = E::F::from(M31::from(Opcode::$opcode_id));

                #[allow(dead_code)]
                struct TraceMasks<F> {
                    $($col: F,)*
                }

                let $cols_ident = TraceMasks {
                    $($col: $eval_ident.next_trace_mask(),)*
                };

                $eval_ident.add_constraint($cols_ident.enabler.clone() * ($one_ident.clone() - $cols_ident.enabler.clone()));
                $eval_ident.add_constraint($cols_ident.enabler.clone() * ($cols_ident.opcode_id.clone() - expected_opcode_id));

                // User-provided constraints with self passed as parameter
                {
                    let $self_ident = self;
                    $evaluate_block
                }

                $eval_ident.finalize_logup_in_pairs();
                $eval_ident
            }
        }

        pub type Component = FrameworkComponent<Eval>;
    };

    // Helper for generating interaction columns
    (@generate_interaction_columns memory, $relation:ident, $data:ident, $enabler_col:ident, $trace:ident) => {
        paste::paste! {
            for i in (0..[<N_MEMORY_LOOKUPS>]).step_by(2) {
                let mut col = $trace.new_col();
                (
                    col.par_iter_mut(),
                    &$data.lookup_data.memory[i],
                    &$data.lookup_data.memory[i + 1],
                )
                    .into_par_iter()
                    .enumerate()
                    .for_each(|(idx, (writer, memory_prev, memory_new))| {
                        let num_prev = -PackedQM31::from($enabler_col.packed_at(idx));
                        let num_new = PackedQM31::from($enabler_col.packed_at(idx));
                        let denom_prev: PackedQM31 = $relation.combine(memory_prev);
                        let denom_new: PackedQM31 = $relation.combine(memory_new);

                        let numerator = num_prev * denom_new + num_new * denom_prev;
                        let denom = denom_prev * denom_new;

                        writer.write_frac(numerator, denom);
                    });
                col.finalize_col();
            }
        }
    };

    (@generate_interaction_columns registers, $relation:ident, $data:ident, $enabler_col:ident, $trace:ident) => {
        paste::paste! {
            if [<N_REGISTERS_LOOKUPS>] > 0 {
                let mut col = $trace.new_col();
                (
                    col.par_iter_mut(),
                    &$data.lookup_data.registers[0],
                    &$data.lookup_data.registers[1],
                )
                    .into_par_iter()
                    .enumerate()
                    .for_each(|(i, (writer, registers_prev, registers_new))| {
                        let num_prev = -PackedQM31::from($enabler_col.packed_at(i));
                        let num_new = PackedQM31::from($enabler_col.packed_at(i));
                        let denom_prev: PackedQM31 = $relation.combine(registers_prev);
                        let denom_new: PackedQM31 = $relation.combine(registers_new);

                        let numerator = num_prev * denom_new + num_new * denom_prev;
                        let denom = denom_prev * denom_new;

                        writer.write_frac(numerator, denom);
                    });
                col.finalize_col();
            }
        }
    };

    (@generate_interaction_columns range_check_20, $relation:ident, $data:ident, $enabler_col:ident, $trace:ident) => {
        paste::paste! {
            for i in (0..[<N_RANGE_CHECK_20_LOOKUPS>]).step_by(2) {
                let mut col = $trace.new_col();
                if i + 1 < [<N_RANGE_CHECK_20_LOOKUPS>] {
                    (
                        col.par_iter_mut(),
                        &$data.lookup_data.range_check_20[i],
                        &$data.lookup_data.range_check_20[i + 1],
                    )
                        .into_par_iter()
                        .enumerate()
                        .for_each(|(idx, (writer, range_check_0, range_check_1))| {
                            let num = -PackedQM31::from($enabler_col.packed_at(idx));
                            let denom_0: PackedQM31 = $relation.combine(&[*range_check_0]);
                            let denom_1: PackedQM31 = $relation.combine(&[*range_check_1]);

                            let numerator = num * denom_1 + num * denom_0;
                            let denom = denom_0 * denom_1;
                            writer.write_frac(numerator, denom);
                        });
                } else {
                    // Handle odd number of lookups
                    (
                        col.par_iter_mut(),
                        &$data.lookup_data.range_check_20[i],
                    )
                        .into_par_iter()
                        .enumerate()
                        .for_each(|(idx, (writer, range_check))| {
                            let num = -PackedQM31::from($enabler_col.packed_at(idx));
                            let denom: PackedQM31 = $relation.combine(&[*range_check]);
                            writer.write_frac(num, denom);
                        });
                }
                col.finalize_col();
            }
        }
    };
}

pub use define_opcode_component;
