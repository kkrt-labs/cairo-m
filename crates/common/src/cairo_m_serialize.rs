//! # Cairo-M Serialization Trait
//!
//! This module provides a principled, zero-copy ABI layer between Rust and Cairo-M memory.
//! It allows any Rust type to declare how it is laid out in fp-relative memory.

use stwo_prover::core::fields::m31::M31;

/// Convert between a Rust value and its Cairo-M stack representation.
///
/// Implementations MUST guarantee that `encode` followed by `decode` round-trips
/// and that `SIZE` matches the number of memory slots written/read.
pub trait CairoMSerialize: Sized {
    /// How many felts the value occupies at runtime.
    const SIZE: usize;

    /// Push the *little-endian* felt representation onto `dst`.
    /// (`dst` is a grow-only buffer so we can reuse the same Vec for all args)
    fn encode(&self, dst: &mut Vec<M31>);

    /// Reconstruct the value from `src[start…start+SIZE)`.
    ///
    /// Returns the reconstructed value **and** the next cursor position so that
    /// higher-level code can chain decoders without manual indexing.
    ///
    /// # Panics
    /// May panic if `src.len() < start + Self::SIZE`. We can switch to `Result`
    /// if we want fallible decoding later.
    fn decode(src: &[M31], start: usize) -> (Self, usize);
}

// Primitive implementations

impl CairoMSerialize for M31 {
    const SIZE: usize = 1;

    #[inline]
    fn encode(&self, dst: &mut Vec<M31>) {
        dst.push(*self);
    }

    #[inline]
    fn decode(src: &[M31], start: usize) -> (Self, usize) {
        (src[start], start + Self::SIZE)
    }
}

impl CairoMSerialize for u32 {
    const SIZE: usize = 2;

    #[inline]
    fn encode(&self, dst: &mut Vec<M31>) {
        let lo = M31::from(*self & 0xFFFF);
        let hi = M31::from(*self >> 16);
        dst.extend_from_slice(&[lo, hi]);
    }

    #[inline]
    fn decode(src: &[M31], start: usize) -> (Self, usize) {
        let lo = src[start].0;
        let hi = src[start + 1].0;
        (lo | (hi << 16), start + Self::SIZE)
    }
}

// Utility helpers

/// Encode a homogeneous argument list into felt words
pub fn encode_args<T: CairoMSerialize>(args: &[T]) -> Vec<M31> {
    let mut dst = Vec::with_capacity(args.len() * T::SIZE);
    for arg in args {
        arg.encode(&mut dst);
    }
    dst
}

/// Decode a single value from a slice of M31 words
pub fn decode_value<T: CairoMSerialize>(src: &[M31]) -> T {
    let (value, _) = T::decode(src, 0);
    value
}

/// Encode multiple heterogeneous values
///
/// # Example
/// ```ignore
/// let mut encoded = Vec::new();
/// encode_many(&mut encoded, &[&123u32, &M31::from(456)]);
/// ```
pub fn encode_many(dst: &mut Vec<M31>, values: &[&dyn EncodableValue]) {
    for value in values {
        value.encode_dyn(dst);
    }
}

/// Helper trait for dynamic encoding (used in encode_many)
pub trait EncodableValue {
    fn encode_dyn(&self, dst: &mut Vec<M31>);
}

impl<T: CairoMSerialize> EncodableValue for T {
    fn encode_dyn(&self, dst: &mut Vec<M31>) {
        self.encode(dst);
    }
}
