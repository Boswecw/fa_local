mod support;

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::app::execution_service::{
    CoordinationContext, CoordinationDirective, CoordinationInput, ExecutionService,
};
use fa_local::app::review_service::{
    ReviewEmissionContext, ReviewEmissionInput, ReviewEmissionOutcome, ReviewNonEmissionReason,
    ReviewService,
};
use fa_local::domain::capabilities::CapabilityRegistryLoader;
use fa_local::domain::execution::{ExecutionPlan, ExecutionPlanValidator, ValidatedExecutionPlan};
use fa_local::domain::requester_trust::UserIntentBasis;
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{ApprovalPosture, ExecutionState, RequestId};

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

fn coordination_context() -> CoordinationContext {
    CoordinationContext::new(
        ts(2030, 1, 1, 0, 15, 0),
        ts(2030, 1, 1, 0, 15, 5),
        ts(2030, 1, 1, 0, 15, 30),
    )
}

fn emission_context() -> ReviewEmissionContext {
    ReviewEmissionContext::new(ts(2030, 1, 1, 0, 15, 45))
}

fn validated_plan() -> ValidatedExecutionPlan {
    let registry = CapabilityRegistryLoader::load_contract_value(&support::load_fixture_json(
        "valid",
        "capability-registry-basic.json",
    ))
    .unwrap();
    let plan = ExecutionPlan::load_contract_value(&support::load_fixture_json(
        "valid",
        "execution-plan-basic.json",
    ))
    .unwrap();

    ExecutionPlanValidator::validate(&plan, &registry).unwrap()
}

fn route_decision(file_name: &str) -> RouteDecision {
    RouteDecisionLoader::load_contract_value(&support::load_fixture_json("valid", file_name))
        .unwrap()
}

fn base_input(
    route_decision: RouteDecision,
    validated_plan: Option<ValidatedExecutionPlan>,
    execution_status: Option<fa_local::domain::status::ValidatedExecutionStatus>,
) -> ReviewEmissionInput {
    ReviewEmissionInput::new(
        route_decision,
        validated_plan,
        execution_status,
        UserIntentBasis::ExplicitUserAction,
        "trusted app surface requested a governed export write".to_owned(),
        "bounded two-step local export plan using admitted capability only".to_owned(),
        "local_file_write only; no external network or process spawn".to_owned(),
        vec![
            fa_local::domain::review::ApprovalOption::ApproveExecute,
            fa_local::domain::review::ApprovalOption::DeclineRequest,
        ],
        "request remains denied until a new governed approval path exists".to_owned(),
        emission_context(),
    )
    .unwrap()
}

#[test]
fn emits_review_package_for_coherent_explicit_approval_path() {
    let route = route_decision("route-decision-explicit-operator-approval-basic.json");
    let plan = validated_plan();
    let waiting_trace = ExecutionService
        .coordinate(
            CoordinationInput::new(
                route.clone(),
                Some(plan.clone()),
                CoordinationDirective::NoExecution,
                coordination_context(),
            )
            .unwrap(),
        )
        .unwrap();
    let waiting_status = waiting_trace.final_status().clone();

    let outcome = ReviewService
        .emit_review_package(base_input(route, Some(plan.clone()), Some(waiting_status)))
        .unwrap();

    let ReviewEmissionOutcome::Emitted(package) = outcome else {
        panic!("expected explicit approval path to emit review package");
    };

    assert_eq!(
        package.package.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(
        package.package.execution_plan_id,
        Some(plan.plan.execution_plan_id)
    );
    assert_eq!(
        package.package.stable_plan_hash,
        Some(plan.stable_plan_hash.clone())
    );
    assert_eq!(
        package
            .package
            .execution_status_context
            .as_ref()
            .unwrap()
            .state,
        ExecutionState::WaitingExplicitApproval
    );
    assert_eq!(
        package
            .package
            .execution_status_context
            .as_ref()
            .unwrap()
            .truthful_user_visible_summary,
        "waiting for explicit operator approval"
    );
}

#[test]
fn emits_review_package_for_coherent_review_required_path() {
    let route = route_decision("route-decision-review-required-basic.json");
    let review_trace = ExecutionService
        .coordinate(
            CoordinationInput::new(
                route.clone(),
                None,
                CoordinationDirective::NoExecution,
                coordination_context(),
            )
            .unwrap(),
        )
        .unwrap();
    let review_status = review_trace.final_status().clone();

    let outcome = ReviewService
        .emit_review_package(base_input(route, None, Some(review_status)))
        .unwrap();

    let ReviewEmissionOutcome::Emitted(package) = outcome else {
        panic!("expected review_required path to emit review package");
    };

    assert_eq!(
        package.package.current_posture,
        ApprovalPosture::ReviewRequired
    );
    assert!(package.package.execution_plan_id.is_none());
    assert!(package.package.stable_plan_hash.is_none());
    assert_eq!(
        package
            .package
            .execution_status_context
            .as_ref()
            .unwrap()
            .state,
        ExecutionState::ReviewRequired
    );
}

#[test]
fn non_review_path_does_not_emit_review_package() {
    let outcome = ReviewService
        .emit_review_package(base_input(
            route_decision("route-decision-execute-allowed-basic.json"),
            Some(validated_plan()),
            None,
        ))
        .unwrap();

    assert_eq!(
        outcome,
        ReviewEmissionOutcome::NotEmitted(ReviewNonEmissionReason::NonReviewPath)
    );
}

#[test]
fn denied_path_does_not_emit_review_package() {
    let outcome = ReviewService
        .emit_review_package(base_input(
            route_decision("route-decision-denied-basic.json"),
            None,
            None,
        ))
        .unwrap();

    assert_eq!(
        outcome,
        ReviewEmissionOutcome::NotEmitted(ReviewNonEmissionReason::NonReviewPath)
    );
}

#[test]
fn rejects_missing_or_inconsistent_emitter_inputs() {
    let error = ReviewEmissionInput::new(
        route_decision("route-decision-explicit-operator-approval-basic.json"),
        None,
        None,
        UserIntentBasis::ExplicitUserAction,
        "trusted requester".to_owned(),
        "bounded execution summary".to_owned(),
        "local_file_write only".to_owned(),
        vec![
            fa_local::domain::review::ApprovalOption::ApproveExecute,
            fa_local::domain::review::ApprovalOption::DeclineRequest,
        ],
        "request remains denied".to_owned(),
        emission_context(),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: explicit approval review-package emission requires validated execution plan"
    );

    let mut mismatched_plan = validated_plan();
    mismatched_plan.plan.originating_request_id =
        RequestId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap());

    let mismatch_error = ReviewEmissionInput::new(
        route_decision("route-decision-explicit-operator-approval-basic.json"),
        Some(mismatched_plan),
        None,
        UserIntentBasis::ExplicitUserAction,
        "trusted requester".to_owned(),
        "bounded execution summary".to_owned(),
        "local_file_write only".to_owned(),
        vec![
            fa_local::domain::review::ApprovalOption::ApproveExecute,
            fa_local::domain::review::ApprovalOption::DeclineRequest,
        ],
        "request remains denied".to_owned(),
        emission_context(),
    )
    .unwrap_err();

    assert_eq!(
        mismatch_error.to_string(),
        "contract invalid: review-package emission plan request_id does not match route decision"
    );

    let review_required_error = ReviewEmissionInput::new(
        route_decision("route-decision-review-required-basic.json"),
        Some(validated_plan()),
        None,
        UserIntentBasis::ExplicitUserAction,
        "trusted requester".to_owned(),
        "bounded execution summary".to_owned(),
        "local_file_write only".to_owned(),
        vec![
            fa_local::domain::review::ApprovalOption::ApproveExecute,
            fa_local::domain::review::ApprovalOption::DeclineRequest,
        ],
        "request remains denied".to_owned(),
        emission_context(),
    )
    .unwrap_err();

    assert_eq!(
        review_required_error.to_string(),
        "contract invalid: review_required review-package emission path must not include validated execution plan"
    );
}

#[test]
fn fabricated_execution_success_context_is_not_emitted() {
    let explicit_route = route_decision("route-decision-explicit-operator-approval-basic.json");
    let explicit_plan = validated_plan();
    let completed_trace = ExecutionService
        .coordinate(
            CoordinationInput::new(
                route_decision("route-decision-policy-preapproved-basic.json"),
                Some(validated_plan()),
                CoordinationDirective::CompleteDeclaredPlan,
                coordination_context(),
            )
            .unwrap(),
        )
        .unwrap();
    let completed_status = completed_trace.final_status().clone();

    let error = ReviewEmissionInput::new(
        explicit_route,
        Some(explicit_plan),
        Some(completed_status),
        UserIntentBasis::ExplicitUserAction,
        "trusted requester".to_owned(),
        "bounded execution summary".to_owned(),
        "local_file_write only".to_owned(),
        vec![
            fa_local::domain::review::ApprovalOption::ApproveExecute,
            fa_local::domain::review::ApprovalOption::DeclineRequest,
        ],
        "request remains denied".to_owned(),
        emission_context(),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: review-package emission status posture does not match route decision"
    );
}
