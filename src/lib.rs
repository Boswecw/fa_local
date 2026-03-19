pub mod adapters;
pub mod app;
pub mod config;
pub mod domain;
pub mod errors;
pub mod integrations;

pub use config::{CRATE_VERSION, SERVICE_ID};
pub use domain::guards::{DenialGuard, deny, ensure, fail_closed};
pub use domain::shared::{
    ApprovalPosture, CapabilityId, CorrelationId, DegradedSubtype, DenialBasis, DenialReasonClass,
    DenialScope, EnvironmentMode, ExecutionPlanId, ExecutionState, ForensicEventId, PolicyId,
    RequestId, RequesterClass, RequesterId, ReviewPackageId, RevocationState, RouteDecisionId,
    SideEffectClass, TimestampUtc, now_utc,
};
pub use errors::{FaLocalError, FaLocalResult};
