//! End-to-end tests for the admission facade (AdmissionService::admit).
//!
//! Uses the aligned per-contract `valid` fixtures (trusted_app_surface +
//! capability 4444… + local_file_write + policy_preapproved) so the gate
//! resolves to an executable decision, plus fail-closed variants.

mod support;

use chrono::{DateTime, Utc};
use fa_local::{
    AdmissionContext, AdmissionInputs, AdmissionService, ApprovalPosture, EnvironmentMode,
};

fn at(ts: &str) -> DateTime<Utc> {
    ts.parse().expect("valid RFC3339 timestamp")
}

fn basic_inputs() -> AdmissionInputs {
    AdmissionInputs {
        requester_trust: support::load_fixture_json("valid", "requester-trust-basic.json"),
        policy: support::load_fixture_json("valid", "policy-artifact-basic.json"),
        capability_registry: support::load_fixture_json("valid", "capability-registry-basic.json"),
        execution_request: support::load_fixture_json("valid", "execution-request-basic.json"),
    }
}

#[test]
fn aligned_inputs_resolve_to_executable_policy_preapproved() {
    let context = AdmissionContext::new(EnvironmentMode::Prod, at("2030-01-01T00:00:00Z"));
    let outcome = AdmissionService
        .admit(&basic_inputs(), context)
        .expect("admit ok");

    assert_eq!(
        outcome.route_decision.resolved_approval_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert!(outcome.route_decision.execution_allowed);
    assert!(outcome.route_decision.denial_guards.is_empty());
    // Executable path emits no review package.
    assert!(outcome.review_package.is_none());
}

#[test]
fn expired_requester_fails_closed_with_denial() {
    // now is past the requester token expiry (2030-01-02) -> trust denied ->
    // capability admission skipped -> route denied.
    let context = AdmissionContext::new(EnvironmentMode::Prod, at("2030-02-01T00:00:00Z"));
    let outcome = AdmissionService
        .admit(&basic_inputs(), context)
        .expect("admit ok");

    assert!(!outcome.route_decision.execution_allowed);
    assert_eq!(
        outcome.route_decision.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!outcome.route_decision.denial_guards.is_empty());
    assert!(outcome.review_package.is_none());
}

#[test]
fn environment_mismatch_fails_closed() {
    // envelope environment_mode is prod; expecting dev -> trust denied.
    let context = AdmissionContext::new(EnvironmentMode::Dev, at("2030-01-01T00:00:00Z"));
    let outcome = AdmissionService
        .admit(&basic_inputs(), context)
        .expect("admit ok");

    assert!(!outcome.route_decision.execution_allowed);
    assert!(!outcome.route_decision.denial_guards.is_empty());
}

#[test]
fn malformed_execution_request_is_a_hard_error() {
    // The request is the thing being admitted; if it cannot parse, admit errors
    // rather than fabricating a decision.
    let mut inputs = basic_inputs();
    inputs.execution_request = serde_json::json!({ "not": "a request" });
    let context = AdmissionContext::new(EnvironmentMode::Prod, at("2030-01-01T00:00:00Z"));

    assert!(AdmissionService.admit(&inputs, context).is_err());
}
