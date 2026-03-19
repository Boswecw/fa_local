mod support;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::domain::friction::{
    FrictionKind, FrictionPayload, OperatorAction, ValidatedFrictionPayload,
};
use fa_local::{
    ApprovalPosture, CorrelationId, DegradedSubtype, ExecutionPlanId, ExecutionState,
    FrictionPayloadId, RequestId, ReviewPackageId,
};

fn ts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .unwrap()
}

fn base_denial_payload() -> FrictionPayload {
    let value = support::load_fixture_json("valid", "friction-payload-denial-basic.json");
    FrictionPayload::load_contract_value(&value).unwrap()
}

fn base_explicit_approval_payload() -> FrictionPayload {
    let value =
        support::load_fixture_json("valid", "friction-payload-explicit-approval-basic.json");
    FrictionPayload::load_contract_value(&value).unwrap()
}

fn base_execution_constraint_payload() -> FrictionPayload {
    let value =
        support::load_fixture_json("valid", "friction-payload-execution-constraint-basic.json");
    FrictionPayload::load_contract_value(&value).unwrap()
}

#[test]
fn valid_friction_payload_fixtures_load_and_validate() {
    let denial = base_denial_payload();
    denial.validate().unwrap();

    let review = FrictionPayload::load_contract_value(&support::load_fixture_json(
        "valid",
        "friction-payload-review-required-basic.json",
    ))
    .unwrap();
    review.validate().unwrap();

    let explicit = base_explicit_approval_payload();
    explicit.validate().unwrap();

    let constrained = base_execution_constraint_payload();
    let validated = ValidatedFrictionPayload::new(constrained).unwrap();

    assert_eq!(
        validated.payload.friction_kind,
        FrictionKind::ExecutionConstraint
    );
}

#[test]
fn friction_payload_preserves_posture_and_state_distinction() {
    let explicit = base_explicit_approval_payload();
    assert_eq!(
        explicit.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(
        explicit.execution_state,
        ExecutionState::WaitingExplicitApproval
    );

    let constrained = base_execution_constraint_payload();
    assert_eq!(
        constrained.current_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert_eq!(
        constrained.execution_state,
        ExecutionState::CompletedWithConstraints
    );
}

#[test]
fn denial_friction_requires_denial_guards() {
    let mut payload = base_denial_payload();
    payload.denial_guards.clear();

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: denial friction payload must include at least one denial guard"
    );
}

#[test]
fn explicit_approval_friction_keeps_review_surface_distinct() {
    let payload = base_explicit_approval_payload();

    assert_eq!(
        payload.friction_kind,
        FrictionKind::ExplicitApprovalRequired
    );
    assert_eq!(payload.denial_guards.len(), 0);
    assert!(payload.review_package_id.is_some());
    assert!(payload.execution_plan_id.is_some());
    assert!(payload.stable_plan_hash.is_some());
}

#[test]
fn friction_payload_rejects_planner_or_workflow_narration() {
    let mut payload = base_execution_constraint_payload();
    payload.operator_visible_summary = "planner selected the next workflow step".to_owned();

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: friction payload summary must not narrate planner, workflow, or semantic interpretation"
    );
}

#[test]
fn friction_payload_requires_explicit_fallback_subtype_when_narrated() {
    let mut payload = base_execution_constraint_payload();
    payload.operator_visible_summary =
        "execution completed after fallback to a constrained local export path".to_owned();
    payload.degraded_subtype = None;

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: completed_with_constraints friction payload must include an explicit fallback degraded_subtype"
    );
}

#[test]
fn execution_constraint_friction_cannot_carry_review_package_reference() {
    let mut payload = base_execution_constraint_payload();
    payload.review_package_id = Some(ReviewPackageId::from_uuid(
        Uuid::parse_str("abababab-abab-4bab-8bab-abababababab").unwrap(),
    ));

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: execution_constraint friction payload must not include review_package_id"
    );
}

#[test]
fn friction_payload_constructor_rejects_non_minimized_payloads() {
    let denial = base_denial_payload();
    let error = FrictionPayload::new(
        FrictionPayloadId::from_uuid(
            Uuid::parse_str("d5d5d5d5-d5d5-4dd5-8dd5-d5d5d5d5d5d5").unwrap(),
        ),
        CorrelationId::from_uuid(Uuid::parse_str("66666666-6666-4666-8666-666666666666").unwrap()),
        RequestId::from_uuid(Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap()),
        FrictionKind::Denial,
        OperatorAction::Stop,
        None,
        None,
        None,
        None,
        denial.forensic_event_id,
        ApprovalPosture::Denied,
        ExecutionState::Denied,
        None,
        denial.denial_guards.clone(),
        "request denied due to missing trust basis".to_owned(),
        false,
        ts(2030, 1, 1, 0, 8, 31),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: friction payload payload_minimized must remain true for bounded friction"
    );
}

#[test]
fn explicit_approval_friction_requires_review_package_reference() {
    let mut payload = base_explicit_approval_payload();
    payload.review_package_id = None;

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: explicit_approval_required friction payload must include review_package_id"
    );
}

#[test]
fn review_required_friction_stays_distinct_from_execution_status_surface() {
    let mut payload = FrictionPayload::load_contract_value(&support::load_fixture_json(
        "valid",
        "friction-payload-review-required-basic.json",
    ))
    .unwrap();
    payload.execution_plan_id = Some(ExecutionPlanId::from_uuid(
        Uuid::parse_str("99999999-9999-4999-8999-999999999999").unwrap(),
    ));
    payload.stable_plan_hash =
        Some("a8f7e75a4a4ea309803cf16fec43a4adb788ddcaa5589d91a9c00a41a8f56df7".to_owned());

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review_required friction payload must not include execution_plan_id"
    );
}

#[test]
fn completed_with_constraints_requires_explicit_fallback_subtype() {
    let mut payload = base_execution_constraint_payload();
    payload.degraded_subtype = Some(DegradedSubtype::DegradedInFlight);

    let error = payload.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: completed_with_constraints friction payload must include an explicit fallback degraded_subtype"
    );
}
