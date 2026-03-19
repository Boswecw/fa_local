use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::shared::{DenialBasis, DenialReasonClass, DenialScope, TimestampUtc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{summary}")]
pub struct DenialGuard {
    pub reason_class: DenialReasonClass,
    pub scope: DenialScope,
    pub basis: DenialBasis,
    pub remediable: bool,
    pub review_available: bool,
    pub summary: String,
    pub timestamp_utc: TimestampUtc,
}

impl DenialGuard {
    pub fn new(
        reason_class: DenialReasonClass,
        scope: DenialScope,
        basis: DenialBasis,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            reason_class,
            scope,
            basis,
            remediable: false,
            review_available: false,
            summary: summary.into(),
            timestamp_utc: Utc::now(),
        }
    }

    pub fn remediable(mut self, remediable: bool) -> Self {
        self.remediable = remediable;
        self
    }

    pub fn review_available(mut self, review_available: bool) -> Self {
        self.review_available = review_available;
        self
    }
}

pub fn deny(
    reason_class: DenialReasonClass,
    scope: DenialScope,
    basis: DenialBasis,
    summary: impl Into<String>,
) -> DenialGuard {
    DenialGuard::new(reason_class, scope, basis, summary)
}

pub fn ensure(condition: bool, denial: impl FnOnce() -> DenialGuard) -> Result<(), DenialGuard> {
    if condition { Ok(()) } else { Err(denial()) }
}

pub fn fail_closed<T>(
    value: Option<T>,
    denial: impl FnOnce() -> DenialGuard,
) -> Result<T, DenialGuard> {
    value.ok_or_else(denial)
}
