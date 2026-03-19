mod support;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::domain::capabilities::{CapabilityRegistry, CapabilityRegistryLoader, EnabledState};
use fa_local::domain::execution::ExecutionRequest;
use fa_local::domain::policy::{PolicyArtifact, PolicyArtifactLoader};
use fa_local::domain::requester_trust::{
    RequesterTrustEngine, RequesterTrustEnvelope, TrustEvaluationContext,
};
use fa_local::{
    CapabilityId, DenialReasonClass, DenialScope, RequesterClass, RevocationState, SideEffectClass,
};

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap()
}

fn trust_context() -> TrustEvaluationContext {
    TrustEvaluationContext {
        expected_environment: fa_local::EnvironmentMode::Prod,
        now: fixed_now(),
    }
}

fn valid_requester() -> RequesterTrustEnvelope {
    let value = support::load_fixture_json("valid", "requester-trust-basic.json");
    RequesterTrustEngine::load_contract_value(&value).unwrap()
}

fn valid_policy() -> PolicyArtifact {
    let value = support::load_fixture_json("valid", "policy-artifact-basic.json");
    PolicyArtifactLoader::load_contract_value(&value).unwrap()
}

fn valid_registry() -> CapabilityRegistry {
    let value = support::load_fixture_json("valid", "capability-registry-basic.json");
    CapabilityRegistryLoader::load_contract_value(&value).unwrap()
}

fn valid_request() -> ExecutionRequest {
    let value = support::load_fixture_json("valid", "execution-request-basic.json");
    ExecutionRequest::load_contract_value(&value).unwrap()
}

#[test]
fn denies_unknown_requester() {
    let mut requester = valid_requester();
    requester.requester_class = RequesterClass::UntrustedUnknown;

    let error = RequesterTrustEngine::evaluate(&requester, &trust_context()).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::UnknownRequester);
    assert_eq!(error.scope, DenialScope::Request);
}

#[test]
fn denies_malformed_requester_envelope() {
    let value = support::load_fixture_json("invalid", "requester-trust-missing-requester-id.json");
    let error = RequesterTrustEngine::load_and_evaluate(&value, &trust_context()).unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(error.scope, DenialScope::Request);
}

#[test]
fn denies_environment_mismatch() {
    let requester = valid_requester();
    let context = TrustEvaluationContext {
        expected_environment: fa_local::EnvironmentMode::Test,
        now: fixed_now(),
    };

    let error = RequesterTrustEngine::evaluate(&requester, &context).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(error.summary, "environment mismatch");
}

#[test]
fn denies_invalid_request_token_or_nonce() {
    let mut requester = valid_requester();
    requester.request_nonce_or_token = "bad!".to_owned();

    let error = RequesterTrustEngine::evaluate(&requester, &trust_context()).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::UntrustedRequester);
    assert_eq!(error.summary, "request token or nonce is invalid");
}

#[test]
fn denies_expired_request_token_or_nonce() {
    let mut requester = valid_requester();
    requester.expires_at = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();

    let error = RequesterTrustEngine::evaluate(&requester, &trust_context()).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::UntrustedRequester);
    assert_eq!(error.summary, "request token or nonce is expired");
}

#[test]
fn denies_missing_trust_basis() {
    let value = support::load_fixture_json("invalid", "requester-trust-missing-trust-basis.json");
    let error = RequesterTrustEngine::load_and_evaluate(&value, &trust_context()).unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::UntrustedRequester);
    assert_eq!(error.summary, "missing trust basis");
}

#[test]
fn denies_invalid_policy() {
    let value =
        support::load_fixture_json("invalid", "policy-artifact-invalid-failure-behavior.json");
    let error = PolicyArtifactLoader::load_required_value(Some(&value)).unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(error.scope, DenialScope::Artifact);
}

#[test]
fn denies_missing_policy() {
    let error = PolicyArtifactLoader::load_required_value(None).unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::MissingPolicy);
    assert_eq!(error.scope, DenialScope::Artifact);
}

#[test]
fn denies_unregistered_capability() {
    let requester = valid_requester();
    let policy = valid_policy();
    let registry = valid_registry();
    let mut request = valid_request();
    request.requested_capability_id =
        CapabilityId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap());

    let error =
        CapabilityRegistryLoader::admit_execution_request(&registry, &policy, &requester, &request)
            .unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::CapabilityNotAdmitted);
    assert_eq!(error.summary, "unregistered capability");
}

#[test]
fn denies_disabled_capability() {
    let requester = valid_requester();
    let policy = valid_policy();
    let mut registry = valid_registry();
    let request = valid_request();
    registry.capabilities[0].enabled_state = EnabledState::Disabled;

    let error =
        CapabilityRegistryLoader::admit_execution_request(&registry, &policy, &requester, &request)
            .unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::DisabledByOperator);
    assert_eq!(error.summary, "capability is disabled");
}

#[test]
fn denies_revoked_capability() {
    let requester = valid_requester();
    let policy = valid_policy();
    let mut registry = valid_registry();
    let request = valid_request();
    registry.capabilities[0].revocation_state = RevocationState::Revoked;

    let error =
        CapabilityRegistryLoader::admit_execution_request(&registry, &policy, &requester, &request)
            .unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::CapabilityNotAdmitted);
    assert_eq!(error.summary, "capability is revoked");
}

#[test]
fn denies_policy_capability_mismatch() {
    let requester = valid_requester();
    let mut policy = valid_policy();
    let registry = valid_registry();
    let request = valid_request();
    policy.capability_rules[0].allowed_side_effect_classes = vec![SideEffectClass::LocalDbMutation];

    let error =
        CapabilityRegistryLoader::admit_execution_request(&registry, &policy, &requester, &request)
            .unwrap_err();

    assert_eq!(error.reason_class, DenialReasonClass::PolicyDenied);
    assert_eq!(
        error.summary,
        "policy/capability mismatch: side effect class not allowed"
    );
}
