mod support;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::domain::requester_trust::UserIntentBasis;
use fa_local::domain::review::{
    ApprovalOption, ReviewExecutionStatusContext, ReviewPackage, ValidatedReviewPackage,
};
use fa_local::{
    ApprovalPosture, ExecutionPlanId, ExecutionState, RequestId, ReviewPackageId, RouteDecisionId,
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

fn base_review_package() -> ReviewPackage {
    let value = support::load_fixture_json("valid", "review-package-basic.json");
    ReviewPackage::load_contract_value(&value).unwrap()
}

fn base_review_required_package() -> ReviewPackage {
    let value = support::load_fixture_json("valid", "review-package-review-required-basic.json");
    ReviewPackage::load_contract_value(&value).unwrap()
}

#[test]
fn valid_review_package_fixture_loads_and_validates() {
    let package = base_review_package();
    package.validate().unwrap();
    let validated = ValidatedReviewPackage::new(package).unwrap();

    assert_eq!(
        validated.package.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
}

#[test]
fn valid_review_required_review_package_fixture_loads_and_validates() {
    let package = base_review_required_package();
    package.validate().unwrap();
    let validated = ValidatedReviewPackage::new(package).unwrap();

    assert_eq!(
        validated.package.current_posture,
        ApprovalPosture::ReviewRequired
    );
    assert!(validated.package.execution_plan_id.is_none());
    assert!(validated.package.stable_plan_hash.is_none());
}

#[test]
fn review_status_context_helper_rejects_non_waiting_state() {
    let error = ReviewExecutionStatusContext::new(
        ExecutionState::Completed,
        None,
        ts(2030, 1, 1, 0, 9, 30),
        "execution completed".to_owned(),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: review package execution_status_context must remain review_required or waiting_explicit_approval"
    );
}

#[test]
fn review_package_preserves_posture_state_distinction() {
    let package = base_review_package();
    let context = package.execution_status_context.as_ref().unwrap();

    assert_eq!(
        package.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(context.state, ExecutionState::WaitingExplicitApproval);

    let review_required = base_review_required_package();
    let review_context = review_required.execution_status_context.as_ref().unwrap();
    assert_eq!(
        review_required.current_posture,
        ApprovalPosture::ReviewRequired
    );
    assert_eq!(review_context.state, ExecutionState::ReviewRequired);

    let mut invalid = base_review_package();
    invalid.current_posture = ApprovalPosture::PolicyPreapproved;
    let error = invalid.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review package must preserve review_required or explicit_operator_approval posture"
    );
}

#[test]
fn review_package_cannot_fabricate_execution_success() {
    let mut package = base_review_package();
    package.execution_status_context = Some(ReviewExecutionStatusContext {
        state: ExecutionState::Completed,
        degraded_subtype: None,
        updated_at_utc: ts(2030, 1, 1, 0, 9, 30),
        truthful_user_visible_summary: "execution completed".to_owned(),
    });

    let error = package.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review package execution_status_context must remain review_required or waiting_explicit_approval"
    );
}

#[test]
fn review_package_requires_decline_option() {
    let mut package = base_review_package();
    package.approval_options_allowed_by_policy = vec![ApprovalOption::ApproveExecute];

    let error = package.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review package must include decline_request as an allowed approval option"
    );
}

#[test]
fn review_package_requires_explicit_degraded_or_fallback_posture_when_narrated() {
    let mut package = base_review_package();
    package.proposed_execution_summary =
        "bounded execution may use a fallback path if explicitly approved".to_owned();
    package.degraded_or_fallback_posture = None;

    let error = package.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review package cannot narrate degraded or fallback posture without explicit degraded_or_fallback_posture"
    );
}

#[test]
fn review_package_fallback_context_fixture_validates() {
    let value = support::load_fixture_json("valid", "review-package-fallback-context-basic.json");
    let package = ReviewPackage::load_contract_value(&value).unwrap();
    package.validate().unwrap();

    assert!(package.execution_status_context.is_none());
    assert!(package.degraded_or_fallback_posture.is_some());
}

#[test]
fn review_required_review_package_must_not_carry_plan_linkage() {
    let mut package = base_review_required_package();
    package.execution_plan_id = Some(ExecutionPlanId::from_uuid(
        Uuid::parse_str("99999999-9999-4999-8999-999999999999").unwrap(),
    ));
    package.stable_plan_hash =
        Some("a8f7e75a4a4ea309803cf16fec43a4adb788ddcaa5589d91a9c00a41a8f56df7".to_owned());

    let error = package.validate().unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: review_required review package must not include execution_plan_id or stable_plan_hash"
    );
}

#[test]
fn review_package_constructor_validates_inputs() {
    let error = ReviewPackage::new(
        ReviewPackageId::from_uuid(
            Uuid::parse_str("edededed-eded-4ded-8ded-edededededed").unwrap(),
        ),
        RequestId::from_uuid(Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap()),
        fa_local::CorrelationId::from_uuid(
            Uuid::parse_str("66666666-6666-4666-8666-666666666666").unwrap(),
        ),
        RouteDecisionId::from_uuid(
            Uuid::parse_str("77777777-7777-4777-8777-777777777773").unwrap(),
        ),
        Some(ExecutionPlanId::from_uuid(
            Uuid::parse_str("99999999-9999-4999-8999-999999999999").unwrap(),
        )),
        Some("bad-hash".to_owned()),
        ApprovalPosture::ExplicitOperatorApproval,
        None,
        UserIntentBasis::ExplicitUserAction,
        "trusted requester".to_owned(),
        "bounded execution summary".to_owned(),
        "local_file_write only".to_owned(),
        None,
        vec![
            ApprovalOption::ApproveExecute,
            ApprovalOption::DeclineRequest,
        ],
        "request remains denied".to_owned(),
        ts(2030, 1, 1, 0, 9, 30),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: review package stable_plan_hash must be a 64-character lowercase hex digest"
    );
}
