mod support;

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{TimeZone, Utc};

use fa_local::adapters::execution_delivery::{
    AdapterDeliveryRequest, AdapterDeliveryResult, ExternalRouteDeliveryAdapter,
};
use fa_local::app::execution_service::{CoordinationContext, ExecutionService};
use fa_local::app::routing_service::{
    RoutePathKind, RoutingInput, RoutingService, SelectedExecutionRoute,
};
use fa_local::domain::capabilities::CapabilityRegistryLoader;
use fa_local::domain::execution::{ExecutionPlan, ExecutionPlanValidator, ValidatedExecutionPlan};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{ApprovalPosture, DegradedSubtype, ExecutionState};

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
        ts(2030, 1, 1, 0, 20, 0),
        ts(2030, 1, 1, 0, 20, 5),
        ts(2030, 1, 1, 0, 20, 30),
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

fn selected_route(
    file_name: &str,
    validated_plan: Option<ValidatedExecutionPlan>,
) -> SelectedExecutionRoute {
    RoutingService
        .select_route(RoutingInput::new(route_decision(file_name), validated_plan).unwrap())
        .unwrap()
}

#[derive(Debug)]
struct StubAdapter {
    result: AdapterDeliveryResult,
    calls: AtomicUsize,
    last_request: Mutex<Option<AdapterDeliveryRequest>>,
}

impl StubAdapter {
    fn new(result: AdapterDeliveryResult) -> Self {
        Self {
            result,
            calls: AtomicUsize::new(0),
            last_request: Mutex::new(None),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn last_request(&self) -> Option<AdapterDeliveryRequest> {
        self.last_request.lock().unwrap().clone()
    }
}

impl ExternalRouteDeliveryAdapter for StubAdapter {
    fn adapter_id(&self) -> &'static str {
        "stub-external-adapter"
    }

    fn deliver_route(&self, request: &AdapterDeliveryRequest) -> AdapterDeliveryResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last_request.lock().unwrap() = Some(request.clone());
        self.result.clone()
    }
}

#[test]
fn denied_and_review_required_routes_never_invoke_adapters() {
    let denied_route = selected_route("route-decision-denied-basic.json", None);
    let denied_adapter = StubAdapter::new(AdapterDeliveryResult::DeliveredAllSteps);
    let denied_error = ExecutionService
        .deliver_selected_route(&denied_route, &denied_adapter, context())
        .unwrap_err();
    assert_eq!(
        denied_error.to_string(),
        "contract invalid: non-executable route must not reach adapter delivery"
    );
    assert_eq!(denied_adapter.calls(), 0);

    let review_route = selected_route("route-decision-review-required-basic.json", None);
    let review_adapter = StubAdapter::new(AdapterDeliveryResult::DeliveredAllSteps);
    let review_error = ExecutionService
        .deliver_selected_route(&review_route, &review_adapter, context())
        .unwrap_err();
    assert_eq!(
        review_error.to_string(),
        "contract invalid: non-executable route must not reach adapter delivery"
    );
    assert_eq!(review_adapter.calls(), 0);
}

#[test]
fn explicit_approval_route_never_reaches_adapter_delivery() {
    let route = selected_route(
        "route-decision-explicit-operator-approval-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::DeliveredAllSteps);

    let error = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap_err();

    assert_eq!(route.route_path_kind, RoutePathKind::AwaitExplicitApproval);
    assert!(!route.executable);
    assert!(route.explicit_approval_required);
    assert_eq!(
        route.resolved_approval_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(
        error.to_string(),
        "contract invalid: explicit approval route must not reach adapter delivery"
    );
    assert_eq!(adapter.calls(), 0);
}

#[test]
fn admitted_routed_work_reaches_adapter_and_maps_to_completed_trace() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::DeliveredAllSteps);

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    let delivered_request = adapter.last_request().unwrap();
    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        route.route_path_kind,
        RoutePathKind::ExternalAdapterBoundedExecution
    );
    assert!(route.executable);
    assert_eq!(delivered_request.declared_step_ids, route.declared_step_ids);
    assert_eq!(
        delivered_request.declared_capability_ids,
        route.declared_capability_ids
    );
    assert_eq!(
        delivered_request.declared_fallback_references,
        route.declared_fallback_references
    );
    assert_eq!(
        delivered_request.requested_capability_id,
        route.requested_capability_id
    );

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
    assert_eq!(in_progress_steps, route.declared_step_ids);
    assert_eq!(trace.final_status().status.state, ExecutionState::Completed);
    assert_eq!(
        trace.final_status().status.current_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert_eq!(
        trace.final_status().status.completion_summary.as_deref(),
        Some("external delivery completed for all declared plan steps")
    );
}

#[test]
fn adapter_failure_maps_back_into_truthful_failed_status() {
    let route = selected_route(
        "route-decision-execute-allowed-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::FailedAtDeclaredStep {
        step_id: "step_export_prepare".to_owned(),
        failure_summary: "external target rejected declared step".to_owned(),
    });

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        trace.statuses[0].status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(trace.statuses[1].status.state, ExecutionState::InProgress);
    assert_eq!(
        trace.statuses[1].status.current_step.as_deref(),
        Some("step_export_prepare")
    );
    assert_eq!(trace.final_status().status.state, ExecutionState::Failed);
    assert_eq!(
        trace.final_status().status.current_posture,
        ApprovalPosture::ExecuteAllowed
    );
    assert_eq!(
        trace.final_status().status.failure_summary.as_deref(),
        Some("external target rejected declared step")
    );
}

#[test]
fn adapter_dependency_unavailable_maps_to_explicit_degraded_surface() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::DependencyUnavailable {
        summary: "external dependency unavailable for declared capability".to_owned(),
    });

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(trace.statuses.len(), 2);
    assert_eq!(
        trace.statuses[0].status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(trace.final_status().status.state, ExecutionState::Degraded);
    assert_eq!(
        trace.final_status().status.degraded_subtype,
        Some(DegradedSubtype::UnavailableDependencyBlock)
    );
    assert!(trace.final_status().status.started_at_utc.is_none());
    assert!(trace.final_status().status.current_step.is_none());
    assert_eq!(
        trace.final_status().status.truthful_user_visible_summary,
        "external dependency unavailable for declared capability"
    );
}

#[test]
fn adapter_cannot_invent_dynamic_steps() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::FailedAtDeclaredStep {
        step_id: "step_invented".to_owned(),
        failure_summary: "invented step failed".to_owned(),
    });

    let error = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap_err();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        error.to_string(),
        "contract invalid: adapter reported step that is not declared in execution route"
    );
}

#[test]
fn adapter_cannot_invent_fallbacks() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::CompletedWithDeclaredFallback {
        step_id: "step_export_prepare".to_owned(),
        fallback_step_id: "step_export_commit".to_owned(),
        degraded_subtype: DegradedSubtype::DegradedFallbackLimited,
    });

    let error = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap_err();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        error.to_string(),
        "contract invalid: adapter reported fallback that is not declared in execution route"
    );
}

#[test]
fn unsupported_adapter_conditions_fail_closed() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::Unsupported {
        summary: "transport state is not supported".to_owned(),
    });

    let error = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap_err();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        error.to_string(),
        "contract invalid: unsupported adapter condition from stub-external-adapter: transport state is not supported"
    );
}

#[test]
fn routing_and_adapter_delivery_boundaries_remain_separate() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = StubAdapter::new(AdapterDeliveryResult::CanceledAtDeclaredStep {
        step_id: "step_export_prepare".to_owned(),
    });

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    assert_eq!(
        route.route_path_kind,
        RoutePathKind::ExternalAdapterBoundedExecution
    );
    assert_eq!(
        route.resolved_approval_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert_eq!(trace.final_status().status.state, ExecutionState::Canceled);
    assert_eq!(
        trace.final_status().status.execution_plan_id,
        route.execution_plan_id
    );
    assert_eq!(
        trace.final_status().status.stable_plan_hash,
        route.stable_plan_hash
    );
}
