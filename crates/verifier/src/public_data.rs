use serde::{Deserialize, Serialize};
use stwo_prover::core::fields::qm31::{SecureField, QM31};
use stwo_prover::core::fields::FieldExpOps;

use crate::components::Relations;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PublicData {}

impl PublicData {
    pub fn initial_logup_sum(&self, _relations: &Relations) -> SecureField {
        let values_to_inverse = vec![];

        let inverted_values = QM31::batch_inverse(&values_to_inverse);
        inverted_values.iter().sum::<QM31>()
    }
}
