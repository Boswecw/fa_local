use crate::domain::execution::{FallbackReference, ValidatedExecutionPlan};
use crate::domain::routing::RouteDecision;
use crate::domain::shared::{
    ApprovalPosture, CapabilityId, CorrelationId, ExecutionPlanId, RequestId, RouteDecisionId,
};
use crate::errors::{FaLocalError, FaLocalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePathKind {
    NonExecutableDenied,
    NonExecutableReviewRequired,
    AwaitExplicitApproval,
    ExternalAdapterBoundedExecution,
}

#[derive(Debug, Clone)]
pub struct RoutingInput {
    pub route_decision: RouteDecision,
    pub validated_plan: Option<ValidatedExecutionPlan>,
}

impl RoutingInput {
    pub fn new(
        route_decision: RouteDecision,
        validated_plan: Option<ValidatedExecutionPlan>,
    ) -> FaLocalResult<Self> {
        validate_route_decision_surface(&route_decision)?;

        match route_decision.resolved_approval_posture {
            ApprovalPosture::Denied | ApprovalPosture::ReviewRequired => {
                if validated_plan.is_some() {
                    return Err(contract_invalid(
                        "non-executable routing input must not include validated execution plan",
                    ));
                }
            }
            ApprovalPosture::ExplicitOperatorApproval
            | ApprovalPosture::PolicyPreapproved
            | ApprovalPosture::ExecuteAllowed => {
                let validated_plan = validated_plan.as_ref().ok_or_else(|| {
                    contract_invalid("routable execution input requires validated execution plan")
                })?;
                validate_plan_matches_route(&route_decision, validated_plan)?;
            }
        }

        Ok(Self {
            route_decision,
            validated_plan,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedExecutionRoute {
    pub route_path_kind: RoutePathKind,
    pub route_decision_id: RouteDecisionId,
    pub correlation_id: CorrelationId,
    pub request_id: RequestId,
    pub resolved_approval_posture: ApprovalPosture,
    pub requested_capability_id: CapabilityId,
    pub execution_plan_id: Option<ExecutionPlanId>,
    pub stable_plan_hash: Option<String>,
    pub declared_step_ids: Vec<String>,
    pub declared_capability_ids: Vec<CapabilityId>,
    pub declared_fallback_references: Vec<FallbackReference>,
    pub executable: bool,
    pub explicit_approval_required: bool,
    pub operator_visible_summary: String,
}

#[derive(Debug, Default)]
pub struct RoutingService;

impl RoutingService {
    pub fn select_route(&self, input: RoutingInput) -> FaLocalResult<SelectedExecutionRoute> {
        let RoutingInput {
            route_decision,
            validated_plan,
        } = input;

        match route_decision.resolved_approval_posture {
            ApprovalPosture::Denied => Ok(non_executable_route(
                &route_decision,
                RoutePathKind::NonExecutableDenied,
                false,
            )),
            ApprovalPosture::ReviewRequired => Ok(non_executable_route(
                &route_decision,
                RoutePathKind::NonExecutableReviewRequired,
                false,
            )),
            ApprovalPosture::ExplicitOperatorApproval => {
                let validated_plan = validated_plan.as_ref().ok_or_else(|| {
                    contract_invalid("explicit approval route selection requires validated plan")
                })?;
                Ok(routable_plan_selection(
                    &route_decision,
                    validated_plan,
                    RoutePathKind::AwaitExplicitApproval,
                    false,
                    true,
                ))
            }
            ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed => {
                let validated_plan = validated_plan.as_ref().ok_or_else(|| {
                    contract_invalid("admitted route selection requires validated plan")
                })?;
                Ok(routable_plan_selection(
                    &route_decision,
                    validated_plan,
                    RoutePathKind::ExternalAdapterBoundedExecution,
                    true,
                    false,
                ))
            }
        }
    }
}

fn non_executable_route(
    route_decision: &RouteDecision,
    route_path_kind: RoutePathKind,
    explicit_approval_required: bool,
) -> SelectedExecutionRoute {
    SelectedExecutionRoute {
        route_path_kind,
        route_decision_id: route_decision.route_decision_id,
        correlation_id: route_decision.correlation_id,
        request_id: route_decision.request_id,
        resolved_approval_posture: route_decision.resolved_approval_posture,
        requested_capability_id: route_decision
            .capability_decision_summary
            .requested_capability_id,
        execution_plan_id: None,
        stable_plan_hash: None,
        declared_step_ids: Vec::new(),
        declared_capability_ids: Vec::new(),
        declared_fallback_references: Vec::new(),
        executable: false,
        explicit_approval_required,
        operator_visible_summary: route_decision.operator_visible_summary.clone(),
    }
}

fn routable_plan_selection(
    route_decision: &RouteDecision,
    validated_plan: &ValidatedExecutionPlan,
    route_path_kind: RoutePathKind,
    executable: bool,
    explicit_approval_required: bool,
) -> SelectedExecutionRoute {
    SelectedExecutionRoute {
        route_path_kind,
        route_decision_id: route_decision.route_decision_id,
        correlation_id: route_decision.correlation_id,
        request_id: route_decision.request_id,
        resolved_approval_posture: route_decision.resolved_approval_posture,
        requested_capability_id: route_decision
            .capability_decision_summary
            .requested_capability_id,
        execution_plan_id: Some(validated_plan.plan.execution_plan_id),
        stable_plan_hash: Some(validated_plan.stable_plan_hash.clone()),
        declared_step_ids: validated_plan
            .plan
            .steps
            .iter()
            .map(|step| step.step_id.clone())
            .collect(),
        declared_capability_ids: validated_plan.plan.referenced_capabilities.clone(),
        declared_fallback_references: validated_plan.plan.fallback_references.clone(),
        executable,
        explicit_approval_required,
        operator_visible_summary: route_decision.operator_visible_summary.clone(),
    }
}

fn validate_route_decision_surface(route_decision: &RouteDecision) -> FaLocalResult<()> {
    match route_decision.resolved_approval_posture {
        ApprovalPosture::Denied => {
            if route_decision.execution_allowed
                || route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "denied route decision is inconsistent with routing expectations",
                ));
            }
        }
        ApprovalPosture::ReviewRequired => {
            if route_decision.execution_allowed
                || !route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "review_required route decision is inconsistent with routing expectations",
                ));
            }
        }
        ApprovalPosture::ExplicitOperatorApproval => {
            if route_decision.execution_allowed
                || route_decision.review_required
                || !route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "explicit_operator_approval route decision is inconsistent with routing expectations",
                ));
            }
        }
        ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed => {
            if !route_decision.execution_allowed
                || route_decision.review_required
                || route_decision.explicit_approval_required
            {
                return Err(contract_invalid(
                    "admitted route decision is inconsistent with routing expectations",
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
            "routing plan correlation_id does not match route decision",
        ));
    }

    if validated_plan.plan.originating_request_id != route_decision.request_id {
        return Err(contract_invalid(
            "routing plan request_id does not match route decision",
        ));
    }

    if !validated_plan.plan.referenced_capabilities.contains(
        &route_decision
            .capability_decision_summary
            .requested_capability_id,
    ) {
        return Err(contract_invalid(
            "routing plan does not include route decision requested capability",
        ));
    }

    Ok(())
}

fn contract_invalid(message: impl Into<String>) -> FaLocalError {
    FaLocalError::ContractInvalid(message.into())
}
