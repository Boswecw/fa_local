mod support;

use std::cell::Cell;

use chrono::{TimeZone, Utc};

use fa_local::adapters::exports::{ForensicEventExportAdapter, ForensicExportResult};
use fa_local::app::forensic_service::{
    ForensicRecordContext, ForensicRecordInput, ForensicRecordKind, ForensicService,
};
use fa_local::domain::forensics::ForensicEventType;
use fa_local::domain::review::{ReviewPackage, ValidatedReviewPackage};
use fa_local::domain::routing::{RouteDecision, RouteDecisionLoader};
use fa_local::domain::status::{ExecutionStatus, ValidatedExecutionStatus};
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

fn context() -> ForensicRecordContext {
    ForensicRecordContext::new(ts(2030, 1, 1, 0, 20, 0))
}

fn route_decision(file_name: &str) -> RouteDecision {
    RouteDecisionLoader::load_contract_value(&support::load_fixture_json("valid", file_name))
        .unwrap()
}

fn validated_review_package(file_name: &str) -> ValidatedReviewPackage {
    let package =
        ReviewPackage::load_contract_value(&support::load_fixture_json("valid", file_name))
            .unwrap();
    ValidatedReviewPackage::new(package).unwrap()
}

fn validated_execution_status(file_name: &str) -> ValidatedExecutionStatus {
    let status =
        ExecutionStatus::load_contract_value(&support::load_fixture_json("valid", file_name))
            .unwrap();
    ValidatedExecutionStatus::new(status).unwrap()
}

#[derive(Debug)]
struct FixedExportAdapter {
    result: ForensicExportResult,
    calls: Cell<usize>,
}

impl FixedExportAdapter {
    fn new(result: ForensicExportResult) -> Self {
        Self {
            result,
            calls: Cell::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl ForensicEventExportAdapter for FixedExportAdapter {
    fn adapter_id(&self) -> &'static str {
        "test-forensic-export"
    }

    fn export_event(
        &self,
        _event: &fa_local::domain::forensics::ValidatedForensicEvent,
    ) -> ForensicExportResult {
        self.calls.set(self.calls.get() + 1);
        self.result.clone()
    }
}

#[test]
fn records_valid_explicit_review_package_with_truthful_linkage() {
    let route = route_decision("route-decision-explicit-operator-approval-basic.json");
    let review_package = validated_review_package("review-package-basic.json");

    let event = ForensicService
        .record_event(
            ForensicRecordInput::new(
                ForensicRecordKind::ReviewPackagePrepared {
                    route_decision: route,
                    review_package,
                },
                fa_local::domain::forensics::RedactionLevel::SensitiveFieldsRedacted,
                context(),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(
        event.event.event_type,
        ForensicEventType::ReviewPackagePrepared
    );
    assert_eq!(
        event.event.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(
        event.event.execution_state,
        ExecutionState::WaitingExplicitApproval
    );
    assert!(event.event.execution_plan_id.is_some());
    assert!(event.event.stable_plan_hash.is_some());
    assert!(event.event.review_package_id.is_some());
}

#[test]
fn route_resolution_record_preserves_posture_state_distinction() {
    let route = route_decision("route-decision-explicit-operator-approval-basic.json");

    let event = ForensicService
        .record_event(
            ForensicRecordInput::new(
                ForensicRecordKind::RouteDecisionResolved {
                    route_decision: route,
                },
                fa_local::domain::forensics::RedactionLevel::SensitiveFieldsRedacted,
                context(),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(
        event.event.current_posture,
        ApprovalPosture::ExplicitOperatorApproval
    );
    assert_eq!(
        event.event.execution_state,
        ExecutionState::WaitingExplicitApproval
    );
    assert_ne!(
        format!("{:?}", event.event.current_posture),
        format!("{:?}", event.event.execution_state)
    );
}

#[test]
fn records_and_exports_valid_execution_status_event() {
    let route = route_decision("route-decision-policy-preapproved-basic.json");
    let execution_status =
        validated_execution_status("execution-status-completed-with-constraints-basic.json");
    let adapter = FixedExportAdapter::new(ForensicExportResult::Exported {
        export_reference: "audit://forensics/event-0001.json".to_owned(),
    });

    let outcome = ForensicService
        .record_and_export_event(
            ForensicRecordInput::new(
                ForensicRecordKind::ExecutionStatusObserved {
                    route_decision: route,
                    execution_status,
                },
                fa_local::domain::forensics::RedactionLevel::LinkageOnly,
                context(),
            )
            .unwrap(),
            &adapter,
        )
        .unwrap();

    assert_eq!(adapter.calls(), 1);
    assert_eq!(
        outcome.event.event.event_type,
        ForensicEventType::ExecutionStatusObserved
    );
    assert_eq!(
        outcome.event.event.current_posture,
        ApprovalPosture::PolicyPreapproved
    );
    assert_eq!(
        outcome.event.event.execution_state,
        ExecutionState::CompletedWithConstraints
    );
    assert_eq!(
        outcome.event.event.summary,
        "execution completed with declared fallback limits"
    );
    assert!(!outcome.event.event.summary.contains("planner"));
    assert!(!outcome.event.event.summary.contains("workflow"));
    assert_eq!(
        outcome.export_receipt.export_reference,
        "audit://forensics/event-0001.json"
    );
}

#[test]
fn review_required_review_package_is_rejected_under_current_forensic_contract_boundary() {
    let error = ForensicRecordInput::new(
        ForensicRecordKind::ReviewPackagePrepared {
            route_decision: route_decision("route-decision-review-required-basic.json"),
            review_package: validated_review_package("review-package-review-required-basic.json"),
        },
        fa_local::domain::forensics::RedactionLevel::SensitiveFieldsRedacted,
        context(),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: forensic review-package record currently requires explicit_operator_approval posture"
    );
}

#[test]
fn fabricated_success_summary_for_pre_execution_route_fails_closed_before_export() {
    let mut route = route_decision("route-decision-denied-basic.json");
    route.operator_visible_summary = "execution completed successfully".to_owned();
    let adapter = FixedExportAdapter::new(ForensicExportResult::Exported {
        export_reference: "audit://forensics/should-not-exist.json".to_owned(),
    });

    let error = ForensicService
        .record_and_export_event(
            ForensicRecordInput::new(
                ForensicRecordKind::DenialIssued {
                    route_decision: route,
                },
                fa_local::domain::forensics::RedactionLevel::SensitiveFieldsRedacted,
                context(),
            )
            .unwrap(),
            &adapter,
        )
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: forensic event summary must not claim success or completion before truthful execution completion"
    );
    assert_eq!(adapter.calls(), 0);
}

#[test]
fn contradictory_route_and_execution_status_inputs_fail_closed() {
    let error = ForensicRecordInput::new(
        ForensicRecordKind::ExecutionStatusObserved {
            route_decision: route_decision("route-decision-review-required-basic.json"),
            execution_status: validated_execution_status(
                "execution-status-completed-with-constraints-basic.json",
            ),
        },
        fa_local::domain::forensics::RedactionLevel::LinkageOnly,
        context(),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: forensic execution-status record requires an admitted route posture"
    );
}

#[test]
fn malformed_export_reference_fails_closed() {
    let adapter = FixedExportAdapter::new(ForensicExportResult::Exported {
        export_reference: String::new(),
    });

    let error = ForensicService
        .record_and_export_event(
            ForensicRecordInput::new(
                ForensicRecordKind::ExecutionStatusObserved {
                    route_decision: route_decision("route-decision-policy-preapproved-basic.json"),
                    execution_status: validated_execution_status(
                        "execution-status-completed-with-constraints-basic.json",
                    ),
                },
                fa_local::domain::forensics::RedactionLevel::LinkageOnly,
                context(),
            )
            .unwrap(),
            &adapter,
        )
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "contract invalid: forensic export adapter must return a bounded export_reference between 1 and 160 characters"
    );
    assert_eq!(adapter.calls(), 1);
}
