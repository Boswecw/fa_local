mod support;

use uuid::Uuid;

use fa_local::app::execution_service::{
    CoordinationContext, CoordinationDirective, CoordinationInput, ExecutionService,
};
use fa_local::app::routing_service::{RoutePathKind, RoutingInput, RoutingService};
use fa_local::domain::capabilities::CapabilityRegistryLoader;
use fa_local::domain::execution::{ExecutionPlan, ExecutionPlanValidator, ValidatedExecutionPlan};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{ApprovalPosture, CapabilityId, ExecutionState};

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

fn coordination_context() -> CoordinationContext {
    CoordinationContext::default()
}

#[test]
fn denied_and_review_required_routes_are_not_executable() {
    let denied = RoutingService
        .select_route(
            RoutingInput::new(route_decision("route-decision-denied-basic.json"), None).unwrap(),
        )
        .unwrap();
    assert_eq!(denied.route_path_kind, RoutePathKind::NonExecutableDenied);
    assert!(!denied.executable);
    assert_eq!(denied.resolved_approval_posture, ApprovalPosture::Denied);
    assert!(denied.execution_plan_id.is_none());

    let review = RoutingService
        .select_route(
            RoutingInput::new(
                route_decision("route-decision-review-required-basic.json"),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        review.route_path_kind,
        RoutePathKind::NonExecutableReviewRequired
    );
    assert!(!review.executable);
    assert_eq!(
        review.resolved_approval_posture,
        ApprovalPosture::ReviewRequired
    );
    assert!(review.execution_plan_id.is_none());
}

#[test]
fn explicit_approval_and_execute_allowed_paths_remain_distinct() {
    let plan = validated_plan();

    let explicit = RoutingService
        .select_route(
            RoutingInput::new(
                route_decision("route-decision-explicit-operator-approval-basic.json"),
                Some(plan.clone()),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        explicit.route_path_kind,
        RoutePathKind::AwaitExplicitApproval
    );
    assert!(!explicit.executable);
    assert!(explicit.explicit_approval_required);
    assert_eq!(
        explicit.resolved_approval_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );

    let execute_allowed = RoutingService
        .select_route(
            RoutingInput::new(
                route_decision("route-decision-execute-allowed-basic.json"),
                Some(plan),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        execute_allowed.route_path_kind,
        RoutePathKind::ExternalAdapterBoundedExecution
    );
    assert!(execute_allowed.executable);
    assert!(!execute_allowed.explicit_approval_required);
    assert_eq!(
        execute_allowed.resolved_approval_posture,
        ApprovalPosture::ExecuteAllowed
    );
}

#[test]
fn routing_does_not_invent_capabilities_or_fallbacks() {
    let plan = validated_plan();
    let selected = RoutingService
        .select_route(
            RoutingInput::new(
                route_decision("route-decision-policy-preapproved-basic.json"),
                Some(plan.clone()),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(
        selected.declared_capability_ids,
        plan.plan.referenced_capabilities
    );
    assert_eq!(
        selected.declared_step_ids,
        plan.plan
            .steps
            .iter()
            .map(|step| step.step_id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        selected.declared_fallback_references,
        plan.plan.fallback_references
    );
}

#[test]
fn ambiguous_or_unsupported_route_conditions_fail_closed() {
    let mut bad_route = route_decision("route-decision-execute-allowed-basic.json");
    bad_route
        .capability_decision_summary
        .requested_capability_id =
        CapabilityId::from_uuid(Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap());

    let error = RoutingInput::new(bad_route, Some(validated_plan())).unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: routing plan does not include route decision requested capability"
    );
}

#[test]
fn coordinator_and_routing_boundaries_remain_separate() {
    let route_decision = route_decision("route-decision-explicit-operator-approval-basic.json");
    let plan = validated_plan();

    let selected = RoutingService
        .select_route(RoutingInput::new(route_decision.clone(), Some(plan.clone())).unwrap())
        .unwrap();
    assert_eq!(
        selected.route_path_kind,
        RoutePathKind::AwaitExplicitApproval
    );
    assert!(!selected.executable);
    assert!(selected.explicit_approval_required);

    let trace = ExecutionService
        .coordinate(
            CoordinationInput::new(
                route_decision,
                Some(plan),
                CoordinationDirective::NoExecution,
                coordination_context(),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(
        trace.final_status().status.state,
        ExecutionState::WaitingExplicitApproval
    );
    assert_eq!(
        trace.final_status().status.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
}
