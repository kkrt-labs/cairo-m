use std::collections::BTreeMap;
use std::iter::Peekable;

use itertools::Itertools;
use num_traits::Zero;
use stwo_prover::core::backend::simd::SimdBackend;
use stwo_prover::core::backend::BackendForChannel;
use stwo_prover::core::fields::m31::{BaseField, M31};
use stwo_prover::core::pcs::TreeVec;
use stwo_prover::core::queries::Queries;
use stwo_prover::core::utils::PeekableExt;
use stwo_prover::core::vcs::ops::MerkleHasher;
use stwo_prover::core::vcs::prover::MerkleDecommitment;
use thiserror::Error;

use cairo_m_prover::Proof;

use crate::poseidon31_merkle::{M31Hash, Poseidon31MerkleHasher, ELEMENTS_IN_BLOCK};
use crate::Poseidon31MerkleChannel;

/// Errors that can occur during decommitment hints generation
#[derive(Debug, Error)]
pub enum DecommitmentError {
    #[error("Witness too short: expected {expected} values, got {got}")]
    WitnessTooShort { expected: usize, got: usize },

    #[error("Hash witness exhausted at layer_log_size={layer_log_size}")]
    HashWitnessExhausted { layer_log_size: u32 },

    #[error("Column witness exhausted")]
    ColumnWitnessExhausted,

    #[error("Too many queried values")]
    TooManyQueriedValues,

    #[error("Witness too long")]
    WitnessTooLong,

    #[error("Root mismatch")]
    RootMismatch,
}

/// Represents a single row in the Merkle decommitment hints table
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleDecommitmentHintRow {
    /// The root of the Merkle tree (the commitment)
    pub root: M31Hash,
    /// The depth in the tree (0 is the root layer)
    pub depth: u32,
    /// The node index at this layer
    pub node_index: usize,
    /// Left value in the hashing process
    pub x_0: M31,
    /// Right value in the hashing process
    pub x_1: M31,
    /// Parent hash
    pub parent_hash: M31,
    /// Flag that equals 1 when this is the final hash that produces the parent
    pub final_hash: u8,
}

/// Collection of all Merkle decommitment hints needed for verification
#[derive(Debug, Clone, Default)]
pub struct MerkleDecommitmentHints {
    /// All hint rows needed for the verification
    /// Organized as a flat vector of rows
    pub rows: Vec<MerkleDecommitmentHintRow>,
}

impl MerkleDecommitmentHints {
    pub const fn new() -> Self {
        Self { rows: Vec::new() }
    }

    #[allow(clippy::too_many_arguments)]
    /// Add a hint row for a hash computation
    pub fn add_row(
        &mut self,
        root: M31Hash,
        depth: u32,
        node_index: usize,
        x_0: M31,
        x_1: M31,
        parent_hash: M31,
        final_hash: bool,
    ) {
        self.rows.push(MerkleDecommitmentHintRow {
            root,
            depth,
            node_index,
            x_0,
            x_1,
            parent_hash,
            final_hash: final_hash as u8,
        });
    }
}

/// Generate Merkle decommitment hints by following the verification process
pub fn hints(
    proof: &Proof<Poseidon31MerkleHasher>,
    queries: &Queries,
    column_log_sizes: &TreeVec<Vec<u32>>,
) -> Result<MerkleDecommitmentHints, DecommitmentError>
where
    SimdBackend: BackendForChannel<Poseidon31MerkleChannel>,
{
    let mut hints_collector = MerkleDecommitmentHints::new();

    // Generate query positions per log size
    let queries_per_log_size = get_queries_per_log_size(
        queries,
        column_log_sizes,
        proof.stark_proof.0.config.fri_config.log_blowup_factor,
    );

    // Process each commitment (there are 4 commitments in the proof)
    for (commitment_idx, commitment) in proof.stark_proof.0.commitments.iter().enumerate() {
        assert!(commitment_idx < proof.stark_proof.0.decommitments.len());

        let decommitment = &proof.stark_proof.0.decommitments[commitment_idx];
        let queried_values = &proof.stark_proof.0.queried_values[commitment_idx];
        let commitment_column_log_sizes = &column_log_sizes[commitment_idx];

        // Generate hints for this commitment's decommitment
        verify_tree_decommitment(
            &mut hints_collector,
            *commitment,
            decommitment,
            queried_values,
            commitment_column_log_sizes,
            &queries_per_log_size,
            proof.stark_proof.0.config.fri_config.log_blowup_factor,
        )?;
    }

    Ok(hints_collector)
}

/// Generate hints for a single Merkle tree decommitment
fn verify_tree_decommitment(
    hints: &mut MerkleDecommitmentHints,
    root: M31Hash,
    decommitment: &MerkleDecommitment<Poseidon31MerkleHasher>,
    queried_values: &[BaseField],
    column_log_sizes: &[u32],
    queries_per_log_size: &BTreeMap<u32, Vec<usize>>,
    log_blowup_factor: u32,
) -> Result<(), DecommitmentError> {
    // Count columns per extended log size (domain size)
    let mut n_columns_per_log_size = BTreeMap::new();
    for log_size in column_log_sizes {
        let extended_log_size = log_size + log_blowup_factor;
        *n_columns_per_log_size.entry(extended_log_size).or_insert(0) += 1;
    }

    let max_log_size = column_log_sizes.iter().max().unwrap() + log_blowup_factor;

    let mut queried_values_iter = queried_values.iter().copied();
    let mut hash_witness_iter = decommitment.hash_witness.iter().copied();
    let mut column_witness_iter = decommitment.column_witness.iter().copied();

    let mut last_layer_hashes: Option<Vec<(usize, M31Hash)>> = None;
    dbg!(&n_columns_per_log_size);
    // Process layers from leaf to root
    for layer_log_size in (0..=max_log_size).rev() {
        let n_columns_in_layer = *n_columns_per_log_size.get(&layer_log_size).unwrap_or(&0);

        let mut layer_total_queries = vec![];

        // Setup iterators for queries
        let mut prev_layer_queries = last_layer_hashes
            .iter()
            .flatten()
            .map(|(q, _)| *q)
            .collect_vec()
            .into_iter()
            .peekable();
        let mut prev_layer_hashes = last_layer_hashes.as_ref().map(|x| x.iter().peekable());
        let mut layer_column_queries = queries_per_log_size
            .get(&layer_log_size)
            .into_iter()
            .flatten()
            .copied()
            .peekable();

        // Process each node that needs to be computed
        while let Some(node_index) =
            get_next_node(&mut prev_layer_queries, &mut layer_column_queries)
        {
            prev_layer_queries
                .peek_take_while(|q| q / 2 == node_index)
                .for_each(drop);

            let node_hashes = prev_layer_hashes
                .as_mut()
                .map(|prev_layer_hashes| {
                    // If the left child was not computed, read it from the witness.
                    let left_hash = prev_layer_hashes
                        .next_if(|(index, _)| *index == 2 * node_index)
                        .map(|(_, hash)| *hash)
                        .or_else(|| hash_witness_iter.next())
                        .ok_or(DecommitmentError::HashWitnessExhausted { layer_log_size })?;

                    // If the right child was not computed, read it from the witness.
                    let right_hash = prev_layer_hashes
                        .next_if(|(index, _)| *index == 2 * node_index + 1)
                        .map(|(_, hash)| *hash)
                        .or_else(|| hash_witness_iter.next())
                        .ok_or(DecommitmentError::HashWitnessExhausted { layer_log_size })?;

                    Ok((left_hash, right_hash))
                })
                .transpose()?;

            let node_values_iter = match layer_column_queries.next_if_eq(&node_index) {
                // If the column values were queried, read them from `queried_value`.
                Some(_) => {
                    println!("layer {} node {} queried", layer_log_size, node_index);
                    &mut queried_values_iter
                }
                // Otherwise, read them from the witness.
                None => {
                    println!("layer {} node {} witness", layer_log_size, node_index);
                    &mut column_witness_iter
                }
            };

            let node_values = node_values_iter.take(n_columns_in_layer).collect_vec();
            if node_values.len() != n_columns_in_layer {
                return Err(DecommitmentError::WitnessTooShort {
                    expected: n_columns_in_layer,
                    got: node_values.len(),
                });
            }

            // Generate hint rows for this node's hash computation
            let hinted_parent_hash = generate_node_hints(
                hints,
                root,
                layer_log_size,
                node_index,
                node_hashes,
                &node_values,
            );

            let parent_hash =
                <Poseidon31MerkleHasher as MerkleHasher>::hash_node(node_hashes, &node_values);

            assert_eq!(
                hinted_parent_hash,
                parent_hash.into(),
                "Parent hash mismatch: layer_log_size={}, node_index={}",
                layer_log_size,
                node_index,
            );

            // Compute and store the hash for this node
            layer_total_queries.push((node_index, parent_hash));
        }

        last_layer_hashes = Some(layer_total_queries);
    }

    // Check that all witnesses and values have been consumed.
    if hash_witness_iter.next().is_some() {
        return Err(DecommitmentError::WitnessTooLong);
    }
    if queried_values_iter.next().is_some() {
        return Err(DecommitmentError::TooManyQueriedValues);
    }
    if column_witness_iter.next().is_some() {
        return Err(DecommitmentError::ColumnWitnessExhausted);
    }

    let last_layer = last_layer_hashes.unwrap();
    if last_layer.len() != 1 {
        return Err(DecommitmentError::RootMismatch);
    }
    let (_, computed_root) = last_layer[0];
    if computed_root != root {
        return Err(DecommitmentError::RootMismatch);
    }

    Ok(())
}

/// Generate hint rows for a single node's hash computation
/// Following the actual implementation pattern:
/// 1. Collect children hashes (if any)
/// 2. Hash column values in blocks of ELEMENTS_IN_BLOCK using hash_pair
/// 3. Hash all collected values together using hash_pair iteratively
fn generate_node_hints(
    hints: &mut MerkleDecommitmentHints,
    root: M31Hash,
    depth: u32,
    node_index: usize,
    children_hashes: Option<(M31Hash, M31Hash)>,
    column_values: &[BaseField],
) -> M31 {
    let n_column_blocks = column_values.len().div_ceil(ELEMENTS_IN_BLOCK);
    let mut values_to_hash = Vec::with_capacity(2 + n_column_blocks);

    // Step 1: Add children hashes if they exist
    if let Some((left, right)) = children_hashes {
        values_to_hash.push(left.0);
        values_to_hash.push(right.0);
    }

    // Step 2: Process column values in blocks and compute block hashes using hash_pair
    if !column_values.is_empty() {
        let padding_length = ELEMENTS_IN_BLOCK * n_column_blocks - column_values.len();
        let padded_values = column_values
            .iter()
            .copied()
            .chain(std::iter::repeat(BaseField::zero()).take(padding_length))
            .collect::<Vec<_>>();

        for chunk in padded_values.chunks(ELEMENTS_IN_BLOCK) {
            // Hash each block using hash_pair iteratively
            values_to_hash.push({
                // Multiple elements, hash them with hash_pair
                let mut acc = chunk[0];
                for value in chunk[1..].iter() {
                    let new_hash = Poseidon31MerkleHasher::hash_pair(acc, *value);

                    // Record hint for this pair hash within the block
                    hints.add_row(
                        root, depth, node_index, acc, *value, new_hash, false, // Not final
                    );

                    acc = new_hash;
                }
                acc
            });
        }
    }

    // Step 3: Hash all collected values together using hash_pair iteratively
    if values_to_hash.is_empty() {
        // No values to hash (shouldn't happen in practice)
        M31::zero()
    } else if values_to_hash.len() == 1 {
        // Single value, return as is
        values_to_hash[0]
    } else {
        // Multiple values, hash them together using hash_pair
        let mut acc = values_to_hash[0];
        for i in 1..values_to_hash.len() {
            let new_hash = Poseidon31MerkleHasher::hash_pair(acc, values_to_hash[i]);

            // Record hint for this final stage hash
            hints.add_row(
                root,
                depth,
                node_index,
                acc,
                values_to_hash[i],
                new_hash,
                i == values_to_hash.len() - 1, // Mark as final if this is the last hash
            );

            acc = new_hash;
        }
        acc
    }
}

/// Get the next node index to process from the queries
fn get_next_node(
    prev_queries: &mut Peekable<impl Iterator<Item = usize>>,
    layer_queries: &mut Peekable<impl Iterator<Item = usize>>,
) -> Option<usize> {
    prev_queries
        .peek()
        .map(|q| *q / 2)
        .into_iter()
        .chain(layer_queries.peek().into_iter().copied())
        .min()
}

/// Generate query positions mapped by extended log size
fn get_queries_per_log_size(
    queries: &Queries,
    column_log_sizes: &[Vec<u32>],
    log_blowup_factor: u32,
) -> BTreeMap<u32, Vec<usize>> {
    column_log_sizes
        .iter()
        .flatten()
        .sorted()
        .rev()
        .dedup()
        .map(|column_log_size| {
            let log_domain_size = column_log_size + log_blowup_factor;
            let column_queries = queries.fold(queries.log_domain_size - log_domain_size);
            (log_domain_size, column_queries.positions)
        })
        .collect()
}
