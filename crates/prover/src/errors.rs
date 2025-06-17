use stwo_prover::core::prover::{
    ProvingError as StwoProvingError, VerificationError as StwoVerificationError,
};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum VerificationError {
    #[error("Invalid logup sum.")]
    InvalidLogupSum,
    #[error(transparent)]
    Stwo(#[from] StwoVerificationError),
}

#[derive(Clone, Debug, Error)]
pub enum ProvingError {
    #[error(transparent)]
    Stwo(#[from] StwoProvingError),
}
