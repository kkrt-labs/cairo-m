#![feature(iter_advance_by)]
#![feature(raw_slice_split)]
#![feature(portable_simd)]
#![feature(iter_array_chunks)]

pub mod poseidon31_merkle;
pub use poseidon31_merkle::{Poseidon31Channel, Poseidon31MerkleChannel, Poseidon31MerkleHasher};
