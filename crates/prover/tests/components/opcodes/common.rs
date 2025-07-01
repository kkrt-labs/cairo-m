//! Common test utilities for opcode constraint tests.

/// Helper macro to reduce boilerplate for testing opcode constraints.
#[macro_export]
macro_rules! test_opcode_constraints {
    ($execution_bundles:expr, $opcode_module:path) => {{
        use $opcode_module as opcode;

        let mut execution_bundles = $execution_bundles;

        let mut commitment_scheme = cairo_m_prover::debug_tools::assert_constraints::MockCommitmentScheme::default();

        // Preprocessed trace
        let preprocessed_trace = cairo_m_prover::preprocessed::PreProcessedTraceBuilder::default().build();
        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(preprocessed_trace.gen_trace());
        tree_builder.finalize_interaction();

        // Write trace for the opcode
        let (claim, trace, interaction_claim_data) =
            opcode::Claim::write_trace::<stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel>(&mut execution_bundles);

        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(trace.to_evals());
        tree_builder.finalize_interaction();

        // Interaction trace
        let mut dummy_channel = stwo_prover::core::channel::Blake2sChannel::default();
        let relations = cairo_m_prover::components::Relations::draw(&mut dummy_channel);

        let (interaction_claim, interaction_trace) =
            opcode::InteractionClaim::write_interaction_trace(
                &relations.registers,
                &relations.memory,
                &relations.range_check_20,
                &interaction_claim_data,
            );

        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(interaction_trace);
        tree_builder.finalize_interaction();

        // Create component
        let mut tree_span_provider =
            stwo_prover::constraint_framework::TraceLocationAllocator::new_with_preproccessed_columns(&preprocessed_trace.ids());

        let eval = opcode::Eval {
            claim: claim.clone(),
            memory: relations.memory.clone(),
            registers: relations.registers.clone(),
            range_check_20: relations.range_check_20.clone(),
        };

        let component = stwo_prover::constraint_framework::FrameworkComponent::new(
            &mut tree_span_provider,
            eval,
            interaction_claim.claimed_sum
        );

        // Extract relevant trace columns
        let trace = commitment_scheme.trace_domain_evaluations();
        let mut component_trace = trace
            .sub_tree(component.trace_locations())
            .map(|tree| tree.into_iter().cloned().collect::<Vec<_>>());

        component_trace[stwo_prover::constraint_framework::PREPROCESSED_TRACE_IDX] = component
            .preproccessed_column_indices()
            .iter()
            .map(|idx| trace[stwo_prover::constraint_framework::PREPROCESSED_TRACE_IDX][*idx])
            .collect();

        let log_size = component.log_size();
        let component_eval = component.deref();

        // Assert constraints
        stwo_prover::constraint_framework::assert_constraints_on_trace(
            &component_trace,
            log_size,
            |eval| {
                component_eval.evaluate(eval);
            },
            component.claimed_sum(),
        );
    }};
}
