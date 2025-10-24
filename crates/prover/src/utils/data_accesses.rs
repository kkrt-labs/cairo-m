use num_traits::Zero;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::core::fields::m31::M31;

use crate::adapter::memory::DataAccess;
use crate::utils::execution_bundle::PackedExecutionBundle;

/// Gather a PackedM31 column by reading the k-th access from each lane's span
/// in the global access log and extracting a field from `DataAccess`.
pub fn get_access_field<F>(
    input: &PackedExecutionBundle,
    data_accesses: &[DataAccess],
    idx_in_span: usize,
    f: F,
) -> PackedM31
where
    F: Fn(&DataAccess) -> M31 + Copy,
{
    PackedM31::from_array(std::array::from_fn(|i| {
        let s = input.span_start[i];
        let l = input.span_len[i] as usize;
        data_accesses
            .get(s..s + l)
            .and_then(|a| a.get(idx_in_span))
            .map(f)
            .unwrap_or_else(M31::zero)
    }))
}

pub fn get_prev_clock(
    input: &PackedExecutionBundle,
    data_accesses: &[DataAccess],
    idx_in_span: usize,
) -> PackedM31 {
    get_access_field(input, data_accesses, idx_in_span, |d| d.prev_clock)
}

pub fn get_prev_value(
    input: &PackedExecutionBundle,
    data_accesses: &[DataAccess],
    idx_in_span: usize,
) -> PackedM31 {
    get_access_field(input, data_accesses, idx_in_span, |d| d.prev_value)
}

pub fn get_value(
    input: &PackedExecutionBundle,
    data_accesses: &[DataAccess],
    idx_in_span: usize,
) -> PackedM31 {
    get_access_field(input, data_accesses, idx_in_span, |d| d.value)
}

pub fn get_address(
    input: &PackedExecutionBundle,
    data_accesses: &[DataAccess],
    idx_in_span: usize,
) -> PackedM31 {
    get_access_field(input, data_accesses, idx_in_span, |d| d.address)
}
