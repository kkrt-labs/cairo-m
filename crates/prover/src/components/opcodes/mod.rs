pub mod call_abs_fp;
pub mod call_abs_imm;
pub mod call_rel_fp;
pub mod call_rel_imm;
pub mod jmp_abs_add_fp_fp;
pub mod jmp_abs_add_fp_imm;
pub mod jmp_abs_deref_fp;
pub mod jmp_abs_double_deref_fp;
pub mod jmp_abs_imm;
pub mod jmp_abs_mul_fp_fp;
pub mod jmp_abs_mul_fp_imm;
pub mod jmp_rel_add_fp_fp;
pub mod jmp_rel_add_fp_imm;
pub mod jmp_rel_deref_fp;
pub mod jmp_rel_double_deref_fp;
pub mod jmp_rel_imm;
pub mod jmp_rel_mul_fp_fp;
pub mod jmp_rel_mul_fp_imm;
pub mod jnz_fp_fp;
pub mod jnz_fp_imm;
pub mod ret;
pub mod store_add_fp_fp;
pub mod store_add_fp_imm;
pub mod store_deref_fp;
pub mod store_div_fp_fp;
pub mod store_div_fp_imm;
pub mod store_double_deref_fp;
pub mod store_imm;
pub mod store_mul_fp_fp;
pub mod store_mul_fp_imm;
pub mod store_sub_fp_fp;
pub mod store_sub_fp_imm;

use cairo_m_common::Opcode;
use num_traits::Zero;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
pub use stwo_air_utils::trace::component_trace::ComponentTrace;
pub use stwo_air_utils_derive::{IterMut, ParIterMut, Uninitialized};
use stwo_prover::constraint_framework::TraceLocationAllocator;
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Claim {
    pub call_abs_fp: call_abs_fp::Claim,
    pub call_abs_imm: call_abs_imm::Claim,
    pub call_rel_fp: call_rel_fp::Claim,
    pub call_rel_imm: call_rel_imm::Claim,
    pub jmp_abs_add_fp_fp: jmp_abs_add_fp_fp::Claim,
    pub jmp_abs_add_fp_imm: jmp_abs_add_fp_imm::Claim,
    pub jmp_abs_deref_fp: jmp_abs_deref_fp::Claim,
    pub jmp_abs_double_deref_fp: jmp_abs_double_deref_fp::Claim,
    pub jmp_abs_imm: jmp_abs_imm::Claim,
    pub jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp::Claim,
    pub jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm::Claim,
    pub jmp_rel_add_fp_fp: jmp_rel_add_fp_fp::Claim,
    pub jmp_rel_add_fp_imm: jmp_rel_add_fp_imm::Claim,
    pub jmp_rel_deref_fp: jmp_rel_deref_fp::Claim,
    pub jmp_rel_double_deref_fp: jmp_rel_double_deref_fp::Claim,
    pub jmp_rel_imm: jmp_rel_imm::Claim,
    pub jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp::Claim,
    pub jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm::Claim,
    pub jnz_fp_fp: jnz_fp_fp::Claim,
    pub jnz_fp_imm: jnz_fp_imm::Claim,
    pub ret: ret::Claim,
    pub store_add_fp_fp: store_add_fp_fp::Claim,
    pub store_add_fp_imm: store_add_fp_imm::Claim,
    pub store_deref_fp: store_deref_fp::Claim,
    pub store_div_fp_fp: store_div_fp_fp::Claim,
    pub store_div_fp_imm: store_div_fp_imm::Claim,
    pub store_double_deref_fp: store_double_deref_fp::Claim,
    pub store_imm: store_imm::Claim,
    pub store_mul_fp_fp: store_mul_fp_fp::Claim,
    pub store_mul_fp_imm: store_mul_fp_imm::Claim,
    pub store_sub_fp_fp: store_sub_fp_fp::Claim,
    pub store_sub_fp_imm: store_sub_fp_imm::Claim,
}

pub struct InteractionClaimData {
    pub call_abs_fp: call_abs_fp::InteractionClaimData,
    pub call_abs_imm: call_abs_imm::InteractionClaimData,
    pub call_rel_fp: call_rel_fp::InteractionClaimData,
    pub call_rel_imm: call_rel_imm::InteractionClaimData,
    pub jmp_abs_add_fp_fp: jmp_abs_add_fp_fp::InteractionClaimData,
    pub jmp_abs_add_fp_imm: jmp_abs_add_fp_imm::InteractionClaimData,
    pub jmp_abs_deref_fp: jmp_abs_deref_fp::InteractionClaimData,
    pub jmp_abs_double_deref_fp: jmp_abs_double_deref_fp::InteractionClaimData,
    pub jmp_abs_imm: jmp_abs_imm::InteractionClaimData,
    pub jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp::InteractionClaimData,
    pub jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm::InteractionClaimData,
    pub jmp_rel_add_fp_fp: jmp_rel_add_fp_fp::InteractionClaimData,
    pub jmp_rel_add_fp_imm: jmp_rel_add_fp_imm::InteractionClaimData,
    pub jmp_rel_deref_fp: jmp_rel_deref_fp::InteractionClaimData,
    pub jmp_rel_double_deref_fp: jmp_rel_double_deref_fp::InteractionClaimData,
    pub jmp_rel_imm: jmp_rel_imm::InteractionClaimData,
    pub jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp::InteractionClaimData,
    pub jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm::InteractionClaimData,
    pub jnz_fp_fp: jnz_fp_fp::InteractionClaimData,
    pub jnz_fp_imm: jnz_fp_imm::InteractionClaimData,
    pub ret: ret::InteractionClaimData,
    pub store_add_fp_fp: store_add_fp_fp::InteractionClaimData,
    pub store_add_fp_imm: store_add_fp_imm::InteractionClaimData,
    pub store_deref_fp: store_deref_fp::InteractionClaimData,
    pub store_div_fp_fp: store_div_fp_fp::InteractionClaimData,
    pub store_div_fp_imm: store_div_fp_imm::InteractionClaimData,
    pub store_double_deref_fp: store_double_deref_fp::InteractionClaimData,
    pub store_imm: store_imm::InteractionClaimData,
    pub store_mul_fp_fp: store_mul_fp_fp::InteractionClaimData,
    pub store_mul_fp_imm: store_mul_fp_imm::InteractionClaimData,
    pub store_sub_fp_fp: store_sub_fp_fp::InteractionClaimData,
    pub store_sub_fp_imm: store_sub_fp_imm::InteractionClaimData,
}

impl InteractionClaimData {
    pub fn range_check_20(&self) -> impl ParallelIterator<Item = &PackedM31> {
        self.call_abs_fp
            .lookup_data
            .range_check_20
            .par_iter()
            .flatten()
    }
}

#[derive(Serialize, Deserialize)]
pub struct InteractionClaim {
    pub call_abs_fp: call_abs_fp::InteractionClaim,
    pub call_abs_imm: call_abs_imm::InteractionClaim,
    pub call_rel_fp: call_rel_fp::InteractionClaim,
    pub call_rel_imm: call_rel_imm::InteractionClaim,
    pub jmp_abs_add_fp_fp: jmp_abs_add_fp_fp::InteractionClaim,
    pub jmp_abs_add_fp_imm: jmp_abs_add_fp_imm::InteractionClaim,
    pub jmp_abs_deref_fp: jmp_abs_deref_fp::InteractionClaim,
    pub jmp_abs_double_deref_fp: jmp_abs_double_deref_fp::InteractionClaim,
    pub jmp_abs_imm: jmp_abs_imm::InteractionClaim,
    pub jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp::InteractionClaim,
    pub jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm::InteractionClaim,
    pub jmp_rel_add_fp_fp: jmp_rel_add_fp_fp::InteractionClaim,
    pub jmp_rel_add_fp_imm: jmp_rel_add_fp_imm::InteractionClaim,
    pub jmp_rel_deref_fp: jmp_rel_deref_fp::InteractionClaim,
    pub jmp_rel_double_deref_fp: jmp_rel_double_deref_fp::InteractionClaim,
    pub jmp_rel_imm: jmp_rel_imm::InteractionClaim,
    pub jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp::InteractionClaim,
    pub jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm::InteractionClaim,
    pub jnz_fp_fp: jnz_fp_fp::InteractionClaim,
    pub jnz_fp_imm: jnz_fp_imm::InteractionClaim,
    pub ret: ret::InteractionClaim,
    pub store_add_fp_fp: store_add_fp_fp::InteractionClaim,
    pub store_add_fp_imm: store_add_fp_imm::InteractionClaim,
    pub store_deref_fp: store_deref_fp::InteractionClaim,
    pub store_div_fp_fp: store_div_fp_fp::InteractionClaim,
    pub store_div_fp_imm: store_div_fp_imm::InteractionClaim,
    pub store_double_deref_fp: store_double_deref_fp::InteractionClaim,
    pub store_imm: store_imm::InteractionClaim,
    pub store_mul_fp_fp: store_mul_fp_fp::InteractionClaim,
    pub store_mul_fp_imm: store_mul_fp_imm::InteractionClaim,
    pub store_sub_fp_fp: store_sub_fp_fp::InteractionClaim,
    pub store_sub_fp_imm: store_sub_fp_imm::InteractionClaim,
}

impl Claim {
    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trees = vec![
            self.call_abs_fp.log_sizes(),
            self.call_abs_imm.log_sizes(),
            self.call_rel_fp.log_sizes(),
            self.call_rel_imm.log_sizes(),
            self.jmp_abs_add_fp_fp.log_sizes(),
            self.jmp_abs_add_fp_imm.log_sizes(),
            self.jmp_abs_deref_fp.log_sizes(),
            self.jmp_abs_double_deref_fp.log_sizes(),
            self.jmp_abs_imm.log_sizes(),
            self.jmp_abs_mul_fp_fp.log_sizes(),
            self.jmp_abs_mul_fp_imm.log_sizes(),
            self.jmp_rel_add_fp_fp.log_sizes(),
            self.jmp_rel_add_fp_imm.log_sizes(),
            self.jmp_rel_deref_fp.log_sizes(),
            self.jmp_rel_double_deref_fp.log_sizes(),
            self.jmp_rel_imm.log_sizes(),
            self.jmp_rel_mul_fp_fp.log_sizes(),
            self.jmp_rel_mul_fp_imm.log_sizes(),
            self.jnz_fp_fp.log_sizes(),
            self.jnz_fp_imm.log_sizes(),
            self.ret.log_sizes(),
            self.store_add_fp_fp.log_sizes(),
            self.store_add_fp_imm.log_sizes(),
            self.store_deref_fp.log_sizes(),
            self.store_div_fp_fp.log_sizes(),
            self.store_div_fp_imm.log_sizes(),
            self.store_double_deref_fp.log_sizes(),
            self.store_imm.log_sizes(),
            self.store_mul_fp_fp.log_sizes(),
            self.store_mul_fp_imm.log_sizes(),
            self.store_sub_fp_fp.log_sizes(),
            self.store_sub_fp_imm.log_sizes(),
        ];
        TreeVec::concat_cols(trees.into_iter())
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.call_abs_fp.mix_into(channel);
        self.call_abs_imm.mix_into(channel);
        self.call_rel_fp.mix_into(channel);
        self.call_rel_imm.mix_into(channel);
        self.jmp_abs_add_fp_fp.mix_into(channel);
        self.jmp_abs_add_fp_imm.mix_into(channel);
        self.jmp_abs_deref_fp.mix_into(channel);
        self.jmp_abs_double_deref_fp.mix_into(channel);
        self.jmp_abs_imm.mix_into(channel);
        self.jmp_abs_mul_fp_fp.mix_into(channel);
        self.jmp_abs_mul_fp_imm.mix_into(channel);
        self.jmp_rel_add_fp_fp.mix_into(channel);
        self.jmp_rel_add_fp_imm.mix_into(channel);
        self.jmp_rel_deref_fp.mix_into(channel);
        self.jmp_rel_double_deref_fp.mix_into(channel);
        self.jmp_rel_imm.mix_into(channel);
        self.jmp_rel_mul_fp_fp.mix_into(channel);
        self.jmp_rel_mul_fp_imm.mix_into(channel);
        self.jnz_fp_fp.mix_into(channel);
        self.jnz_fp_imm.mix_into(channel);
        self.ret.mix_into(channel);
        self.store_add_fp_fp.mix_into(channel);
        self.store_add_fp_imm.mix_into(channel);
        self.store_deref_fp.mix_into(channel);
        self.store_div_fp_fp.mix_into(channel);
        self.store_div_fp_imm.mix_into(channel);
        self.store_double_deref_fp.mix_into(channel);
        self.store_imm.mix_into(channel);
        self.store_mul_fp_fp.mix_into(channel);
        self.store_mul_fp_imm.mix_into(channel);
        self.store_sub_fp_fp.mix_into(channel);
        self.store_sub_fp_imm.mix_into(channel);
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
        macro_rules! process_opcode {
            ($opcode:expr, $module:ident) => {{
                let state_data = instructions.states_by_opcodes.entry($opcode).or_default();
                let (claim, trace, interaction_data) = $module::Claim::write_trace(state_data);
                (
                    claim,
                    Box::new(trace.to_evals().into_iter()),
                    interaction_data,
                )
            }};
        }

        let (call_abs_fp_claim, call_abs_fp_trace, call_abs_fp_interaction_claim_data) =
            process_opcode!(Opcode::CallAbsFp, call_abs_fp);

        let (call_abs_imm_claim, call_abs_imm_trace, call_abs_imm_interaction_claim_data) =
            process_opcode!(Opcode::CallAbsImm, call_abs_imm);

        let (call_rel_fp_claim, call_rel_fp_trace, call_rel_fp_interaction_claim_data) =
            process_opcode!(Opcode::CallRelFp, call_rel_fp);

        let (call_rel_imm_claim, call_rel_imm_trace, call_rel_imm_interaction_claim_data) =
            process_opcode!(Opcode::CallRelImm, call_rel_imm);

        let (
            jmp_abs_add_fp_fp_claim,
            jmp_abs_add_fp_fp_trace,
            jmp_abs_add_fp_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsAddFpFp, jmp_abs_add_fp_fp);

        let (
            jmp_abs_add_fp_imm_claim,
            jmp_abs_add_fp_imm_trace,
            jmp_abs_add_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsAddFpImm, jmp_abs_add_fp_imm);

        let (
            jmp_abs_deref_fp_claim,
            jmp_abs_deref_fp_trace,
            jmp_abs_deref_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsDerefFp, jmp_abs_deref_fp);

        let (
            jmp_abs_double_deref_fp_claim,
            jmp_abs_double_deref_fp_trace,
            jmp_abs_double_deref_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsDoubleDerefFp, jmp_abs_double_deref_fp);

        let (jmp_abs_imm_claim, jmp_abs_imm_trace, jmp_abs_imm_interaction_claim_data) =
            process_opcode!(Opcode::JmpAbsImm, jmp_abs_imm);

        let (
            jmp_abs_mul_fp_fp_claim,
            jmp_abs_mul_fp_fp_trace,
            jmp_abs_mul_fp_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsMulFpFp, jmp_abs_mul_fp_fp);

        let (
            jmp_abs_mul_fp_imm_claim,
            jmp_abs_mul_fp_imm_trace,
            jmp_abs_mul_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpAbsMulFpImm, jmp_abs_mul_fp_imm);

        let (
            jmp_rel_add_fp_fp_claim,
            jmp_rel_add_fp_fp_trace,
            jmp_rel_add_fp_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelAddFpFp, jmp_rel_add_fp_fp);

        let (
            jmp_rel_add_fp_imm_claim,
            jmp_rel_add_fp_imm_trace,
            jmp_rel_add_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelAddFpImm, jmp_rel_add_fp_imm);

        let (
            jmp_rel_deref_fp_claim,
            jmp_rel_deref_fp_trace,
            jmp_rel_deref_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelDerefFp, jmp_rel_deref_fp);

        let (
            jmp_rel_double_deref_fp_claim,
            jmp_rel_double_deref_fp_trace,
            jmp_rel_double_deref_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelDoubleDerefFp, jmp_rel_double_deref_fp);

        let (jmp_rel_imm_claim, jmp_rel_imm_trace, jmp_rel_imm_interaction_claim_data) =
            process_opcode!(Opcode::JmpRelImm, jmp_rel_imm);

        let (
            jmp_rel_mul_fp_fp_claim,
            jmp_rel_mul_fp_fp_trace,
            jmp_rel_mul_fp_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelMulFpFp, jmp_rel_mul_fp_fp);

        let (
            jmp_rel_mul_fp_imm_claim,
            jmp_rel_mul_fp_imm_trace,
            jmp_rel_mul_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::JmpRelMulFpImm, jmp_rel_mul_fp_imm);

        let (jnz_fp_fp_claim, jnz_fp_fp_trace, jnz_fp_fp_interaction_claim_data) =
            process_opcode!(Opcode::JnzFpFp, jnz_fp_fp);

        let (jnz_fp_imm_claim, jnz_fp_imm_trace, jnz_fp_imm_interaction_claim_data) =
            process_opcode!(Opcode::JnzFpImm, jnz_fp_imm);

        let (ret_claim, ret_trace, ret_interaction_claim_data) = process_opcode!(Opcode::Ret, ret);

        let (store_add_fp_fp_claim, store_add_fp_fp_trace, store_add_fp_fp_interaction_claim_data) =
            process_opcode!(Opcode::StoreAddFpFp, store_add_fp_fp);

        let (
            store_add_fp_imm_claim,
            store_add_fp_imm_trace,
            store_add_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::StoreAddFpImm, store_add_fp_imm);

        let (store_deref_fp_claim, store_deref_fp_trace, store_deref_fp_interaction_claim_data) =
            process_opcode!(Opcode::StoreDerefFp, store_deref_fp);

        let (store_div_fp_fp_claim, store_div_fp_fp_trace, store_div_fp_fp_interaction_claim_data) =
            process_opcode!(Opcode::StoreDivFpFp, store_div_fp_fp);

        let (
            store_div_fp_imm_claim,
            store_div_fp_imm_trace,
            store_div_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::StoreDivFpImm, store_div_fp_imm);

        let (
            store_double_deref_fp_claim,
            store_double_deref_fp_trace,
            store_double_deref_fp_interaction_claim_data,
        ) = process_opcode!(Opcode::StoreDoubleDerefFp, store_double_deref_fp);

        let (store_imm_claim, store_imm_trace, store_imm_interaction_claim_data) =
            process_opcode!(Opcode::StoreImm, store_imm);

        let (store_mul_fp_fp_claim, store_mul_fp_fp_trace, store_mul_fp_fp_interaction_claim_data) =
            process_opcode!(Opcode::StoreMulFpFp, store_mul_fp_fp);

        let (
            store_mul_fp_imm_claim,
            store_mul_fp_imm_trace,
            store_mul_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::StoreMulFpImm, store_mul_fp_imm);

        let (store_sub_fp_fp_claim, store_sub_fp_fp_trace, store_sub_fp_fp_interaction_claim_data) =
            process_opcode!(Opcode::StoreSubFpFp, store_sub_fp_fp);

        let (
            store_sub_fp_imm_claim,
            store_sub_fp_imm_trace,
            store_sub_fp_imm_interaction_claim_data,
        ) = process_opcode!(Opcode::StoreSubFpImm, store_sub_fp_imm);

        // Gather all claims
        let claim = Self {
            call_abs_fp: call_abs_fp_claim,
            call_abs_imm: call_abs_imm_claim,
            call_rel_fp: call_rel_fp_claim,
            call_rel_imm: call_rel_imm_claim,
            jmp_abs_add_fp_fp: jmp_abs_add_fp_fp_claim,
            jmp_abs_add_fp_imm: jmp_abs_add_fp_imm_claim,
            jmp_abs_deref_fp: jmp_abs_deref_fp_claim,
            jmp_abs_double_deref_fp: jmp_abs_double_deref_fp_claim,
            jmp_abs_imm: jmp_abs_imm_claim,
            jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp_claim,
            jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm_claim,
            jmp_rel_add_fp_fp: jmp_rel_add_fp_fp_claim,
            jmp_rel_add_fp_imm: jmp_rel_add_fp_imm_claim,
            jmp_rel_deref_fp: jmp_rel_deref_fp_claim,
            jmp_rel_double_deref_fp: jmp_rel_double_deref_fp_claim,
            jmp_rel_imm: jmp_rel_imm_claim,
            jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp_claim,
            jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm_claim,
            jnz_fp_fp: jnz_fp_fp_claim,
            jnz_fp_imm: jnz_fp_imm_claim,
            ret: ret_claim,
            store_add_fp_fp: store_add_fp_fp_claim,
            store_add_fp_imm: store_add_fp_imm_claim,
            store_deref_fp: store_deref_fp_claim,
            store_div_fp_fp: store_div_fp_fp_claim,
            store_div_fp_imm: store_div_fp_imm_claim,
            store_double_deref_fp: store_double_deref_fp_claim,
            store_imm: store_imm_claim,
            store_mul_fp_fp: store_mul_fp_fp_claim,
            store_mul_fp_imm: store_mul_fp_imm_claim,
            store_sub_fp_fp: store_sub_fp_fp_claim,
            store_sub_fp_imm: store_sub_fp_imm_claim,
        };

        // Gather all lookup data
        let interaction_claim_data = InteractionClaimData {
            call_abs_fp: call_abs_fp_interaction_claim_data,
            call_abs_imm: call_abs_imm_interaction_claim_data,
            call_rel_fp: call_rel_fp_interaction_claim_data,
            call_rel_imm: call_rel_imm_interaction_claim_data,
            jmp_abs_add_fp_fp: jmp_abs_add_fp_fp_interaction_claim_data,
            jmp_abs_add_fp_imm: jmp_abs_add_fp_imm_interaction_claim_data,
            jmp_abs_deref_fp: jmp_abs_deref_fp_interaction_claim_data,
            jmp_abs_double_deref_fp: jmp_abs_double_deref_fp_interaction_claim_data,
            jmp_abs_imm: jmp_abs_imm_interaction_claim_data,
            jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp_interaction_claim_data,
            jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm_interaction_claim_data,
            jmp_rel_add_fp_fp: jmp_rel_add_fp_fp_interaction_claim_data,
            jmp_rel_add_fp_imm: jmp_rel_add_fp_imm_interaction_claim_data,
            jmp_rel_deref_fp: jmp_rel_deref_fp_interaction_claim_data,
            jmp_rel_double_deref_fp: jmp_rel_double_deref_fp_interaction_claim_data,
            jmp_rel_imm: jmp_rel_imm_interaction_claim_data,
            jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp_interaction_claim_data,
            jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm_interaction_claim_data,
            jnz_fp_fp: jnz_fp_fp_interaction_claim_data,
            jnz_fp_imm: jnz_fp_imm_interaction_claim_data,
            ret: ret_interaction_claim_data,
            store_add_fp_fp: store_add_fp_fp_interaction_claim_data,
            store_add_fp_imm: store_add_fp_imm_interaction_claim_data,
            store_deref_fp: store_deref_fp_interaction_claim_data,
            store_div_fp_fp: store_div_fp_fp_interaction_claim_data,
            store_div_fp_imm: store_div_fp_imm_interaction_claim_data,
            store_double_deref_fp: store_double_deref_fp_interaction_claim_data,
            store_imm: store_imm_interaction_claim_data,
            store_mul_fp_fp: store_mul_fp_fp_interaction_claim_data,
            store_mul_fp_imm: store_mul_fp_imm_interaction_claim_data,
            store_sub_fp_fp: store_sub_fp_fp_interaction_claim_data,
            store_sub_fp_imm: store_sub_fp_imm_interaction_claim_data,
        };

        // Combine all traces
        let trace = call_abs_fp_trace
            .into_iter()
            .chain(call_abs_imm_trace)
            .chain(call_rel_fp_trace)
            .chain(call_rel_imm_trace)
            .chain(jmp_abs_add_fp_fp_trace)
            .chain(jmp_abs_add_fp_imm_trace)
            .chain(jmp_abs_deref_fp_trace)
            .chain(jmp_abs_double_deref_fp_trace)
            .chain(jmp_abs_imm_trace)
            .chain(jmp_abs_mul_fp_fp_trace)
            .chain(jmp_abs_mul_fp_imm_trace)
            .chain(jmp_rel_add_fp_fp_trace)
            .chain(jmp_rel_add_fp_imm_trace)
            .chain(jmp_rel_deref_fp_trace)
            .chain(jmp_rel_double_deref_fp_trace)
            .chain(jmp_rel_imm_trace)
            .chain(jmp_rel_mul_fp_fp_trace)
            .chain(jmp_rel_mul_fp_imm_trace)
            .chain(jnz_fp_fp_trace)
            .chain(jnz_fp_imm_trace)
            .chain(ret_trace)
            .chain(store_add_fp_fp_trace)
            .chain(store_add_fp_imm_trace)
            .chain(store_deref_fp_trace)
            .chain(store_div_fp_fp_trace)
            .chain(store_div_fp_imm_trace)
            .chain(store_double_deref_fp_trace)
            .chain(store_imm_trace)
            .chain(store_mul_fp_fp_trace)
            .chain(store_mul_fp_imm_trace)
            .chain(store_sub_fp_fp_trace)
            .chain(store_sub_fp_imm_trace);

        (claim, trace, interaction_claim_data)
    }
}

impl InteractionClaim {
    pub fn write_interaction_trace(
        relations: &Relations,
        interaction_claim_data: &InteractionClaimData,
    ) -> (
        Self,
        impl IntoIterator<Item = CircleEvaluation<SimdBackend, M31, BitReversedOrder>>,
    ) {
        macro_rules! write_interaction_trace {
            ($opcode:ident) => {
                $opcode::InteractionClaim::write_interaction_trace(
                    &relations.registers,
                    &relations.memory,
                    &relations.range_check_20,
                    &interaction_claim_data.$opcode,
                )
            };
        }

        let (call_abs_fp, call_abs_fp_interaction_trace) = write_interaction_trace!(call_abs_fp);
        let (call_abs_imm, call_abs_imm_interaction_trace) = write_interaction_trace!(call_abs_imm);
        let (call_rel_fp, call_rel_fp_interaction_trace) = write_interaction_trace!(call_rel_fp);
        let (call_rel_imm, call_rel_imm_interaction_trace) = write_interaction_trace!(call_rel_imm);
        let (jmp_abs_add_fp_fp, jmp_abs_add_fp_fp_interaction_trace) =
            write_interaction_trace!(jmp_abs_add_fp_fp);
        let (jmp_abs_add_fp_imm, jmp_abs_add_fp_imm_interaction_trace) =
            write_interaction_trace!(jmp_abs_add_fp_imm);
        let (jmp_abs_deref_fp, jmp_abs_deref_fp_interaction_trace) =
            write_interaction_trace!(jmp_abs_deref_fp);
        let (jmp_abs_double_deref_fp, jmp_abs_double_deref_fp_interaction_trace) =
            write_interaction_trace!(jmp_abs_double_deref_fp);
        let (jmp_abs_imm, jmp_abs_imm_interaction_trace) = write_interaction_trace!(jmp_abs_imm);
        let (jmp_abs_mul_fp_fp, jmp_abs_mul_fp_fp_interaction_trace) =
            write_interaction_trace!(jmp_abs_mul_fp_fp);
        let (jmp_abs_mul_fp_imm, jmp_abs_mul_fp_imm_interaction_trace) =
            write_interaction_trace!(jmp_abs_mul_fp_imm);
        let (jmp_rel_add_fp_fp, jmp_rel_add_fp_fp_interaction_trace) =
            write_interaction_trace!(jmp_rel_add_fp_fp);
        let (jmp_rel_add_fp_imm, jmp_rel_add_fp_imm_interaction_trace) =
            write_interaction_trace!(jmp_rel_add_fp_imm);
        let (jmp_rel_deref_fp, jmp_rel_deref_fp_interaction_trace) =
            write_interaction_trace!(jmp_rel_deref_fp);
        let (jmp_rel_double_deref_fp, jmp_rel_double_deref_fp_interaction_trace) =
            write_interaction_trace!(jmp_rel_double_deref_fp);
        let (jmp_rel_imm, jmp_rel_imm_interaction_trace) = write_interaction_trace!(jmp_rel_imm);
        let (jmp_rel_mul_fp_fp, jmp_rel_mul_fp_fp_interaction_trace) =
            write_interaction_trace!(jmp_rel_mul_fp_fp);
        let (jmp_rel_mul_fp_imm, jmp_rel_mul_fp_imm_interaction_trace) =
            write_interaction_trace!(jmp_rel_mul_fp_imm);
        let (jnz_fp_fp, jnz_fp_fp_interaction_trace) = write_interaction_trace!(jnz_fp_fp);
        let (jnz_fp_imm, jnz_fp_imm_interaction_trace) = write_interaction_trace!(jnz_fp_imm);
        let (ret, ret_interaction_trace) = write_interaction_trace!(ret);
        let (store_add_fp_fp, store_add_fp_fp_interaction_trace) =
            write_interaction_trace!(store_add_fp_fp);
        let (store_add_fp_imm, store_add_fp_imm_interaction_trace) =
            write_interaction_trace!(store_add_fp_imm);
        let (store_deref_fp, store_deref_fp_interaction_trace) =
            write_interaction_trace!(store_deref_fp);
        let (store_div_fp_fp, store_div_fp_fp_interaction_trace) =
            write_interaction_trace!(store_div_fp_fp);
        let (store_div_fp_imm, store_div_fp_imm_interaction_trace) =
            write_interaction_trace!(store_div_fp_imm);
        let (store_double_deref_fp, store_double_deref_fp_interaction_trace) =
            write_interaction_trace!(store_double_deref_fp);
        let (store_imm, store_imm_interaction_trace) = write_interaction_trace!(store_imm);
        let (store_mul_fp_fp, store_mul_fp_fp_interaction_trace) =
            write_interaction_trace!(store_mul_fp_fp);
        let (store_mul_fp_imm, store_mul_fp_imm_interaction_trace) =
            write_interaction_trace!(store_mul_fp_imm);
        let (store_sub_fp_fp, store_sub_fp_fp_interaction_trace) =
            write_interaction_trace!(store_sub_fp_fp);
        let (store_sub_fp_imm, store_sub_fp_imm_interaction_trace) =
            write_interaction_trace!(store_sub_fp_imm);

        let interaction_claim = Self {
            call_abs_fp,
            call_abs_imm,
            call_rel_fp,
            call_rel_imm,
            jmp_abs_add_fp_fp,
            jmp_abs_add_fp_imm,
            jmp_abs_deref_fp,
            jmp_abs_double_deref_fp,
            jmp_abs_imm,
            jmp_abs_mul_fp_fp,
            jmp_abs_mul_fp_imm,
            jmp_rel_add_fp_fp,
            jmp_rel_add_fp_imm,
            jmp_rel_deref_fp,
            jmp_rel_double_deref_fp,
            jmp_rel_imm,
            jmp_rel_mul_fp_fp,
            jmp_rel_mul_fp_imm,
            jnz_fp_fp,
            jnz_fp_imm,
            ret,
            store_add_fp_fp,
            store_add_fp_imm,
            store_deref_fp,
            store_div_fp_fp,
            store_div_fp_imm,
            store_double_deref_fp,
            store_imm,
            store_mul_fp_fp,
            store_mul_fp_imm,
            store_sub_fp_fp,
            store_sub_fp_imm,
        };
        let interaction_trace = call_abs_fp_interaction_trace
            .into_iter()
            .chain(call_abs_imm_interaction_trace)
            .chain(call_rel_fp_interaction_trace)
            .chain(call_rel_imm_interaction_trace)
            .chain(jmp_abs_add_fp_fp_interaction_trace)
            .chain(jmp_abs_add_fp_imm_interaction_trace)
            .chain(jmp_abs_deref_fp_interaction_trace)
            .chain(jmp_abs_double_deref_fp_interaction_trace)
            .chain(jmp_abs_imm_interaction_trace)
            .chain(jmp_abs_mul_fp_fp_interaction_trace)
            .chain(jmp_abs_mul_fp_imm_interaction_trace)
            .chain(jmp_rel_add_fp_fp_interaction_trace)
            .chain(jmp_rel_add_fp_imm_interaction_trace)
            .chain(jmp_rel_deref_fp_interaction_trace)
            .chain(jmp_rel_double_deref_fp_interaction_trace)
            .chain(jmp_rel_imm_interaction_trace)
            .chain(jmp_rel_mul_fp_fp_interaction_trace)
            .chain(jmp_rel_mul_fp_imm_interaction_trace)
            .chain(jnz_fp_fp_interaction_trace)
            .chain(jnz_fp_imm_interaction_trace)
            .chain(ret_interaction_trace)
            .chain(store_add_fp_fp_interaction_trace)
            .chain(store_add_fp_imm_interaction_trace)
            .chain(store_deref_fp_interaction_trace)
            .chain(store_div_fp_fp_interaction_trace)
            .chain(store_div_fp_imm_interaction_trace)
            .chain(store_double_deref_fp_interaction_trace)
            .chain(store_imm_interaction_trace)
            .chain(store_mul_fp_fp_interaction_trace)
            .chain(store_mul_fp_imm_interaction_trace)
            .chain(store_sub_fp_fp_interaction_trace)
            .chain(store_sub_fp_imm_interaction_trace);

        (interaction_claim, interaction_trace)
    }

    pub fn claimed_sum(&self) -> SecureField {
        let mut sum = SecureField::zero();
        sum += self.call_abs_fp.claimed_sum;
        sum += self.call_abs_imm.claimed_sum;
        sum += self.call_rel_fp.claimed_sum;
        sum += self.call_rel_imm.claimed_sum;
        sum += self.jmp_abs_add_fp_fp.claimed_sum;
        sum += self.jmp_abs_add_fp_imm.claimed_sum;
        sum += self.jmp_abs_deref_fp.claimed_sum;
        sum += self.jmp_abs_double_deref_fp.claimed_sum;
        sum += self.jmp_abs_imm.claimed_sum;
        sum += self.jmp_abs_mul_fp_fp.claimed_sum;
        sum += self.jmp_abs_mul_fp_imm.claimed_sum;
        sum += self.jmp_rel_add_fp_fp.claimed_sum;
        sum += self.jmp_rel_add_fp_imm.claimed_sum;
        sum += self.jmp_rel_deref_fp.claimed_sum;
        sum += self.jmp_rel_double_deref_fp.claimed_sum;
        sum += self.jmp_rel_imm.claimed_sum;
        sum += self.jmp_rel_mul_fp_fp.claimed_sum;
        sum += self.jmp_rel_mul_fp_imm.claimed_sum;
        sum += self.jnz_fp_fp.claimed_sum;
        sum += self.jnz_fp_imm.claimed_sum;
        sum += self.ret.claimed_sum;
        sum += self.store_add_fp_fp.claimed_sum;
        sum += self.store_add_fp_imm.claimed_sum;
        sum += self.store_deref_fp.claimed_sum;
        sum += self.store_div_fp_fp.claimed_sum;
        sum += self.store_div_fp_imm.claimed_sum;
        sum += self.store_double_deref_fp.claimed_sum;
        sum += self.store_imm.claimed_sum;
        sum += self.store_mul_fp_fp.claimed_sum;
        sum += self.store_mul_fp_imm.claimed_sum;
        sum += self.store_sub_fp_fp.claimed_sum;
        sum += self.store_sub_fp_imm.claimed_sum;
        sum
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.call_abs_fp.mix_into(channel);
        self.call_abs_imm.mix_into(channel);
        self.call_rel_fp.mix_into(channel);
        self.call_rel_imm.mix_into(channel);
        self.jmp_abs_add_fp_fp.mix_into(channel);
        self.jmp_abs_add_fp_imm.mix_into(channel);
        self.jmp_abs_deref_fp.mix_into(channel);
        self.jmp_abs_double_deref_fp.mix_into(channel);
        self.jmp_abs_imm.mix_into(channel);
        self.jmp_abs_mul_fp_fp.mix_into(channel);
        self.jmp_abs_mul_fp_imm.mix_into(channel);
        self.jmp_rel_add_fp_fp.mix_into(channel);
        self.jmp_rel_add_fp_imm.mix_into(channel);
        self.jmp_rel_deref_fp.mix_into(channel);
        self.jmp_rel_double_deref_fp.mix_into(channel);
        self.jmp_rel_imm.mix_into(channel);
        self.jmp_rel_mul_fp_fp.mix_into(channel);
        self.jmp_rel_mul_fp_imm.mix_into(channel);
        self.jnz_fp_fp.mix_into(channel);
        self.jnz_fp_imm.mix_into(channel);
        self.ret.mix_into(channel);
        self.store_add_fp_fp.mix_into(channel);
        self.store_add_fp_imm.mix_into(channel);
        self.store_deref_fp.mix_into(channel);
        self.store_div_fp_fp.mix_into(channel);
        self.store_div_fp_imm.mix_into(channel);
        self.store_double_deref_fp.mix_into(channel);
        self.store_imm.mix_into(channel);
        self.store_mul_fp_fp.mix_into(channel);
        self.store_mul_fp_imm.mix_into(channel);
        self.store_sub_fp_fp.mix_into(channel);
        self.store_sub_fp_imm.mix_into(channel);
    }
}

pub struct Component {
    pub call_abs_fp: call_abs_fp::Component,
    pub call_abs_imm: call_abs_imm::Component,
    pub call_rel_fp: call_rel_fp::Component,
    pub call_rel_imm: call_rel_imm::Component,
    pub jmp_abs_add_fp_fp: jmp_abs_add_fp_fp::Component,
    pub jmp_abs_add_fp_imm: jmp_abs_add_fp_imm::Component,
    pub jmp_abs_deref_fp: jmp_abs_deref_fp::Component,
    pub jmp_abs_double_deref_fp: jmp_abs_double_deref_fp::Component,
    pub jmp_abs_imm: jmp_abs_imm::Component,
    pub jmp_abs_mul_fp_fp: jmp_abs_mul_fp_fp::Component,
    pub jmp_abs_mul_fp_imm: jmp_abs_mul_fp_imm::Component,
    pub jmp_rel_add_fp_fp: jmp_rel_add_fp_fp::Component,
    pub jmp_rel_add_fp_imm: jmp_rel_add_fp_imm::Component,
    pub jmp_rel_deref_fp: jmp_rel_deref_fp::Component,
    pub jmp_rel_double_deref_fp: jmp_rel_double_deref_fp::Component,
    pub jmp_rel_imm: jmp_rel_imm::Component,
    pub jmp_rel_mul_fp_fp: jmp_rel_mul_fp_fp::Component,
    pub jmp_rel_mul_fp_imm: jmp_rel_mul_fp_imm::Component,
    pub jnz_fp_fp: jnz_fp_fp::Component,
    pub jnz_fp_imm: jnz_fp_imm::Component,
    pub ret: ret::Component,
    pub store_add_fp_fp: store_add_fp_fp::Component,
    pub store_add_fp_imm: store_add_fp_imm::Component,
    pub store_deref_fp: store_deref_fp::Component,
    pub store_div_fp_fp: store_div_fp_fp::Component,
    pub store_div_fp_imm: store_div_fp_imm::Component,
    pub store_double_deref_fp: store_double_deref_fp::Component,
    pub store_imm: store_imm::Component,
    pub store_mul_fp_fp: store_mul_fp_fp::Component,
    pub store_mul_fp_imm: store_mul_fp_imm::Component,
    pub store_sub_fp_fp: store_sub_fp_fp::Component,
    pub store_sub_fp_imm: store_sub_fp_imm::Component,
}

impl Component {
    pub fn new(
        location_allocator: &mut TraceLocationAllocator,
        claim: &Claim,
        interaction_claim: &InteractionClaim,
        relations: &Relations,
    ) -> Self {
        macro_rules! new_component {
            ($opcode:ident) => {
                $opcode::Component::new(
                    location_allocator,
                    $opcode::Eval {
                        claim: claim.$opcode.clone(),
                        memory: relations.memory.clone(),
                        registers: relations.registers.clone(),
                        range_check_20: relations.range_check_20.clone(),
                    },
                    interaction_claim.$opcode.claimed_sum,
                )
            };
        }

        let call_abs_fp = new_component!(call_abs_fp);
        let call_abs_imm = new_component!(call_abs_imm);
        let call_rel_fp = new_component!(call_rel_fp);
        let call_rel_imm = new_component!(call_rel_imm);
        let jmp_abs_add_fp_fp = new_component!(jmp_abs_add_fp_fp);
        let jmp_abs_add_fp_imm = new_component!(jmp_abs_add_fp_imm);
        let jmp_abs_deref_fp = new_component!(jmp_abs_deref_fp);
        let jmp_abs_double_deref_fp = new_component!(jmp_abs_double_deref_fp);
        let jmp_abs_imm = new_component!(jmp_abs_imm);
        let jmp_abs_mul_fp_fp = new_component!(jmp_abs_mul_fp_fp);
        let jmp_abs_mul_fp_imm = new_component!(jmp_abs_mul_fp_imm);
        let jmp_rel_add_fp_fp = new_component!(jmp_rel_add_fp_fp);
        let jmp_rel_add_fp_imm = new_component!(jmp_rel_add_fp_imm);
        let jmp_rel_deref_fp = new_component!(jmp_rel_deref_fp);
        let jmp_rel_double_deref_fp = new_component!(jmp_rel_double_deref_fp);
        let jmp_rel_imm = new_component!(jmp_rel_imm);
        let jmp_rel_mul_fp_fp = new_component!(jmp_rel_mul_fp_fp);
        let jmp_rel_mul_fp_imm = new_component!(jmp_rel_mul_fp_imm);
        let jnz_fp_fp = new_component!(jnz_fp_fp);
        let jnz_fp_imm = new_component!(jnz_fp_imm);
        let ret = new_component!(ret);
        let store_add_fp_fp = new_component!(store_add_fp_fp);
        let store_add_fp_imm = new_component!(store_add_fp_imm);
        let store_deref_fp = new_component!(store_deref_fp);
        let store_div_fp_fp = new_component!(store_div_fp_fp);
        let store_div_fp_imm = new_component!(store_div_fp_imm);
        let store_double_deref_fp = new_component!(store_double_deref_fp);
        let store_imm = new_component!(store_imm);
        let store_mul_fp_fp = new_component!(store_mul_fp_fp);
        let store_mul_fp_imm = new_component!(store_mul_fp_imm);
        let store_sub_fp_fp = new_component!(store_sub_fp_fp);
        let store_sub_fp_imm = new_component!(store_sub_fp_imm);

        Self {
            call_abs_fp,
            call_abs_imm,
            call_rel_fp,
            call_rel_imm,
            jmp_abs_add_fp_fp,
            jmp_abs_add_fp_imm,
            jmp_abs_deref_fp,
            jmp_abs_double_deref_fp,
            jmp_abs_imm,
            jmp_abs_mul_fp_fp,
            jmp_abs_mul_fp_imm,
            jmp_rel_add_fp_fp,
            jmp_rel_add_fp_imm,
            jmp_rel_deref_fp,
            jmp_rel_double_deref_fp,
            jmp_rel_imm,
            jmp_rel_mul_fp_fp,
            jmp_rel_mul_fp_imm,
            jnz_fp_fp,
            jnz_fp_imm,
            ret,
            store_add_fp_fp,
            store_add_fp_imm,
            store_deref_fp,
            store_div_fp_fp,
            store_div_fp_imm,
            store_double_deref_fp,
            store_imm,
            store_mul_fp_fp,
            store_mul_fp_imm,
            store_sub_fp_fp,
            store_sub_fp_imm,
        }
    }

    pub fn provers(&self) -> Vec<&dyn ComponentProver<SimdBackend>> {
        vec![
            &self.call_abs_fp,
            &self.call_abs_imm,
            &self.call_rel_fp,
            &self.call_rel_imm,
            &self.jmp_abs_add_fp_fp,
            &self.jmp_abs_add_fp_imm,
            &self.jmp_abs_deref_fp,
            &self.jmp_abs_double_deref_fp,
            &self.jmp_abs_imm,
            &self.jmp_abs_mul_fp_fp,
            &self.jmp_abs_mul_fp_imm,
            &self.jmp_rel_add_fp_fp,
            &self.jmp_rel_add_fp_imm,
            &self.jmp_rel_deref_fp,
            &self.jmp_rel_double_deref_fp,
            &self.jmp_rel_imm,
            &self.jmp_rel_mul_fp_fp,
            &self.jmp_rel_mul_fp_imm,
            &self.jnz_fp_fp,
            &self.jnz_fp_imm,
            &self.ret,
            &self.store_add_fp_fp,
            &self.store_add_fp_imm,
            &self.store_deref_fp,
            &self.store_div_fp_fp,
            &self.store_div_fp_imm,
            &self.store_double_deref_fp,
            &self.store_imm,
            &self.store_mul_fp_fp,
            &self.store_mul_fp_imm,
            &self.store_sub_fp_fp,
            &self.store_sub_fp_imm,
        ]
    }

    pub fn verifiers(&self) -> Vec<&dyn ComponentVerifier> {
        vec![
            &self.call_abs_fp,
            &self.call_abs_imm,
            &self.call_rel_fp,
            &self.call_rel_imm,
            &self.jmp_abs_add_fp_fp,
            &self.jmp_abs_add_fp_imm,
            &self.jmp_abs_deref_fp,
            &self.jmp_abs_double_deref_fp,
            &self.jmp_abs_imm,
            &self.jmp_abs_mul_fp_fp,
            &self.jmp_abs_mul_fp_imm,
            &self.jmp_rel_add_fp_fp,
            &self.jmp_rel_add_fp_imm,
            &self.jmp_rel_deref_fp,
            &self.jmp_rel_double_deref_fp,
            &self.jmp_rel_imm,
            &self.jmp_rel_mul_fp_fp,
            &self.jmp_rel_mul_fp_imm,
            &self.jnz_fp_fp,
            &self.jnz_fp_imm,
            &self.ret,
            &self.store_add_fp_fp,
            &self.store_add_fp_imm,
            &self.store_deref_fp,
            &self.store_div_fp_fp,
            &self.store_div_fp_imm,
            &self.store_double_deref_fp,
            &self.store_imm,
            &self.store_mul_fp_fp,
            &self.store_mul_fp_imm,
            &self.store_sub_fp_fp,
            &self.store_sub_fp_imm,
        ]
    }
}
