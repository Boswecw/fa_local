mod support;

use chrono::{TimeZone, Utc};

use fa_local::app::execution_service::{
    CoordinationContext, CoordinationDirective, CoordinationInput, ExecutionService,
};
use fa_local::domain::capabilities::CapabilityRegistryLoader;
use fa_local::domain::execution::{ExecutionPlan, ExecutionPlanValidator, ValidatedExecutionPlan};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{ApprovalPosture, ExecutionState};

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

fn context() -> CoordinationContext {
    CoordinationContext::new(
        ts(2030, 1, 1, 0, 10, 0),
        ts(2030, 1, 1, 0, 10, 5),
        ts(2030, 1, 1, 0, 10, 30),
    )
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

#[test]
fn denied_route_does_not_execute() {
    let input = CoordinationInput::new(
        route_decision("route-decision-denied-basic.json"),
        None,
        CoordinationDirective::NoExecution,
        context(),
    )
    .unwrap();

    let trace = ExecutionService.coordinate(input).unwrap();

    assert_eq!(trace.statuses.len(), 1);
    assert_eq!(trace.final_status().status.state, ExecutionState::Denied);
    assert_eq!(
        trace.final_status().status.current_posture,
        ApprovalPosture::Denied
    );
    assert!(trace.final_status().status.execution_plan_id.is_none());
}

#[test]
fn review_required_route_does_not_execute() {
    let input = CoordinationInput::new(
        route_decision("route-decision-review-required-basic.json"),
        None,
        CoordinationDirective::NoExecution,
        context(),
    )
    .unwrap();

    let trace = ExecutionService.coordinate(input).unwrap();

    assert_eq!(trace.statuses.len(), 1);
    assert_eq!(
        trace.final_status().status.state,
        ExecutionState::ReviewRequired
    );
    assert_eq!(
        trace.final_status().status.current_posture,
        ApprovalPosture::ReviewRequired
    );
    assert!(trace.final_status().status.execution_plan_id.is_none());
}

#[test]
fn explicit_approval_and_admitted_paths_are_handled_distinctly() {
    let plan = validated_plan();

    let explicit_input = CoordinationInput::new(
        route_decision("route-decision-explicit-operator-approval-basic.json"),
        Some(plan.clone()),
        CoordinationDirective::NoExecution,
        context(),
    )
    .unwrap();
    let explicit_trace = ExecutionService.coordinate(explicit_input).unwrap();

    assert_eq!(
        explicit_trace.final_status().status.state,
        ExecutionState::WaitingExplicitApproval
    );
    assert_eq!(
        explicit_trace.final_status().status.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert!(
        explicit_trace
            .final_status()
            .status
            .execution_plan_id
            .is_none()
    );

    let admitted_input = CoordinationInput::new(
        route_decision("route-decision-policy-preapproved-basic.json"),
        Some(plan),
        CoordinationDirective::NoExecution,
        context(),
    )
    .unwrap();
    let admitted_trace = ExecutionService.coordinate(admitted_input).unwrap();

    assert_eq!(
        admitted_trace.final_status().status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(
        admitted_trace.final_status().status.current_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert!(
        admitted_trace
            .final_status()
            .status
            .execution_plan_id
            .is_some()
    );
}

#[test]
fn admitted_route_progresses_through_declared_steps_only() {
    let plan = validated_plan();
    let declared_step_ids = plan
        .plan
        .steps
        .iter()
        .map(|step| step.step_id.clone())
        .collect::<Vec<_>>();

    let input = CoordinationInput::new(
        route_decision("route-decision-policy-preapproved-basic.json"),
        Some(plan),
        CoordinationDirective::CompleteDeclaredPlan,
        context(),
    )
    .unwrap();

    let trace = ExecutionService.coordinate(input).unwrap();
    let in_progress_steps = trace
        .statuses
        .iter()
        .filter(|status| status.status.state == ExecutionState::InProgress)
        .map(|status| status.status.current_step.clone().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        trace.statuses.first().unwrap().status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(in_progress_steps, declared_step_ids);
    assert_eq!(trace.final_status().status.state, ExecutionState::Completed);
    assert_eq!(
        trace.final_status().status.completion_summary.as_deref(),
        Some("execution completed for all declared plan steps")
    );
}

#[test]
fn coordinator_rejects_dynamic_step_invention() {
    let input = CoordinationInput::new(
        route_decision("route-decision-policy-preapproved-basic.json"),
        Some(validated_plan()),
        CoordinationDirective::FailAtDeclaredStep {
            step_id: "step_invented".to_owned(),
            failure_summary: "declared step failed".to_owned(),
        },
        context(),
    )
    .unwrap();

    let error = ExecutionService.coordinate(input).unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: execution coordinator step is not declared in execution plan"
    );
}

#[test]
fn unsupported_runtime_conditions_fail_closed() {
    let input = CoordinationInput::new(
        route_decision("route-decision-policy-preapproved-basic.json"),
        Some(validated_plan()),
        CoordinationDirective::UnsupportedRuntimeCondition {
            summary: "adapter invocation is not implemented".to_owned(),
        },
        context(),
    )
    .unwrap();

    let error = ExecutionService.coordinate(input).unwrap_err();
    assert_eq!(
        error.to_string(),
        "contract invalid: unsupported runtime condition: adapter invocation is not implemented"
    );
}

#[test]
fn cancel_path_stays_truthful_and_bounded() {
    let input = CoordinationInput::new(
        route_decision("route-decision-policy-preapproved-basic.json"),
        Some(validated_plan()),
        CoordinationDirective::CancelInFlight {
            step_id: "step_export_prepare".to_owned(),
        },
        context(),
    )
    .unwrap();

    let trace = ExecutionService.coordinate(input).unwrap();

    assert_eq!(trace.statuses.len(), 3);
    assert_eq!(
        trace.statuses[0].status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(trace.statuses[1].status.state, ExecutionState::InProgress);
    assert_eq!(
        trace.statuses[1].status.current_step.as_deref(),
        Some("step_export_prepare")
    );
    assert_eq!(trace.final_status().status.state, ExecutionState::Canceled);
    assert!(trace.final_status().status.failure_summary.is_none());
}
