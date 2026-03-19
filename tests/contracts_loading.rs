mod support;

use fa_local::DenialGuard;
use fa_local::deserialize_contract_value;
use fa_local::domain::capabilities::CapabilityRegistryLoader;
use fa_local::domain::execution::ExecutionRequest;
use fa_local::domain::policy::PolicyArtifactLoader;
use fa_local::domain::requester_trust::RequesterTrustEngine;
use fa_local::domain::routing::RouteDecisionLoader;
use fa_local::{EnvironmentMode, RequesterClass, SchemaName};

#[test]
fn valid_requester_trust_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "requester-trust-basic.json");
    let envelope = RequesterTrustEngine::load_contract_value(&value).unwrap();

    assert_eq!(envelope.requester_class, RequesterClass::TrustedAppSurface);
    assert_eq!(envelope.environment_mode, EnvironmentMode::Prod);
    assert_eq!(envelope.app_context.app_id, "forge-author");
}

#[test]
fn valid_policy_artifact_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "policy-artifact-basic.json");
    let policy = PolicyArtifactLoader::load_contract_value(&value).unwrap();

    assert_eq!(policy.scope.service_id, "fa-local");
    assert_eq!(policy.capability_rules.len(), 1);
    assert_eq!(policy.side_effect_rules.len(), 1);
}

#[test]
fn valid_capability_registry_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "capability-registry-basic.json");
    let registry = CapabilityRegistryLoader::load_contract_value(&value).unwrap();

    assert_eq!(registry.capabilities.len(), 1);
    assert_eq!(registry.capabilities[0].owner_service, "fa-local");
}

#[test]
fn valid_execution_request_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "execution-request-basic.json");
    let request = ExecutionRequest::load_contract_value(&value).unwrap();

    assert_eq!(request.environment_mode, EnvironmentMode::Prod);
    assert_eq!(request.intent_summary, "write approved local export");
}

#[test]
fn valid_denial_guard_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "denial-guard-basic.json");
    let guard: DenialGuard = deserialize_contract_value(SchemaName::DenialGuard, &value).unwrap();

    assert!(!guard.remediable);
    assert_eq!(guard.summary, "unknown requester denied");
}

#[test]
fn valid_route_decision_fixture_loads_into_typed_model() {
    let value = support::load_fixture_json("valid", "route-decision-policy-preapproved-basic.json");
    let decision = RouteDecisionLoader::load_contract_value(&value).unwrap();

    assert!(decision.execution_allowed);
    assert_eq!(decision.denial_guards.len(), 0);
    assert_eq!(
        decision.operator_visible_summary,
        "request is policy preapproved for capability 44444444-4444-4444-8444-444444444444"
    );
}
