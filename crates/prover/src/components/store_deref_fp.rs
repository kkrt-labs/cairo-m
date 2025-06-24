use stwo_prover::core::channel::Channel;
use stwo_prover::core::fields::qm31::SecureField;
use stwo_prover::core::fields::secure_column::SECURE_EXTENSION_DEGREE;
use stwo_prover::core::pcs::TreeVec;

const COMPONENT_COLUMNS: usize = 3;

#[derive(Clone, Default)]
pub struct Claim {
    pub log_size: u32,
}

impl Claim {
    pub const fn new(log_size: u32) -> Self {
        Self { log_size }
    }

    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u64(self.log_size as u64);
    }

    pub fn log_sizes(&self) -> TreeVec<Vec<u32>> {
        let trace = vec![self.log_size; COMPONENT_COLUMNS];
        // TODO: check the correct width of vector for the interaction trace
        let interaction_trace = vec![self.log_size; SECURE_EXTENSION_DEGREE * COMPONENT_COLUMNS];
        TreeVec::new(vec![vec![], trace, interaction_trace])
    }
}

#[derive(Clone)]
pub struct InteractionClaim {
    pub claimed_sum: SecureField,
}
impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_felts(&[self.claimed_sum]);
    }
}
