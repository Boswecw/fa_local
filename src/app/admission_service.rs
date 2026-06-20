//! End-to-end admission facade.
//!
//! Runs the existing three-layer gate as a single call:
//! requester-trust -> policy -> capability admission -> approval-posture
//! resolution -> route decision, and (for a review-required posture) emits a
//! review package. This is pure composition over the domain + app services — no
//! daemon, no persistence, no I/O beyond the JSON values handed in.
//!
//! Fail-closed: the execution request must parse (it is the thing being
//! admitted), but every other prerequisite failure flows into the route decision
//! as a structured `DenialGuard` rather than a silent pass. The resolver is the
//! sink: it accepts the three `Result` outcomes and produces a `RouteDecision`.

use serde_json::Value;

use crate::app::review_service::{
    ReviewEmissionContext, ReviewEmissionInput, ReviewEmissionOutcome, ReviewService,
};
use crate::domain::capabilities::CapabilityRegistryLoader;
use crate::domain::execution::ExecutionRequest;
use crate::domain::guards::deny;
use crate::domain::policy::PolicyArtifactLoader;
use crate::domain::posture::{
    ApprovalPostureResolver, RouteResolutionContext, RouteResolutionInput,
};
use crate::domain::requester_trust::{
    RequesterTrustEngine, TrustEvaluationContext, UserIntentBasis,
};
use crate::domain::review::{ApprovalOption, ValidatedReviewPackage};
use crate::domain::routing::RouteDecision;
use crate::domain::shared::{
    ApprovalPosture, DenialBasis, DenialReasonClass, DenialScope, EnvironmentMode, RouteDecisionId,
    TimestampUtc,
};
use crate::errors::FaLocalResult;

/// JSON inputs for one admission decision. Each value is a contract envelope —
/// the same shapes the per-contract loaders validate.
#[derive(Debug, Clone)]
pub struct AdmissionInputs {
    pub requester_trust: Value,
    pub policy: Value,
    pub capability_registry: Value,
    pub execution_request: Value,
}

/// Evaluation context for an admission decision.
#[derive(Debug, Clone)]
pub struct AdmissionContext {
    pub expected_environment: EnvironmentMode,
    pub now: TimestampUtc,
    pub route_decision_id: RouteDecisionId,
}

impl AdmissionContext {
    /// Build a context with a freshly minted route-decision id.
    pub fn new(expected_environment: EnvironmentMode, now: TimestampUtc) -> Self {
        Self {
            expected_environment,
            now,
            route_decision_id: RouteDecisionId::new(),
        }
    }
}

/// Result of running the gate end-to-end.
#[derive(Debug, Clone)]
pub struct AdmissionOutcome {
    pub route_decision: RouteDecision,
    /// Emitted only for a `review_required` posture (which requires no execution
    /// plan). Explicit-approval and executable postures return `None` here — the
    /// caller proceeds per the route flags (supplying a plan for explicit
    /// approval in a later step).
    pub review_package: Option<ValidatedReviewPackage>,
}

/// Stateless facade that orchestrates the admission gate.
#[derive(Debug, Default)]
pub struct AdmissionService;

impl AdmissionService {
    /// Run the admission gate over `inputs`, returning a structured outcome.
    pub fn admit(
        &self,
        inputs: &AdmissionInputs,
        context: AdmissionContext,
    ) -> FaLocalResult<AdmissionOutcome> {
        // The request itself must parse — it is the thing being admitted.
        let request = ExecutionRequest::load_contract_value(&inputs.execution_request)?;

        let trust_ctx = TrustEvaluationContext {
            expected_environment: context.expected_environment,
            now: context.now,
        };
        let requester_trust_outcome =
            RequesterTrustEngine::load_and_evaluate(&inputs.requester_trust, &trust_ctx);
        let policy_outcome = PolicyArtifactLoader::load_required_value(Some(&inputs.policy));

        // Capability admission can only run once trust and policy hold; otherwise
        // it is denied with an explicit prerequisite reason (never skipped silently).
        let capability_admission_outcome =
            match (requester_trust_outcome.as_ref(), policy_outcome.as_ref()) {
                (Ok(requester), Ok(policy)) => {
                    match CapabilityRegistryLoader::load_contract_value(&inputs.capability_registry)
                    {
                        Ok(registry) => CapabilityRegistryLoader::admit_execution_request(
                            &registry, policy, requester, &request,
                        ),
                        Err(err) => Err(deny(
                            DenialReasonClass::ContractInvalid,
                            DenialScope::Capability,
                            DenialBasis::Contract,
                            format!("capability registry invalid: {err}"),
                        )),
                    }
                }
                _ => Err(deny(
                    DenialReasonClass::DependencyUnavailable,
                    DenialScope::Capability,
                    DenialBasis::RuntimeSafety,
                    "capability admission skipped: requester-trust or policy prerequisite denied",
                )),
            };

        // Capture intent basis before the trust outcome is moved into the resolver.
        let intent_basis = requester_trust_outcome
            .as_ref()
            .ok()
            .and_then(|envelope| envelope.user_intent_basis)
            .unwrap_or(UserIntentBasis::OperatorApproval);

        let route_decision = ApprovalPostureResolver::resolve(
            RouteResolutionInput {
                request,
                requester_trust_outcome,
                policy_outcome,
                capability_admission_outcome,
            },
            RouteResolutionContext::new(context.route_decision_id, context.now),
        );

        let review_package = self.maybe_emit_review(&route_decision, intent_basis, context.now)?;

        Ok(AdmissionOutcome {
            route_decision,
            review_package,
        })
    }

    /// Emit a review package for a `review_required` route (which needs no
    /// execution plan). Other postures return `None`.
    fn maybe_emit_review(
        &self,
        route_decision: &RouteDecision,
        intent_basis: UserIntentBasis,
        now: TimestampUtc,
    ) -> FaLocalResult<Option<ValidatedReviewPackage>> {
        if route_decision.resolved_approval_posture != ApprovalPosture::ReviewRequired {
            return Ok(None);
        }

        let summary = &route_decision.capability_decision_summary;
        let input = ReviewEmissionInput::new(
            route_decision.clone(),
            None,
            None,
            intent_basis,
            route_decision.operator_visible_summary.clone(),
            format!(
                "capability {} requested (side effect {:?})",
                summary.requested_capability_id, summary.requested_side_effect_class
            ),
            format!(
                "requested side effect class: {:?}",
                summary.requested_side_effect_class
            ),
            vec![
                ApprovalOption::ApproveExecute,
                ApprovalOption::DeclineRequest,
                ApprovalOption::DeferWithoutExecution,
            ],
            "Declining leaves the request unexecuted; no side effect occurs.".to_string(),
            ReviewEmissionContext::new(now),
        )?;

        match ReviewService.emit_review_package(input)? {
            ReviewEmissionOutcome::Emitted(package) => Ok(Some(package)),
            ReviewEmissionOutcome::NotEmitted(_) => Ok(None),
        }
    }
}
