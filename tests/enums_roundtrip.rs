use fa_local::{
    ApprovalPosture, DegradedSubtype, DenialBasis, DenialReasonClass, DenialScope, EnvironmentMode,
    ExecutionState, RequesterClass, RevocationState, SideEffectClass,
};

#[test]
fn serializes_enums_with_stable_snake_case_labels() {
    assert_eq!(
        serde_json::to_string(&EnvironmentMode::TestHarness).unwrap(),
        "\"test_harness\""
    );
    assert_eq!(
        serde_json::to_string(&RequesterClass::TrustedInternalService).unwrap(),
        "\"trusted_internal_service\""
    );
    assert_eq!(
        serde_json::to_string(&ApprovalPosture::ExplicitOperatorApproval).unwrap(),
        "\"explicit_operator_approval\""
    );
    assert_eq!(
        serde_json::to_string(&ExecutionState::WaitingExplicitApproval).unwrap(),
        "\"waiting_explicit_approval\""
    );
    assert_eq!(
        serde_json::to_string(&DegradedSubtype::UnavailableDependencyBlock).unwrap(),
        "\"unavailable_dependency_block\""
    );
    assert_eq!(
        serde_json::to_string(&SideEffectClass::ExternalNetworkDeniedByDefault).unwrap(),
        "\"external_network_denied_by_default\""
    );
    assert_eq!(
        serde_json::to_string(&RevocationState::Revoked).unwrap(),
        "\"revoked\""
    );
    assert_eq!(
        serde_json::to_string(&DenialReasonClass::CapabilityNotAdmitted).unwrap(),
        "\"capability_not_admitted\""
    );
    assert_eq!(
        serde_json::to_string(&DenialScope::Artifact).unwrap(),
        "\"artifact\""
    );
    assert_eq!(
        serde_json::to_string(&DenialBasis::ContractAndPolicy).unwrap(),
        "\"contract_and_policy\""
    );
}

#[test]
fn round_trips_baseline_enums() {
    let samples = [
        serde_json::to_string(&EnvironmentMode::Airgapped).unwrap(),
        serde_json::to_string(&RequesterClass::ReviewSurface).unwrap(),
        serde_json::to_string(&ApprovalPosture::PolicyPreapproved).unwrap(),
        serde_json::to_string(&ExecutionState::CompletedWithConstraints).unwrap(),
        serde_json::to_string(&DegradedSubtype::DegradedFallbackLimited).unwrap(),
        serde_json::to_string(&SideEffectClass::LocalDbMutation).unwrap(),
        serde_json::to_string(&RevocationState::Disabled).unwrap(),
        serde_json::to_string(&DenialReasonClass::MissingPolicy).unwrap(),
        serde_json::to_string(&DenialScope::Request).unwrap(),
        serde_json::to_string(&DenialBasis::RuntimeSafety).unwrap(),
    ];

    assert_eq!(
        serde_json::from_str::<EnvironmentMode>(&samples[0]).unwrap(),
        EnvironmentMode::Airgapped
    );
    assert_eq!(
        serde_json::from_str::<RequesterClass>(&samples[1]).unwrap(),
        RequesterClass::ReviewSurface
    );
    assert_eq!(
        serde_json::from_str::<ApprovalPosture>(&samples[2]).unwrap(),
        ApprovalPosture::PolicyPreapproved
    );
    assert_eq!(
        serde_json::from_str::<ExecutionState>(&samples[3]).unwrap(),
        ExecutionState::CompletedWithConstraints
    );
    assert_eq!(
        serde_json::from_str::<DegradedSubtype>(&samples[4]).unwrap(),
        DegradedSubtype::DegradedFallbackLimited
    );
    assert_eq!(
        serde_json::from_str::<SideEffectClass>(&samples[5]).unwrap(),
        SideEffectClass::LocalDbMutation
    );
    assert_eq!(
        serde_json::from_str::<RevocationState>(&samples[6]).unwrap(),
        RevocationState::Disabled
    );
    assert_eq!(
        serde_json::from_str::<DenialReasonClass>(&samples[7]).unwrap(),
        DenialReasonClass::MissingPolicy
    );
    assert_eq!(
        serde_json::from_str::<DenialScope>(&samples[8]).unwrap(),
        DenialScope::Request
    );
    assert_eq!(
        serde_json::from_str::<DenialBasis>(&samples[9]).unwrap(),
        DenialBasis::RuntimeSafety
    );
}

#[test]
fn rejects_unknown_enum_values() {
    assert!(serde_json::from_str::<EnvironmentMode>("\"production\"").is_err());
    assert!(serde_json::from_str::<RequesterClass>("\"operator\"").is_err());
    assert!(serde_json::from_str::<ApprovalPosture>("\"allowed\"").is_err());
    assert!(serde_json::from_str::<DegradedSubtype>("\"completed_with_constraints\"").is_err());
    assert!(serde_json::from_str::<RevocationState>("\"paused\"").is_err());
}
