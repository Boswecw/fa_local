use thiserror::Error;

use crate::domain::guards::DenialGuard;

pub type FaLocalResult<T> = Result<T, FaLocalError>;

#[derive(Debug, Error)]
pub enum FaLocalError {
    #[error(transparent)]
    Denied(#[from] DenialGuard),

    #[error("contract invalid: {0}")]
    ContractInvalid(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal invariant violated: {0}")]
    InternalInvariant(String),
}
