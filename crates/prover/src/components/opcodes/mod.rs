macro_rules! define_opcodes {
    // Single pattern: all opcodes use [opcodes...] syntax
    ($(([$($opcode_variant:ident),+ $(,)?], $opcode:ident)),* $(,)?) => {
        define_opcodes!(@check_all_opcodes_used [$($($opcode_variant),+),*]);

        // Generate pub mod declarations for all opcodes
        $(pub mod $opcode;)*

        // Define all structures
        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct Claim {
            $(pub $opcode: $opcode::Claim,)*
        }

        pub struct InteractionClaimData {
            $(pub $opcode: $opcode::InteractionClaimData,)*
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct InteractionClaim {
            $(pub $opcode: $opcode::InteractionClaim,)*
        }

        pub struct Component {
            $(pub $opcode: $opcode::Component,)*
        }

        // Implement Claim methods
        impl Claim {
            pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
                let trees = vec![
                    $(self.$opcode.log_sizes(),)*
                ];
                TreeVec::concat_cols(trees.into_iter())
            }

            pub fn mix_into(&self, channel: &mut impl Channel) {
                $(self.$opcode.mix_into(channel);)*
            }

            pub fn write_trace<MC: MerkleChannel>(
                instructions: &mut Instructions,
            ) -> (
                Self,
                impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
                InteractionClaimData,
            )
            where
                SimdBackend: BackendForChannel<MC>,
            {
                $(
                    // Collect states for all opcodes in this group
                    let mut grouped_states = Vec::new();
                    paste::paste! {
                        $(
                            let state_data = instructions.states_by_opcodes.entry([<$opcode_variant:snake:upper>]).or_default();
                            grouped_states.extend(state_data.drain(..));
                        )+
                    }

                    let (paste::paste! { [<$opcode _claim>] }, paste::paste! { [<$opcode _trace_raw>] }, paste::paste! { [<$opcode _interaction_claim_data>] }) =
                        $opcode::Claim::write_trace(&mut grouped_states);
                    let paste::paste! { [<$opcode _trace>] } = Box::new(paste::paste! { [<$opcode _trace_raw>] }.to_evals().into_iter());
                )*

                let claim = Self {
                    $($opcode: paste::paste! { [<$opcode _claim>] },)*
                };

                let interaction_claim_data = InteractionClaimData {
                    $($opcode: paste::paste! { [<$opcode _interaction_claim_data>] },)*
                };

                let trace = std::iter::empty()
                    $(.chain(paste::paste! { [<$opcode _trace>] }))*;

                (claim, trace, interaction_claim_data)
            }
        }

        // Implement InteractionClaimData methods
        impl InteractionClaimData {
            pub fn range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
                use $crate::preprocessed::range_check::RangeCheckProvider;
                rayon::iter::empty()
                    $(.chain(self.$opcode.get_range_check_20()))*
            }

            pub fn range_check_16(&self) -> impl ParallelIterator<Item = &PackedM31> {
                use $crate::preprocessed::range_check::RangeCheckProvider;
                rayon::iter::empty()
                    $(.chain(self.$opcode.get_range_check_16()))*
            }

            pub fn range_check_8(&self) -> impl ParallelIterator<Item = &PackedM31> {
                use $crate::preprocessed::range_check::RangeCheckProvider;
                rayon::iter::empty()
                    $(.chain(self.$opcode.get_range_check_8()))*
            }
        }

        // Implement InteractionClaim methods
        impl InteractionClaim {
            pub fn claimed_sum(&self) -> SecureField {
                let mut sum = SecureField::zero();
                $(sum += self.$opcode.claimed_sum;)*
                sum
            }

            pub fn mix_into(&self, channel: &mut impl Channel) {
                $(self.$opcode.mix_into(channel);)*
            }

            pub fn write_interaction_trace(
                relations: &Relations,
                interaction_claim_data: &InteractionClaimData,
            ) -> (
                Self,
                impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
            ) {
                $(
                    let ($opcode, paste::paste! { [<$opcode _interaction_trace>] }) =
                        $opcode::InteractionClaim::write_interaction_trace(
                            &relations,
                            &interaction_claim_data.$opcode,
                        );
                )*

                let interaction_claim = Self {
                    $($opcode,)*
                };

                let interaction_trace = std::iter::empty()
                    $(.chain(paste::paste! { [<$opcode _interaction_trace>] }))*;

                (interaction_claim, interaction_trace)
            }
        }

        // Implement Component methods
        impl Component {
            pub fn new(
                location_allocator: &mut TraceLocationAllocator,
                claim: &Claim,
                interaction_claim: &InteractionClaim,
                relations: &Relations,
            ) -> Self {
                Self {
                    $($opcode: $opcode::Component::new(
                        location_allocator,
                        $opcode::Eval {
                            claim: claim.$opcode.clone(),
                            relations: relations.clone(),
                        },
                        interaction_claim.$opcode.claimed_sum,
                    ),)*
                }
            }

            pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
                vec![
                    $(&self.$opcode,)*
                ]
            }

            pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
                vec![
                    $(&self.$opcode,)*
                ]
            }
        }
    };

    // Helper rule to check that all Opcode variants are used
    (@check_all_opcodes_used [$($opcode_variant:ident),* $(,)?]) => {
        // This will be checked at compile time - if any opcode is missing,
        // the match will be non-exhaustive and compilation will fail
        const _: fn() = || {
            use cairo_m_common::instruction::Instruction;
            let _check_all_opcodes = |opcode: Instruction| {
                match opcode {
                    $(
                        Instruction::$opcode_variant { .. } => {},
                    )*
                    // TODO: Add support for these opcodes
                    Instruction::StoreLowerThanFpImm { .. } => {},
                    Instruction::StoreFpImm { .. } => {},
                    Instruction::StoreDoubleDerefFpFp { .. } => {},
                    Instruction::U32StoreAddFpFp { .. } => {},
                    Instruction::U32StoreSubFpFp { .. } => {},
                    Instruction::U32StoreMulFpFp { .. } => {},
                    Instruction::U32StoreDivFpFp { .. } => {},
                    Instruction::U32StoreAddFpImm { .. } => {},
                    Instruction::U32StoreSubFpImm { .. } => {},
                    Instruction::U32StoreMulFpImm { .. } => {},
                    Instruction::U32StoreDivFpImm { .. } => {},
                    Instruction::U32StoreEqFpFp { .. } => {},
                    Instruction::U32StoreNeqFpFp { .. } => {},
                    Instruction::U32StoreGtFpFp { .. } => {},
                    Instruction::U32StoreGeFpFp { .. } => {},
                    Instruction::U32StoreLtFpFp { .. } => {},
                    Instruction::U32StoreLeFpFp { .. } => {},
                    Instruction::U32StoreEqFpImm { .. } => {},
                    Instruction::U32StoreNeqFpImm { .. } => {},
                    Instruction::U32StoreGtFpImm { .. } => {},
                    Instruction::U32StoreGeFpImm { .. } => {},
                    Instruction::U32StoreLtFpImm { .. } => {},
                    Instruction::U32StoreLeFpImm { .. } => {},
                    Instruction::U32StoreAndFpFp { .. } => {},
                    Instruction::U32StoreOrFpFp { .. } => {},
                    Instruction::U32StoreXorFpFp { .. } => {},
                    Instruction::U32StoreAndFpImm { .. } => {},
                    Instruction::U32StoreOrFpImm { .. } => {},
                    Instruction::U32StoreXorFpImm { .. } => {},
                    Instruction::StoreToDoubleDerefFpImm { .. } => {},
                    Instruction::StoreToDoubleDerefFpFp { .. } => {},
                    // Unsound opcodes
                    Instruction::PrintM31 { .. } => {},
                    Instruction::PrintU32 { .. } => {} ,
                }
            };
        };
    };

}

use cairo_m_common::instruction::*;
use num_traits::Zero;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::air::{Component as ComponentVerifier, ComponentProver};
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

use crate::adapter::Instructions;
use crate::components::Relations;

// Define all opcode structures and implementations with a single macro call
define_opcodes!(
    ([AssertEqFpFp], assert_eq_fp_fp),
    ([AssertEqFpImm], assert_eq_fp_imm),
    ([CallAbsImm], call_abs_imm),
    ([JmpAbsImm, JmpRelImm], jmp_imm),
    ([JnzFpImm], jnz_fp_imm),
    ([Ret], ret),
    ([StoreImm], store_imm),
    (
        [StoreAddFpFp, StoreSubFpFp, StoreMulFpFp, StoreDivFpFp,],
        store_fp_fp
    ),
    (
        [StoreAddFpImm, StoreSubFpImm, StoreMulFpImm, StoreDivFpImm,],
        store_fp_imm
    ),
    ([StoreDoubleDerefFp], store_double_deref_fp),
    ([U32StoreImm], u32_store_imm),
);
