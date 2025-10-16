#![feature(iter_advance_by)]
#![feature(raw_slice_split)]
#![feature(portable_simd)]
#![feature(iter_array_chunks)]

//! # Cairo-M Prover
//!
//! The prover crate implements a STARK (Scalable Transparent ARgument of Knowledge) prover
//! for the Cairo-M virtual machine using Starkware's Stwo proving system. It generates
//! cryptographic proofs that demonstrate correct execution of Cairo-M programs.
//!
//! ## Architecture Overview
//!
//! The prover follows a three-layer architecture:
//!
//! ┌─────────────┐    ┌──────────────┐    ┌─────────────┐
//! │   Runner    │───▶│   Adapter    │───▶│ Components  │
//! │ Execution   │    │   Layer      │    │   & Proof   │
//! │   Trace     │    │              │    │ Generation  │
//! └─────────────┘    └──────────────┘    └─────────────┘

pub mod adapter;
pub mod components;
pub mod debug_tools;
pub mod errors;
pub mod poseidon2;
pub mod preprocessed;
pub mod prover;
pub mod prover_config;
pub mod public_data;
pub mod relations;
pub mod utils;
pub mod verifier;

use std::collections::HashMap;

use adapter::merkle::build_partial_merkle_tree;
use cairo_m_common::PublicAddressRanges;
use num_traits::Zero;
use poseidon2::Poseidon2Hash;
use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::prover::StarkProof;
use stwo_prover::core::vcs::ops::MerkleHasher;

use crate::components::{Claim, InteractionClaim};
use crate::public_data::PublicData;

/// A complete cryptographic proof for a Cairo-M program execution.
///
/// This structure contains all the necessary components to verify that a Cairo-M
/// program was executed correctly, including:
/// - Execution trace claims and proofs
/// - Public input/output data
/// - The underlying STARK proof
/// - Proof-of-work nonce for additional security
///
/// ## Type Parameters
/// * `H` - The Merkle hasher used for tree commitments (typically Blake2s)
#[derive(Serialize, Deserialize, Clone)]
pub struct Proof<H: MerkleHasher> {
    /// Claim about the execution trace (log sizes for each component)
    pub claim: Claim,
    /// Claim about interaction trace (claimed sums for each component)
    pub interaction_claim: InteractionClaim,
    /// Public data: VM initial and final state, public memory (program, input, output)
    pub public_data: PublicData,
    /// The underlying STARK proof containing polynomial commitments and evaluations
    pub stark_proof: StarkProof<H>,
    /// Proof-of-work nonce
    pub interaction_pow: u64,
}

impl<H: MerkleHasher> Proof<H> {
    pub fn program_id(&self) -> M31 {
        // Reconstruct HashMap from program
        let mut program_map = HashMap::<M31, (QM31, M31, M31)>::new();
        for (addr, value, _clock) in self
            .public_data
            .public_memory
            .program
            .iter()
            .map(|res| res.unwrap())
        {
            program_map.insert(addr, (value, M31::zero(), M31::zero()));
        }

        // Compute Poseidon2 hash of the program.
        let (_, program_id) = build_partial_merkle_tree::<Poseidon2Hash>(
            &program_map,
            adapter::merkle::TreeType::Initial,
            &PublicAddressRanges::default(),
        );

        program_id.unwrap()
    }
}
