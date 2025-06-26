use std::time::Instant;

use stwo_prover::constraint_framework::TraceLocationAllocator;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::channel::{Channel, MerkleChannel};
use stwo_prover::core::pcs::{CommitmentSchemeProver, PcsConfig};
use stwo_prover::core::poly::circle::{CanonicCoset, PolyOps};
use stwo_prover::core::proof_of_work::GrindOps;
use stwo_prover::core::prover::prove;
use tracing::{info, span, Level};

use crate::adapter::ProverInput;
use crate::components::{Claim, Components, InteractionClaim, Relations};
use crate::errors::ProvingError;
use crate::preprocessed::PreProcessedTraceBuilder;
use crate::public_data::PublicData;
use crate::{relations, Proof};

pub(crate) const PREPROCESSED_TRACE_LOG_SIZE: u32 = 20;

pub fn prove_cairo_m<MC: MerkleChannel>(
    input: &mut ProverInput,
) -> Result<Proof<MC::H>, ProvingError>
where
    SimdBackend: BackendForChannel<MC>,
{
    let _span = span!(Level::INFO, "prove_cairo_m").entered();

    // Setup protocol.
    let channel = &mut MC::C::default();

    let pcs_config = PcsConfig::default();
    pcs_config.mix_into(channel);

    let trace_log_size = std::cmp::max(
        PREPROCESSED_TRACE_LOG_SIZE,
        std::cmp::max(
            (input.memory_boundaries.initial_memory.len()
                + input.memory_boundaries.final_memory.len())
            .next_power_of_two()
            .ilog2(),
            input
                .instructions
                .states_by_opcodes
                .values()
                .map(|states| states.len().next_power_of_two())
                .max()
                .unwrap_or(1)
                .ilog2(),
        ),
    );

    info!("twiddles");
    let twiddles = SimdBackend::precompute_twiddles(
        CanonicCoset::new(trace_log_size + pcs_config.fri_config.log_blowup_factor + 2)
            .circle_domain()
            .half_coset,
    );
    let mut commitment_scheme =
        CommitmentSchemeProver::<SimdBackend, MC>::new(pcs_config, &twiddles);

    let public_data = PublicData::new(&input.instructions);
    dbg!(&public_data);

    // Preprocessed traces
    info!("preprocessed trace");
    let preprocessed_trace = PreProcessedTraceBuilder::default().build();
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(preprocessed_trace.gen_trace());
    tree_builder.commit(channel);

    // Execution traces
    info!("execution trace");
    dbg!(&input.instructions);
    let (claim, trace, lookup_data) = Claim::write_trace(input);
    dbg!(&claim);
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
        use crate::debug_tools::track_and_summarize_relations;
        let summary = track_and_summarize_relations(&commitment_scheme, &components, &public_data);
        println!("Relations summary: {:?}", summary);
    }

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
        public_data,
        stark_proof,
        interaction_pow,
    })
}
