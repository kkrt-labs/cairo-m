macro_rules! define_opcodes {
    ($(($opcode_enum:expr, $opcode:ident)),* $(,)?) => {
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
                    let state_data = instructions.states_by_opcodes.entry($opcode_enum).or_default();
                    let (paste::paste! { [<$opcode _claim>] }, paste::paste! { [<$opcode _trace_raw>] }, paste::paste! { [<$opcode _interaction_claim_data>] }) =
                        $opcode::Claim::write_trace(state_data);
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
                define_opcodes!(@range_check_20 self, $($opcode),*)
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

    // Helper rule for range_check_20 chaining
    (@range_check_20 $self:ident, $first:ident $(, $rest:ident)*) => {
        $self.$first
            .lookup_data
            .range_check_20
            .par_iter()
            .flatten()
            $(.chain(
                $self.$rest
                    .lookup_data
                    .range_check_20
                    .par_iter()
                    .flatten(),
            ))*
    };
}

use cairo_m_common::Opcode;
use num_traits::Zero;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::air::{Component as ComponentVerifier, ComponentProver};
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::backend::simd::SimdBackend;
pub use stwo_prover::core::backend::simd::m31::PackedM31;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::poly::BitReversedOrder;
use stwo_prover::core::poly::circle::CircleEvaluation;

use crate::adapter::Instructions;
use crate::components::Relations;

// Define all opcode structures and implementations with a single macro call
define_opcodes! {
    (Opcode::CallAbsImm, call_abs_imm),
    (Opcode::JmpAbsImm, jmp_abs_imm),
    (Opcode::JmpRelImm, jmp_rel_imm),
    (Opcode::JnzFpImm, jnz_fp_imm),
    (Opcode::JnzFpImm, jnz_fp_imm_taken),
    (Opcode::Ret, ret),
    (Opcode::StoreAddFpFp, store_add_fp_fp),
    (Opcode::StoreAddFpFp, store_add_fp_fp_inplace),
    (Opcode::StoreAddFpImm, store_add_fp_imm),
    (Opcode::StoreAddFpImm, store_add_fp_imm_inplace),
    (Opcode::StoreDerefFp, store_deref_fp),
    (Opcode::StoreDivFpFp, store_div_fp_fp),
    (Opcode::StoreDivFpImm, store_div_fp_imm),
    (Opcode::StoreDoubleDerefFp, store_double_deref_fp),
    (Opcode::StoreImm, store_imm),
    (Opcode::StoreMulFpFp, store_mul_fp_fp),
    (Opcode::StoreMulFpImm, store_mul_fp_imm),
    (Opcode::StoreSubFpFp, store_sub_fp_fp),
    (Opcode::StoreSubFpImm, store_sub_fp_imm),
}
