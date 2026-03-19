mod support;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::deny;
use fa_local::domain::capabilities::{
    CapabilityRecord, CapabilityRegistry, CapabilityRegistryLoader, ReviewClass,
};
use fa_local::domain::execution::ExecutionRequest;
use fa_local::domain::policy::{PolicyArtifact, PolicyArtifactLoader};
use fa_local::domain::posture::{
    ApprovalPostureResolver, RouteResolutionContext, RouteResolutionInput,
};
use fa_local::domain::requester_trust::{RequesterTrustEngine, RequesterTrustEnvelope};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{
    ApprovalPosture, DenialBasis, DenialReasonClass, DenialScope, RequesterId, RouteDecisionId,
    SideEffectClass,
};

fn decision_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 1, 1, 0, 5, 0).unwrap()
}

fn decision_context(value: &str) -> RouteResolutionContext {
    RouteResolutionContext::new(
        RouteDecisionId::from_uuid(Uuid::parse_str(value).unwrap()),
        decision_time(),
    )
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

fn valid_capability() -> CapabilityRecord {
    valid_registry().capabilities.into_iter().next().unwrap()
}

fn valid_request() -> ExecutionRequest {
    let value = support::load_fixture_json("valid", "execution-request-basic.json");
    ExecutionRequest::load_contract_value(&value).unwrap()
}

fn expected_decision(file_name: &str) -> RouteDecision {
    let value = support::load_fixture_json("valid", file_name);
    RouteDecisionLoader::load_contract_value(&value).unwrap()
}

fn resolve(
    request: ExecutionRequest,
    requester: Result<RequesterTrustEnvelope, fa_local::DenialGuard>,
    policy: Result<PolicyArtifact, fa_local::DenialGuard>,
    capability: Result<CapabilityRecord, fa_local::DenialGuard>,
    context: RouteResolutionContext,
) -> RouteDecision {
    ApprovalPostureResolver::resolve(
        RouteResolutionInput {
            request,
            requester_trust_outcome: requester,
            policy_outcome: policy,
            capability_admission_outcome: capability,
        },
        context,
    )
}

#[test]
fn golden_denied_route_decision_matches_fixture() {
    let request = {
        let mut request = valid_request();
        request.requested_side_effect_class = SideEffectClass::ExternalNetworkDeniedByDefault;
        request
    };
    let requester = valid_requester();
    let policy = {
        let mut policy = valid_policy();
        policy.capability_rules[0].allowed_side_effect_classes =
            vec![SideEffectClass::ExternalNetworkDeniedByDefault];
        policy.capability_rules[0].required_approval_posture =
            ApprovalPosture::ExplicitOperatorApproval;
        policy.side_effect_rules[0].side_effect_class =
            SideEffectClass::ExternalNetworkDeniedByDefault;
        policy
    };
    let capability = {
        let mut capability = valid_capability();
        capability.side_effect_class = SideEffectClass::ExternalNetworkDeniedByDefault;
        capability.approval_posture = ApprovalPosture::ExplicitOperatorApproval;
        capability
    };

    let actual = resolve(
        request,
        Ok(requester),
        Ok(policy),
        Ok(capability),
        decision_context("77777777-7777-4777-8777-777777777771"),
    );

    assert_eq!(
        actual,
        expected_decision("route-decision-denied-basic.json")
    );
}

#[test]
fn golden_review_required_route_decision_matches_fixture() {
    let request = valid_request();
    let requester = valid_requester();
    let policy = {
        let mut policy = valid_policy();
        policy.capability_rules[0].required_approval_posture = ApprovalPosture::ReviewRequired;
        policy
    };
    let capability = valid_capability();

    let actual = resolve(
        request,
        Ok(requester),
        Ok(policy),
        Ok(capability),
        decision_context("77777777-7777-4777-8777-777777777772"),
    );

    assert_eq!(
        actual,
        expected_decision("route-decision-review-required-basic.json")
    );
}

#[test]
fn golden_explicit_operator_approval_route_decision_matches_fixture() {
    let request = valid_request();
    let requester = valid_requester();
    let policy = valid_policy();
    let capability = {
        let mut capability = valid_capability();
        capability.review_class = ReviewClass::Operator;
        capability
    };

    let actual = resolve(
        request,
        Ok(requester),
        Ok(policy),
        Ok(capability),
        decision_context("77777777-7777-4777-8777-777777777773"),
    );

    assert_eq!(
        actual,
        expected_decision("route-decision-explicit-operator-approval-basic.json")
    );
}

#[test]
fn golden_policy_preapproved_route_decision_matches_fixture() {
    let actual = resolve(
        valid_request(),
        Ok(valid_requester()),
        Ok(valid_policy()),
        Ok(valid_capability()),
        decision_context("77777777-7777-4777-8777-777777777774"),
    );

    assert_eq!(
        actual,
        expected_decision("route-decision-policy-preapproved-basic.json")
    );
}

#[test]
fn golden_execute_allowed_route_decision_matches_fixture() {
    let request = {
        let mut request = valid_request();
        request.requested_side_effect_class = SideEffectClass::None;
        request
    };
    let requester = valid_requester();
    let policy = {
        let mut policy = valid_policy();
        policy.capability_rules[0].allowed_side_effect_classes = vec![SideEffectClass::None];
        policy.capability_rules[0].required_approval_posture = ApprovalPosture::ExecuteAllowed;
        policy.side_effect_rules[0].side_effect_class = SideEffectClass::None;
        policy
    };
    let capability = {
        let mut capability = valid_capability();
        capability.side_effect_class = SideEffectClass::None;
        capability.approval_posture = ApprovalPosture::ExecuteAllowed;
        capability
    };

    let actual = resolve(
        request,
        Ok(requester),
        Ok(policy),
        Ok(capability),
        decision_context("77777777-7777-4777-8777-777777777775"),
    );

    assert_eq!(
        actual,
        expected_decision("route-decision-execute-allowed-basic.json")
    );
}

#[test]
fn requester_policy_and_capability_denials_map_to_denied_posture() {
    let request = valid_request();
    let requester_denied = resolve(
        request.clone(),
        Err(deny(
            DenialReasonClass::UnknownRequester,
            DenialScope::Request,
            DenialBasis::RuntimeSafety,
            "unknown requester denied",
        )),
        Ok(valid_policy()),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888881"),
    );
    assert_eq!(
        requester_denied.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!requester_denied.execution_allowed);
    assert_eq!(
        requester_denied.denial_guards[0].reason_class,
        DenialReasonClass::UnknownRequester
    );

    let policy_denied = resolve(
        request.clone(),
        Ok(valid_requester()),
        Err(deny(
            DenialReasonClass::MissingPolicy,
            DenialScope::Artifact,
            DenialBasis::Policy,
            "missing policy artifact",
        )),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888882"),
    );
    assert_eq!(
        policy_denied.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!policy_denied.execution_allowed);
    assert_eq!(
        policy_denied.denial_guards[0].reason_class,
        DenialReasonClass::MissingPolicy
    );

    let capability_denied = resolve(
        request,
        Ok(valid_requester()),
        Ok(valid_policy()),
        Err(deny(
            DenialReasonClass::CapabilityNotAdmitted,
            DenialScope::Capability,
            DenialBasis::Policy,
            "unregistered capability",
        )),
        decision_context("88888888-8888-4888-8888-888888888883"),
    );
    assert_eq!(
        capability_denied.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!capability_denied.execution_allowed);
    assert_eq!(
        capability_denied.denial_guards[0].reason_class,
        DenialReasonClass::CapabilityNotAdmitted
    );
}

#[test]
fn review_required_and_explicit_operator_approval_remain_distinct() {
    let review_required = resolve(
        valid_request(),
        Ok(valid_requester()),
        Ok({
            let mut policy = valid_policy();
            policy.capability_rules[0].required_approval_posture = ApprovalPosture::ReviewRequired;
            policy
        }),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888884"),
    );

    let explicit_operator_approval = resolve(
        valid_request(),
        Ok(valid_requester()),
        Ok(valid_policy()),
        Ok({
            let mut capability = valid_capability();
            capability.review_class = ReviewClass::Operator;
            capability
        }),
        decision_context("88888888-8888-4888-8888-888888888885"),
    );

    assert_eq!(
        review_required.resolved_approval_posture,
        ApprovalPosture::ReviewRequired
    );
    assert!(review_required.review_required);
    assert!(!review_required.explicit_approval_required);

    assert_eq!(
        explicit_operator_approval.resolved_approval_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert!(!explicit_operator_approval.review_required);
    assert!(explicit_operator_approval.explicit_approval_required);
}

#[test]
fn invalid_inputs_cannot_yield_execute_allowed() {
    let requester_denied = resolve(
        valid_request(),
        Err(deny(
            DenialReasonClass::ContractInvalid,
            DenialScope::Request,
            DenialBasis::Contract,
            "malformed requester envelope",
        )),
        Ok(valid_policy()),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888886"),
    );
    assert_eq!(
        requester_denied.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!requester_denied.execution_allowed);

    let mut mismatched_request = valid_request();
    mismatched_request.requester_id =
        RequesterId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap());
    let mismatched = resolve(
        mismatched_request,
        Ok(valid_requester()),
        Ok(valid_policy()),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888887"),
    );
    assert_eq!(
        mismatched.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!mismatched.execution_allowed);

    let capability_denied = resolve(
        valid_request(),
        Ok(valid_requester()),
        Ok(valid_policy()),
        Err(deny(
            DenialReasonClass::DisabledByOperator,
            DenialScope::Capability,
            DenialBasis::Policy,
            "capability is disabled",
        )),
        decision_context("88888888-8888-4888-8888-888888888888"),
    );
    assert_eq!(
        capability_denied.resolved_approval_posture,
        ApprovalPosture::Denied
    );
    assert!(!capability_denied.execution_allowed);
}

#[test]
fn deny_to_posture_mapping_preserves_request_identity() {
    let request = valid_request();
    let denied = resolve(
        request.clone(),
        Err(deny(
            DenialReasonClass::UnknownRequester,
            DenialScope::Request,
            DenialBasis::RuntimeSafety,
            "unknown requester denied",
        )),
        Ok(valid_policy()),
        Ok(valid_capability()),
        decision_context("88888888-8888-4888-8888-888888888889"),
    );

    assert_eq!(denied.request_id, request.request_id);
    assert_eq!(denied.correlation_id, request.correlation_id);
    assert_eq!(denied.resolved_approval_posture, ApprovalPosture::Denied);
    assert_eq!(
        denied.operator_visible_summary,
        "request denied: unknown requester denied"
    );
}
