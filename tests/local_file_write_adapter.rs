mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use fa_local::adapters::execution_delivery::local_file_write::{
    LocalFileWriteAdapterConfig, LocalFileWriteDeliveryAdapter,
};
use fa_local::adapters::execution_delivery::{
    AdapterDeliveryRequest, AdapterDeliveryResult, ExternalRouteDeliveryAdapter,
};
use fa_local::app::execution_service::{CoordinationContext, ExecutionService};
use fa_local::app::routing_service::{RoutingInput, RoutingService, SelectedExecutionRoute};
use fa_local::domain::capabilities::{CapabilityRegistry, CapabilityRegistryLoader};
use fa_local::domain::execution::{ExecutionPlan, ExecutionPlanValidator, ValidatedExecutionPlan};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::{DegradedSubtype, ExecutionState};

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
        ts(2030, 1, 1, 0, 30, 0),
        ts(2030, 1, 1, 0, 30, 5),
        ts(2030, 1, 1, 0, 30, 30),
    )
}

fn capability_registry() -> CapabilityRegistry {
    CapabilityRegistryLoader::load_contract_value(&support::load_fixture_json(
        "valid",
        "capability-registry-basic.json",
    ))
    .unwrap()
}

fn supported_capability_id() -> fa_local::CapabilityId {
    capability_registry().capabilities[0].capability_id
}

fn validated_plan() -> ValidatedExecutionPlan {
    let registry = capability_registry();
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

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("fa-local-{label}-{}", Uuid::new_v4()))
}

#[derive(Debug)]
struct CountingAdapter<A> {
    inner: A,
    calls: AtomicUsize,
}

impl<A> CountingAdapter<A> {
    fn new(inner: A) -> Self {
        Self {
            inner,
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl<A: ExternalRouteDeliveryAdapter> ExternalRouteDeliveryAdapter for CountingAdapter<A> {
    fn adapter_id(&self) -> &'static str {
        self.inner.adapter_id()
    }

    fn deliver_route(&self, request: &AdapterDeliveryRequest) -> AdapterDeliveryResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.deliver_route(request)
    }
}

#[derive(Debug, Clone)]
struct FixedResultAdapter {
    result: AdapterDeliveryResult,
}

impl ExternalRouteDeliveryAdapter for FixedResultAdapter {
    fn adapter_id(&self) -> &'static str {
        "fixed-result-adapter"
    }

    fn deliver_route(&self, _request: &AdapterDeliveryRequest) -> AdapterDeliveryResult {
        self.result.clone()
    }
}

fn local_file_write_adapter(delivery_root: PathBuf) -> LocalFileWriteDeliveryAdapter {
    LocalFileWriteDeliveryAdapter::new(LocalFileWriteAdapterConfig::new(
        supported_capability_id(),
        delivery_root,
    ))
}

#[test]
fn denied_and_review_required_routes_never_invoke_concrete_adapter() {
    let delivery_root = temp_dir("concrete-adapter-not-invoked");
    fs::create_dir_all(&delivery_root).unwrap();
    let adapter = CountingAdapter::new(local_file_write_adapter(delivery_root.clone()));

    let denied_error = ExecutionService
        .deliver_selected_route(
            &selected_route("route-decision-denied-basic.json", None),
            &adapter,
            context(),
        )
        .unwrap_err();
    assert_eq!(
        denied_error.to_string(),
        "contract invalid: non-executable route must not reach adapter delivery"
    );

    let review_error = ExecutionService
        .deliver_selected_route(
            &selected_route("route-decision-review-required-basic.json", None),
            &adapter,
            context(),
        )
        .unwrap_err();
    assert_eq!(
        review_error.to_string(),
        "contract invalid: non-executable route must not reach adapter delivery"
    );

    assert_eq!(adapter.calls(), 0);
    assert_eq!(fs::read_dir(&delivery_root).unwrap().count(), 0);

    let _ = fs::remove_dir_all(delivery_root);
}

#[test]
fn admitted_routed_work_reaches_concrete_adapter_and_maps_success_truthfully() {
    let delivery_root = temp_dir("concrete-adapter-success");
    fs::create_dir_all(&delivery_root).unwrap();
    let adapter = local_file_write_adapter(delivery_root.clone());
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    let receipt_path =
        adapter.receipt_path_for(route.request_id, route.stable_plan_hash.as_deref().unwrap());
    let receipt = fs::read_to_string(&receipt_path).unwrap();

    assert_eq!(
        trace.statuses.first().unwrap().status.state,
        ExecutionState::AdmittedNotStarted
    );
    assert_eq!(trace.final_status().status.state, ExecutionState::Completed);
    assert!(receipt.contains("adapter_id=local-file-write-delivery"));
    assert!(receipt.contains(&format!("request_id={}", route.request_id)));
    assert!(receipt.contains("declared_step_ids=step_export_prepare,step_export_commit"));

    let _ = fs::remove_dir_all(delivery_root);
}

#[test]
fn unavailable_dependency_maps_to_truthful_degraded_status() {
    let delivery_root = temp_dir("concrete-adapter-missing-root");
    let adapter = local_file_write_adapter(delivery_root.clone());
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    assert_eq!(trace.statuses.len(), 2);
    assert_eq!(trace.final_status().status.state, ExecutionState::Degraded);
    assert_eq!(
        trace.final_status().status.degraded_subtype,
        Some(DegradedSubtype::UnavailableDependencyBlock)
    );
    assert_eq!(
        trace.final_status().status.truthful_user_visible_summary,
        "local file write delivery root is unavailable"
    );
}

#[test]
fn adapter_refusal_maps_to_truthful_failed_status() {
    let delivery_root = temp_dir("concrete-adapter-refusal");
    let adapter = local_file_write_adapter(delivery_root.clone());
    fs::create_dir_all(adapter.delivery_root()).unwrap();
    fs::write(adapter.refusal_marker_path(), "refuse").unwrap();
    let route = selected_route(
        "route-decision-execute-allowed-basic.json",
        Some(validated_plan()),
    );

    let trace = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap();

    assert_eq!(trace.statuses[1].status.state, ExecutionState::InProgress);
    assert_eq!(trace.final_status().status.state, ExecutionState::Failed);
    assert_eq!(
        trace.final_status().status.failure_summary.as_deref(),
        Some("local file write delivery refused by operator marker")
    );

    let _ = fs::remove_dir_all(delivery_root);
}

#[test]
fn malformed_adapter_outputs_fail_closed() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let adapter = FixedResultAdapter {
        result: AdapterDeliveryResult::CompletedWithDeclaredFallback {
            step_id: "step_export_prepare".to_owned(),
            fallback_step_id: "step_export_commit".to_owned(),
            degraded_subtype: DegradedSubtype::DegradedInFlight,
        },
    };

    let error = ExecutionService
        .deliver_selected_route(&route, &adapter, context())
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: adapter fallback completion must use an explicit fallback degraded_subtype"
    );
}

#[test]
fn dynamic_step_and_fallback_invention_fail_closed() {
    let route = selected_route(
        "route-decision-policy-preapproved-basic.json",
        Some(validated_plan()),
    );
    let invented_step = FixedResultAdapter {
        result: AdapterDeliveryResult::FailedAtDeclaredStep {
            step_id: "step_invented".to_owned(),
            failure_summary: "invented step failed".to_owned(),
        },
    };
    let step_error = ExecutionService
        .deliver_selected_route(&route, &invented_step, context())
        .unwrap_err();
    assert_eq!(
        step_error.to_string(),
        "contract invalid: adapter reported step that is not declared in execution route"
    );

    let invented_fallback = FixedResultAdapter {
        result: AdapterDeliveryResult::CompletedWithDeclaredFallback {
            step_id: "step_export_prepare".to_owned(),
            fallback_step_id: "step_export_commit".to_owned(),
            degraded_subtype: DegradedSubtype::DegradedFallbackLimited,
        },
    };
    let fallback_error = ExecutionService
        .deliver_selected_route(&route, &invented_fallback, context())
        .unwrap_err();
    assert_eq!(
        fallback_error.to_string(),
        "contract invalid: adapter reported fallback that is not declared in execution route"
    );
}
