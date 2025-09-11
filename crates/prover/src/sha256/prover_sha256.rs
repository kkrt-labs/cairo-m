use std::time::Instant;

use stwo_constraint_framework::TraceLocationAllocator;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::pcs::{CommitmentSchemeProver, PcsConfig};
use stwo_prover::core::poly::circle::{CanonicCoset, PolyOps};
use stwo_prover::core::proof_of_work::GrindOps;
use stwo_prover::core::prover::prove;
use tracing::{info, span, Level};

use crate::adapter::SHA256HashInput;
use crate::errors::ProvingError;
use crate::preprocessed::PreProcessedTraceBuilder;
use crate::prover_config::REGULAR_96_BITS;
use crate::relations;
use crate::sha256::{Claim, Components, InteractionClaim, Proof, Relations};

const MAX_TRACE_LOG_SIZE: u32 = 21;

pub fn prove_sha256<MC: MerkleChannel>(
    inputs: &Vec<SHA256HashInput>,
    pcs_config: Option<PcsConfig>,
) -> Result<Proof<MC::H>, ProvingError>
where
    SimdBackend: BackendForChannel<MC>,
{
    let _span = span!(Level::INFO, "prove_sha256").entered();

    // Setup protocol.
    let channel = &mut MC::C::default();

    let pcs_config = pcs_config.unwrap_or(REGULAR_96_BITS);
    pcs_config.mix_into(channel);

    let trace_log_size = MAX_TRACE_LOG_SIZE;

    info!("twiddles");
    let twiddles = SimdBackend::precompute_twiddles(
        CanonicCoset::new(trace_log_size + pcs_config.fri_config.log_blowup_factor + 2)
            .circle_domain()
            .half_coset,
    );

    let mut commitment_scheme =
        CommitmentSchemeProver::<SimdBackend, MC>::new(pcs_config, &twiddles);

    // Preprocessed traces
    info!("preprocessed trace");
    let preprocessed_trace = PreProcessedTraceBuilder::for_sha256().build();
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(preprocessed_trace.gen_trace());
    tree_builder.commit(channel);

    // Execution traces
    info!("execution trace");
    let (claim, trace, lookup_data) = Claim::write_trace::<MC>(inputs);
    claim.mix_into(channel);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(trace);
    tree_builder.commit(channel);

    // Interaction trace
    // Draw interaction elements.
    info!(
        "proof of work with {} bits",
        relations::INTERACTION_POW_BITS
    );
    let interaction_pow = SimdBackend::grind(channel, relations::INTERACTION_POW_BITS);
    channel.mix_u64(interaction_pow);

    info!("interaction trace");
    let relations = Relations::draw(channel);

    let (interaction_trace, interaction_claim) =
        InteractionClaim::write_interaction_trace(&relations, &lookup_data);
    interaction_claim.mix_into(channel);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(interaction_trace);
    tree_builder.commit(channel);

    // Prove stark.
    info!("prove stark");
    let mut tree_span_provider =
        TraceLocationAllocator::new_with_preproccessed_columns(&preprocessed_trace.ids());
    let components = Components::new(
        &mut tree_span_provider,
        &claim,
        &interaction_claim,
        &relations,
    );

    #[cfg(feature = "relation-tracker")]
    {
        use crate::sha256::debug_tools::relation_tracker::track_and_summarize_relations;
        let summary = track_and_summarize_relations(&commitment_scheme, &components);
        println!("Relations summary: {:?}", summary);
    }
    info!(
        "commitment scheme tree widths: {:?} (evaluations per tree)",
        commitment_scheme
            .trees
            .as_ref()
            .map(|tree| tree.evaluations.len())
    );

    let proving_start = Instant::now();

    let stark_proof = prove::<SimdBackend, _>(&components.provers(), channel, commitment_scheme)
        .map_err(ProvingError::from)?;

    let proving_duration = proving_start.elapsed();
    let proving_mhz = ((1 << trace_log_size) as f64) / proving_duration.as_secs_f64() / 1_000_000.0;
    info!("Trace size: {:?}", 1 << trace_log_size);
    info!("Proving time: {:?}", proving_duration);
    info!("Proving speed: {:.2} MHz", proving_mhz);

    Ok(Proof {
        claim,
        interaction_claim,
        stark_proof,
        interaction_pow,
    })
}
