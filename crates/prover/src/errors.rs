use stwo::core::verifier::VerificationError as StwoVerificationError;
use stwo::prover::ProvingError as StwoProvingError;
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
