use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentMode {
    Dev,
    Test,
    Staging,
    Prod,
    Airgapped,
    TestHarness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequesterClass {
    TrustedAppSurface,
    TrustedInternalService,
    ReviewSurface,
    DevelopmentTestSurface,
    UntrustedUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPosture {
    Denied,
    ReviewRequired,
    ExplicitOperatorApproval,
    PolicyPreapproved,
    ExecuteAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Denied,
    ReviewRequired,
    WaitingExplicitApproval,
    AdmittedNotStarted,
    InProgress,
    Degraded,
    PartialSuccess,
    CompletedWithConstraints,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradedSubtype {
    DegradedPreStart,
    DegradedInFlight,
    DegradedFallbackEquivalent,
    DegradedFallbackLimited,
    DegradedPartial,
    UnavailableDependencyBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    None,
    LocalFileWrite,
    LocalDbMutation,
    LocalProcessSpawn,
    ExternalNetworkDeniedByDefault,
    OtherGoverned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevocationState {
    Active,
    Disabled,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenialReasonClass {
    UnknownRequester,
    UntrustedRequester,
    MissingPolicy,
    PolicyDenied,
    CapabilityNotAdmitted,
    ContractInvalid,
    IntegrityFailed,
    DependencyUnavailable,
    PrivacyScopeViolation,
    ReviewRequired,
    UnsupportedRoute,
    DisabledByOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenialScope {
    Request,
    Capability,
    Route,
    Service,
    Artifact,
    Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenialBasis {
    Contract,
    Policy,
    ContractAndPolicy,
    RuntimeSafety,
}
