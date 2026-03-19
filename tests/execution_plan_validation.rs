mod support;

use uuid::Uuid;

use fa_local::domain::capabilities::{
    CapabilityRecord, CapabilityRegistry, CapabilityRegistryLoader, EnabledState,
};
use fa_local::domain::execution::{
    CompletionPolicy, ExecutionPlan, ExecutionPlanValidator, FallbackReference,
};
use fa_local::{CapabilityId, DenialReasonClass, RevocationState, SideEffectClass};

fn valid_registry() -> CapabilityRegistry {
    let value = support::load_fixture_json("valid", "capability-registry-basic.json");
    CapabilityRegistryLoader::load_contract_value(&value).unwrap()
}

fn valid_plan() -> ExecutionPlan {
    let value = support::load_fixture_json("valid", "execution-plan-basic.json");
    ExecutionPlan::load_contract_value(&value).unwrap()
}

fn secondary_capability() -> CapabilityRecord {
    let mut capability = valid_registry().capabilities.into_iter().next().unwrap();
    capability.capability_id =
        CapabilityId::from_uuid(Uuid::parse_str("45454545-4545-4545-8545-454545454545").unwrap());
    capability
}

#[test]
fn valid_execution_plan_fixture_hash_matches_canonical_hash() {
    let plan = valid_plan();
    let expected = ExecutionPlanValidator::compute_stable_plan_hash(&plan);

    assert_eq!(plan.stable_plan_hash, expected);
}

#[test]
fn validates_bounded_multi_step_execution_plan() {
    let plan = valid_plan();
    let registry = valid_registry();
    let validated = ExecutionPlanValidator::validate(&plan, &registry).unwrap();

    assert_eq!(validated.plan.steps.len(), 2);
    assert_eq!(validated.stable_plan_hash, plan.stable_plan_hash);
}

#[test]
fn rejects_plan_that_exceeds_declared_max_step_count() {
    let mut plan = valid_plan();
    let registry = valid_registry();
    plan.declared_max_step_count = 1;

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(
        error.summary,
        "execution plan exceeds declared max step count"
    );
}

#[test]
fn rejects_undeclared_fallback_reference() {
    let mut plan = valid_plan();
    let registry = valid_registry();
    plan.completion_policy = CompletionPolicy::AllowDeclaredFallbackCompletion;
    plan.fallback_references = vec![FallbackReference {
        step_id: "step_export_prepare".to_owned(),
        fallback_step_id: "missing_step".to_owned(),
    }];
    plan.stable_plan_hash = ExecutionPlanValidator::compute_stable_plan_hash(&plan);

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(
        error.summary,
        "execution plan fallback references undeclared fallback step"
    );
}

#[test]
fn rejects_unregistered_capability_reference() {
    let mut plan = valid_plan();
    let registry = valid_registry();
    let unknown_capability =
        CapabilityId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap());
    plan.steps[0].capability_id = unknown_capability;
    plan.referenced_capabilities = vec![unknown_capability];
    plan.stable_plan_hash = ExecutionPlanValidator::compute_stable_plan_hash(&plan);

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::CapabilityNotAdmitted);
    assert_eq!(
        error.summary,
        "execution plan references unregistered capability"
    );
}

#[test]
fn rejects_disabled_capability_reference() {
    let plan = valid_plan();
    let mut registry = valid_registry();
    registry.capabilities[0].enabled_state = EnabledState::Disabled;

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::DisabledByOperator);
    assert_eq!(
        error.summary,
        "execution plan references disabled capability"
    );
}

#[test]
fn rejects_revoked_capability_reference() {
    let plan = valid_plan();
    let mut registry = valid_registry();
    registry.capabilities[0].revocation_state = RevocationState::Revoked;

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::CapabilityNotAdmitted);
    assert_eq!(
        error.summary,
        "execution plan references revoked capability"
    );
}

#[test]
fn stable_plan_hash_is_deterministic_for_same_logical_plan() {
    let mut left = valid_plan();
    let mut right = valid_plan();

    left.execution_plan_id = fa_local::ExecutionPlanId::from_uuid(
        Uuid::parse_str("11111111-aaaa-4111-8111-111111111111").unwrap(),
    );
    left.correlation_id = fa_local::CorrelationId::from_uuid(
        Uuid::parse_str("22222222-bbbb-4222-8222-222222222222").unwrap(),
    );
    left.originating_request_id = fa_local::RequestId::from_uuid(
        Uuid::parse_str("33333333-cccc-4333-8333-333333333333").unwrap(),
    );
    left.planned_at_utc = chrono::DateTime::parse_from_rfc3339("2030-01-01T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    right.stable_plan_hash =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();

    assert_eq!(
        ExecutionPlanValidator::compute_stable_plan_hash(&left),
        ExecutionPlanValidator::compute_stable_plan_hash(&right)
    );
}

#[test]
fn changed_step_order_yields_different_hash() {
    let original = valid_plan();
    let mut reordered = valid_plan();
    reordered.steps.swap(0, 1);

    assert_ne!(
        ExecutionPlanValidator::compute_stable_plan_hash(&original),
        ExecutionPlanValidator::compute_stable_plan_hash(&reordered)
    );
}

#[test]
fn changed_fallback_or_capability_set_yields_different_hash() {
    let original = valid_plan();

    let mut changed_fallback = valid_plan();
    changed_fallback.completion_policy = CompletionPolicy::AllowDeclaredFallbackCompletion;
    changed_fallback.fallback_references = vec![FallbackReference {
        step_id: "step_export_prepare".to_owned(),
        fallback_step_id: "step_export_commit".to_owned(),
    }];

    let mut changed_capability_set = valid_plan();
    changed_capability_set
        .referenced_capabilities
        .push(secondary_capability().capability_id);

    assert_ne!(
        ExecutionPlanValidator::compute_stable_plan_hash(&original),
        ExecutionPlanValidator::compute_stable_plan_hash(&changed_fallback)
    );
    assert_ne!(
        ExecutionPlanValidator::compute_stable_plan_hash(&original),
        ExecutionPlanValidator::compute_stable_plan_hash(&changed_capability_set)
    );
}

#[test]
fn invalid_plans_cannot_produce_admitted_validated_plan() {
    let registry = valid_registry();

    let mut bad_hash = valid_plan();
    bad_hash.stable_plan_hash =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned();
    assert!(ExecutionPlanValidator::validate(&bad_hash, &registry).is_err());

    let mut bad_fallback = valid_plan();
    bad_fallback.completion_policy = CompletionPolicy::AllowDeclaredFallbackCompletion;
    bad_fallback.fallback_references = vec![FallbackReference {
        step_id: "step_export_prepare".to_owned(),
        fallback_step_id: "missing_step".to_owned(),
    }];
    bad_fallback.stable_plan_hash = ExecutionPlanValidator::compute_stable_plan_hash(&bad_fallback);
    assert!(ExecutionPlanValidator::validate(&bad_fallback, &registry).is_err());

    let mut bad_capability = valid_plan();
    let unknown_capability =
        CapabilityId::from_uuid(Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap());
    bad_capability.steps[0].capability_id = unknown_capability;
    bad_capability.referenced_capabilities = vec![unknown_capability];
    bad_capability.stable_plan_hash =
        ExecutionPlanValidator::compute_stable_plan_hash(&bad_capability);
    assert!(ExecutionPlanValidator::validate(&bad_capability, &registry).is_err());
}

#[test]
fn capability_set_reordering_does_not_change_hash_for_same_logical_plan() {
    let mut base = valid_plan();
    let extra_capability = secondary_capability().capability_id;
    base.referenced_capabilities.push(extra_capability);

    let mut reordered = base.clone();
    reordered.referenced_capabilities.reverse();

    assert_eq!(
        ExecutionPlanValidator::compute_stable_plan_hash(&base),
        ExecutionPlanValidator::compute_stable_plan_hash(&reordered)
    );
}

#[test]
fn declared_side_effect_class_must_cover_each_step() {
    let mut plan = valid_plan();
    let registry = valid_registry();
    plan.declared_side_effect_classes = vec![SideEffectClass::LocalDbMutation];
    plan.stable_plan_hash = ExecutionPlanValidator::compute_stable_plan_hash(&plan);

    let error = ExecutionPlanValidator::validate(&plan, &registry).unwrap_err();
    assert_eq!(error.reason_class, DenialReasonClass::ContractInvalid);
    assert_eq!(
        error.summary,
        "execution plan step uses undeclared side effect class"
    );
}
