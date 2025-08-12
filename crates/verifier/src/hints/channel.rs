use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::circle::CirclePoint;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::queries::Queries;

use crate::components::Relations;
use cairo_m_prover::Proof;

/// Stores all data received from channel requests during proof verification
#[derive(Debug, Clone)]
pub struct ChannelHints {
    /// First request: Relations drawn after initial commitments and interaction pow
    pub relations: Relations,

    /// Second request: Random coefficient after interaction claim mixed in
    pub cp_coeff: SecureField,

    /// Third request: OODS point after composition commitment
    pub oods_point: CirclePoint<SecureField>,

    /// Fourth request: Random coefficient after OODS values mixed in
    pub random_coeff_after_oods: SecureField,

    /// Fifth+ requests: Folding alphas for each FRI layer (dynamic count)
    pub fri_folding_alphas: Vec<SecureField>,

    /// Final request: Query positions after proof of work
    pub queries: Queries,
}

impl ChannelHints {
    pub fn new() -> Self {
        // Use dummy values for initialization
        // These will be replaced with actual values during transcript reconstruction
        use crate::relations;
        Self {
            relations: Relations {
                poseidon2: relations::Poseidon2::dummy(),
            },
            cp_coeff: SecureField::default(),
            oods_point: CirclePoint::zero(),
            random_coeff_after_oods: SecureField::default(),
            fri_folding_alphas: Vec::new(),
            queries: Queries {
                positions: Vec::new(),
                log_domain_size: 0,
            },
        }
    }
}

impl Default for ChannelHints {
    fn default() -> Self {
        Self::new()
    }
}

/// Rebuild the channel transcript from a proof and extract all channel hints
/// This function is generic over any MerkleChannel implementation
pub fn hints<MC: MerkleChannel>(proof: &Proof<MC::H>) -> ChannelHints
where
    SimdBackend: BackendForChannel<MC>,
{
    use crate::prover_config::REGULAR_96_BITS;

    let mut hints = ChannelHints::new();
    let channel = &mut MC::C::default();
    let pcs_config = REGULAR_96_BITS;

    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              Commitments                                 ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    // Preprocessed trace
    pcs_config.mix_into(channel);
    MC::mix_root(channel, proof.stark_proof.0.commitments[0]);

    // Execution trace
    proof.claim.mix_into(channel);
    MC::mix_root(channel, proof.stark_proof.0.commitments[1]);

    // Interaction trace
    channel.mix_u64(proof.interaction_pow);
    hints.relations = Relations::draw(channel);
    proof.interaction_claim.mix_into(channel);
    MC::mix_root(channel, proof.stark_proof.0.commitments[2]);

    // Composition polynomial
    hints.cp_coeff = channel.draw_secure_felt();
    MC::mix_root(channel, *proof.stark_proof.0.commitments.last().unwrap());

    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              OODS                                        ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    hints.oods_point = CirclePoint::<SecureField>::get_random_point(channel);
    channel.mix_felts(&proof.stark_proof.0.sampled_values.clone().flatten_cols());
    hints.random_coeff_after_oods = channel.draw_secure_felt();

    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              FRI                                         ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    // Commit to first layer
    MC::mix_root(
        channel,
        proof.stark_proof.0.fri_proof.first_layer.commitment,
    );

    // Inner layers
    for layer in &proof.stark_proof.0.fri_proof.inner_layers {
        // Commit to layer
        MC::mix_root(channel, layer.commitment);

        // Draw folding alpha
        let folding_alpha = channel.draw_secure_felt();
        hints.fri_folding_alphas.push(folding_alpha);
    }

    // Mix the last layer polynomial coefficients
    channel.mix_felts(&proof.stark_proof.0.fri_proof.last_layer_poly);

    // Mix proof of work
    channel.mix_u64(proof.stark_proof.0.proof_of_work);

    // ╔══════════════════════════════════════════════════════════════════════════╗
    // ║                              Queries                                     ║
    // ╚══════════════════════════════════════════════════════════════════════════╝
    // Determine sampling range
    let max_column_log_size = proof
        .claim
        .log_sizes()
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(0);

    hints.queries = Queries::generate(
        channel,
        max_column_log_size + 2,
        pcs_config.fri_config.n_queries,
    );

    hints
}
