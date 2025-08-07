#![feature(iter_advance_by)]
#![feature(raw_slice_split)]
#![feature(portable_simd)]
#![feature(iter_array_chunks)]

pub mod poseidon31_merkle;

use num_traits::Zero;
use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::pcs::{CommitmentSchemeVerifier, PcsConfig};
use stwo_prover::core::prover::{verify, VerificationError as StwoVerificationError};
use tracing::{info, span, Level};

use cairo_m_prover::components::{Components, Relations};
use cairo_m_prover::preprocessed::PreProcessedTraceBuilder;
use cairo_m_prover::prover_config::REGULAR_96_BITS;
use cairo_m_prover::{relations, Proof};

pub use poseidon31_merkle::{Poseidon31Channel, Poseidon31MerkleChannel, Poseidon31MerkleHasher};

#[derive(thiserror::Error, Debug)]
pub enum VerificationError {
    #[error("Stwo verification error: {0}")]
    Stwo(#[from] StwoVerificationError),

    #[error("Invalid logup sum")]
    InvalidLogupSum,
}

pub fn verify_cairo_m<MC: MerkleChannel>(
    proof: Proof<MC::H>,
    pcs_config: Option<PcsConfig>,
) -> Result<(), VerificationError>
where
    SimdBackend: BackendForChannel<MC>,
{
    let _span = span!(Level::INFO, "verify_cairo_m").entered();

    // Setup protocol.
    let channel = &mut MC::C::default();

    let pcs_config = pcs_config.unwrap_or(REGULAR_96_BITS);
    pcs_config.mix_into(channel);

    let commitment_scheme_verifier = &mut CommitmentSchemeVerifier::<MC>::new(pcs_config);

    // Preprocessed trace.
    info!("preprocessed trace");
    let preprocessed_trace = PreProcessedTraceBuilder::default().build();
    // TODO: assert proof.stark_proof.commitments[0] == known_root of preprocessed trace commitment
    commitment_scheme_verifier.commit(
        proof.stark_proof.commitments[0],
        &preprocessed_trace.log_sizes(),
        channel,
    );

    // Execution traces
    info!("execution trace");
    proof.claim.mix_into(channel);
    commitment_scheme_verifier.commit(
        proof.stark_proof.commitments[1],
        &proof.claim.log_sizes()[1],
        channel,
    );

    // Proof of work.
    channel.mix_u64(proof.interaction_pow);
    if channel.trailing_zeros() < relations::INTERACTION_POW_BITS {
        return Err(VerificationError::Stwo(StwoVerificationError::ProofOfWork));
    }

    info!("interaction trace");
    let relations = Relations::draw(channel);

    // Verify lookup argument.
    if proof
        .interaction_claim
        .claimed_sum(&relations, proof.public_data)
        != SecureField::zero()
    {
        return Err(VerificationError::InvalidLogupSum);
    }
    proof.interaction_claim.mix_into(channel);
    commitment_scheme_verifier.commit(
        proof.stark_proof.commitments[2],
        &proof.claim.log_sizes()[2],
        channel,
    );

    // Verify stark.
    info!("verify stark");
    let mut tree_span_provider =
        TraceLocationAllocator::new_with_preproccessed_columns(&preprocessed_trace.ids());
    let components = Components::new(
        &mut tree_span_provider,
        &proof.claim,
        &proof.interaction_claim,
        &relations,
    );
    verify(
        &components.verifiers(),
        channel,
        commitment_scheme_verifier,
        proof.stark_proof,
    )
    .map_err(VerificationError::from)
}
