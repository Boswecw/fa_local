use crate::domain::execution::ValidatedExecutionPlan;
use crate::domain::requester_trust::UserIntentBasis;
use crate::domain::review::{
    ApprovalOption, ReviewExecutionStatusContext, ReviewPackage, ValidatedReviewPackage,
};
use crate::domain::routing::RouteDecision;
use crate::domain::shared::{
    ApprovalPosture, ExecutionState, ReviewPackageId, TimestampUtc, now_utc,
};
use crate::domain::status::ValidatedExecutionStatus;
use crate::errors::{FaLocalError, FaLocalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewEmissionContext {
    pub packaged_at_utc: TimestampUtc,
}

impl ReviewEmissionContext {
    pub fn new(packaged_at_utc: TimestampUtc) -> Self {
        Self { packaged_at_utc }
    }
}

impl Default for ReviewEmissionContext {
    fn default() -> Self {
        Self::new(now_utc())
    }
}

#[derive(Debug, Clone)]
pub struct ReviewEmissionInput {
    pub route_decision: RouteDecision,
    pub validated_plan: Option<ValidatedExecutionPlan>,
    pub execution_status: Option<ValidatedExecutionStatus>,
    pub intent_basis: UserIntentBasis,
    pub requester_summary: String,
    pub proposed_execution_summary: String,
    pub side_effect_assessment: String,
    pub approval_options_allowed_by_policy: Vec<ApprovalOption>,
    pub denial_consequences_if_declined: String,
    pub context: ReviewEmissionContext,
}

impl ReviewEmissionInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        route_decision: RouteDecision,
        validated_plan: Option<ValidatedExecutionPlan>,
        execution_status: Option<ValidatedExecutionStatus>,
        intent_basis: UserIntentBasis,
        requester_summary: String,
        proposed_execution_summary: String,
        side_effect_assessment: String,
        approval_options_allowed_by_policy: Vec<ApprovalOption>,
        denial_consequences_if_declined: String,
        context: ReviewEmissionContext,
    ) -> FaLocalResult<Self> {
        validate_route_decision_surface(&route_decision)?;

        match route_decision.resolved_approval_posture {
            ApprovalPosture::Denied => {
                if validated_plan.is_some() {
                    return Err(contract_invalid(
                        "non-executable review-package emission path must not include validated execution plan",
                    ));
                }

                if execution_status.is_some() {
                    return Err(contract_invalid(
                        "non-executable review-package emission path must not include execution status context",
                    ));
                }
            }
            ApprovalPosture::ReviewRequired => {
                if validated_plan.is_some() {
                    return Err(contract_invalid(
                        "review_required review-package emission path must not include validated execution plan",
                    ));
                }

                if let Some(execution_status) = execution_status.as_ref() {
                    validate_status_matches_review_route(&route_decision, execution_status)?;
                }
            }
            ApprovalPosture::ExplicitOperatorApproval => {
                let validated_plan = validated_plan.as_ref().ok_or_else(|| {
                    contract_invalid(
                        "explicit approval review-package emission requires validated execution plan",
                    )
                })?;
                validate_plan_matches_route(&route_decision, validated_plan)?;

                if let Some(execution_status) = execution_status.as_ref() {
                    validate_status_matches_review_route(&route_decision, execution_status)?;
                }
            }
            ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed => {
                if let Some(validated_plan) = validated_plan.as_ref() {
                    validate_plan_matches_route(&route_decision, validated_plan)?;
                }

                if execution_status.is_some() {
                    return Err(contract_invalid(
                        "non-review review-package emission path must not include execution status context",
                    ));
                }
            }
        }

        Ok(Self {
            route_decision,
            validated_plan,
            execution_status,
            intent_basis,
            requester_summary,
            proposed_execution_summary,
            side_effect_assessment,
            approval_options_allowed_by_policy,
            denial_consequences_if_declined,
            context,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewNonEmissionReason {
    NonReviewPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewEmissionOutcome {
    Emitted(ValidatedReviewPackage),
    NotEmitted(ReviewNonEmissionReason),
}

#[derive(Debug, Default)]
pub struct ReviewService;

impl ReviewService {
    pub fn emit_review_package(
        &self,
        input: ReviewEmissionInput,
    ) -> FaLocalResult<ReviewEmissionOutcome> {
        let ReviewEmissionInput {
            route_decision,
            validated_plan,
            execution_status,
            intent_basis,
            requester_summary,
            proposed_execution_summary,
            side_effect_assessment,
            approval_options_allowed_by_policy,
            denial_consequences_if_declined,
            context,
        } = input;

        match route_decision.resolved_approval_posture {
            ApprovalPosture::Denied
            | ApprovalPosture::PolicyPreapproved
            | ApprovalPosture::ExecuteAllowed => Ok(ReviewEmissionOutcome::NotEmitted(
                ReviewNonEmissionReason::NonReviewPath,
            )),
            ApprovalPosture::ReviewRequired => {
                let execution_status_context = execution_status
                    .as_ref()
                    .map(review_status_context_from_status)
                    .transpose()?;

                let package = ReviewPackage::new(
                    ReviewPackageId::new(),
                    route_decision.request_id,
                    route_decision.correlation_id,
                    route_decision.route_decision_id,
                    None,
                    None,
                    route_decision.resolved_approval_posture,
                    execution_status_context,
                    intent_basis,
                    requester_summary,
                    proposed_execution_summary,
                    side_effect_assessment,
                    route_decision.degraded_subtype,
                    approval_options_allowed_by_policy,
                    denial_consequences_if_declined,
                    context.packaged_at_utc,
                )?;

                Ok(ReviewEmissionOutcome::Emitted(ValidatedReviewPackage::new(
                    package,
                )?))
            }
            ApprovalPosture::ExplicitOperatorApproval => {
                let validated_plan = validated_plan.as_ref().ok_or_else(|| {
                    contract_invalid(
                        "explicit approval review-package emission requires validated execution plan",
                    )
                })?;

                let execution_status_context = execution_status
                    .as_ref()
                    .map(review_status_context_from_status)
                    .transpose()?;

                let package = ReviewPackage::new(
                    ReviewPackageId::new(),
                    route_decision.request_id,
                    route_decision.correlation_id,
                    route_decision.route_decision_id,
                    Some(validated_plan.plan.execution_plan_id),
                    Some(validated_plan.stable_plan_hash.clone()),
                    route_decision.resolved_approval_posture,
                    execution_status_context,
                    intent_basis,
                    requester_summary,
                    proposed_execution_summary,
                    side_effect_assessment,
                    route_decision.degraded_subtype,
                    approval_options_allowed_by_policy,
                    denial_consequences_if_declined,
                    context.packaged_at_utc,
                )?;

                Ok(ReviewEmissionOutcome::Emitted(ValidatedReviewPackage::new(
                    package,
                )?))
            }
        }
    }
}

fn review_status_context_from_status(
    execution_status: &ValidatedExecutionStatus,
) -> FaLocalResult<ReviewExecutionStatusContext> {
    execution_status.status.validate()?;
    ReviewExecutionStatusContext::new(
        execution_status.status.state,
        execution_status.status.degraded_subtype,
        execution_status.status.updated_at_utc,
        execution_status
            .status
            .truthful_user_visible_summary
            .clone(),
    )
}

fn validate_route_decision_surface(route_decision: &RouteDecision) -> FaLocalResult<()> {
    match route_decision.resolved_approval_posture {
        ApprovalPosture::Denied => {
            if route_decision.execution_allowed
                || route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "denied route decision is inconsistent with review-package emission expectations",
                ));
            }
        }
        ApprovalPosture::ReviewRequired => {
            if route_decision.execution_allowed
                || !route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "review_required route decision is inconsistent with review-package emission expectations",
                ));
            }
        }
        ApprovalPosture::ExplicitOperatorApproval => {
            if route_decision.execution_allowed
                || route_decision.review_required
                || !route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "explicit_operator_approval route decision is inconsistent with review-package emission expectations",
                ));
            }
        }
        ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed => {
            if !route_decision.execution_allowed
                || route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "admitted route decision is inconsistent with review-package emission expectations",
                ));
            }
        }
    }

    Ok(())
}

fn validate_plan_matches_route(
    route_decision: &RouteDecision,
    validated_plan: &ValidatedExecutionPlan,
) -> FaLocalResult<()> {
    if validated_plan.plan.correlation_id != route_decision.correlation_id {
        return Err(contract_invalid(
            "review-package emission plan correlation_id does not match route decision",
        ));
    }

    if validated_plan.plan.originating_request_id != route_decision.request_id {
        return Err(contract_invalid(
            "review-package emission plan request_id does not match route decision",
        ));
    }

    if !validated_plan.plan.referenced_capabilities.contains(
        &route_decision
            .capability_decision_summary
            .requested_capability_id,
    ) {
        return Err(contract_invalid(
            "review-package emission plan does not include route decision requested capability",
        ));
    }

    Ok(())
}

fn validate_status_matches_review_route(
    route_decision: &RouteDecision,
    execution_status: &ValidatedExecutionStatus,
) -> FaLocalResult<()> {
    execution_status.status.validate()?;

    if execution_status.status.request_id != route_decision.request_id {
        return Err(contract_invalid(
            "review-package emission status request_id does not match route decision",
        ));
    }

    if execution_status.status.correlation_id != route_decision.correlation_id {
        return Err(contract_invalid(
            "review-package emission status correlation_id does not match route decision",
        ));
    }

    if execution_status.status.current_posture != route_decision.resolved_approval_posture {
        return Err(contract_invalid(
            "review-package emission status posture does not match route decision",
        ));
    }

    let review_status_context = review_status_context_from_status(execution_status)?;
    match route_decision.resolved_approval_posture {
        ApprovalPosture::ReviewRequired => {
            if execution_status.status.current_posture != ApprovalPosture::ReviewRequired {
                return Err(contract_invalid(
                    "review-package emission status must remain review_required",
                ));
            }

            if review_status_context.state != ExecutionState::ReviewRequired {
                return Err(contract_invalid(
                    "review-package emission status state does not match review_required posture",
                ));
            }
        }
        ApprovalPosture::ExplicitOperatorApproval => {
            if execution_status.status.current_posture != ApprovalPosture::ExplicitOperatorApproval
            {
                return Err(contract_invalid(
                    "review-package emission status must remain explicit_operator_approval",
                ));
            }

            if review_status_context.state != ExecutionState::WaitingExplicitApproval {
                return Err(contract_invalid(
                    "review-package emission status state does not match explicit_operator_approval posture",
                ));
            }
        }
        _ => {
            return Err(contract_invalid(
                "review-package emission status is only allowed for review postures",
            ));
        }
    }

    Ok(())
}

fn contract_invalid(message: impl Into<String>) -> FaLocalError {
    FaLocalError::ContractInvalid(message.into())
}
