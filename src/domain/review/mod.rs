use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::requester_trust::UserIntentBasis;
use crate::domain::shared::{
    ApprovalPosture, CorrelationId, DegradedSubtype, ExecutionPlanId, ExecutionState, RequestId,
    ReviewPackageId, RouteDecisionId, SchemaName, TimestampUtc, deserialize_contract_value,
};
use crate::errors::{FaLocalError, FaLocalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOption {
    ApproveExecute,
    DeclineRequest,
    DeferWithoutExecution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewExecutionStatusContext {
    pub state: ExecutionState,
    pub degraded_subtype: Option<DegradedSubtype>,
    pub updated_at_utc: TimestampUtc,
    pub truthful_user_visible_summary: String,
}

impl ReviewExecutionStatusContext {
    pub fn new(
        state: ExecutionState,
        degraded_subtype: Option<DegradedSubtype>,
        updated_at_utc: TimestampUtc,
        truthful_user_visible_summary: String,
    ) -> FaLocalResult<Self> {
        let context = Self {
            state,
            degraded_subtype,
            updated_at_utc,
            truthful_user_visible_summary,
        };
        context.validate()?;
        Ok(context)
    }

    pub fn validate(&self) -> FaLocalResult<()> {
        validate_required_summary(
            &self.truthful_user_visible_summary,
            "execution_status_context.truthful_user_visible_summary",
        )?;

        if !matches!(
            self.state,
            ExecutionState::ReviewRequired | ExecutionState::WaitingExplicitApproval
        ) {
            return Err(contract_invalid(
                "review package execution_status_context must remain review_required or waiting_explicit_approval",
            ));
        }

        if self.degraded_subtype.is_some() {
            return Err(contract_invalid(
                "review package execution_status_context must not imply degraded execution progress",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPackage {
    pub review_package_id: ReviewPackageId,
    pub originating_request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub route_decision_id: RouteDecisionId,
    pub execution_plan_id: Option<ExecutionPlanId>,
    pub stable_plan_hash: Option<String>,
    pub current_posture: ApprovalPosture,
    pub execution_status_context: Option<ReviewExecutionStatusContext>,
    pub intent_basis: UserIntentBasis,
    pub requester_summary: String,
    pub proposed_execution_summary: String,
    pub side_effect_assessment: String,
    pub degraded_or_fallback_posture: Option<DegradedSubtype>,
    pub approval_options_allowed_by_policy: Vec<ApprovalOption>,
    pub denial_consequences_if_declined: String,
    pub packaged_at_utc: TimestampUtc,
}

impl ReviewPackage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        review_package_id: ReviewPackageId,
        originating_request_id: RequestId,
        correlation_id: CorrelationId,
        route_decision_id: RouteDecisionId,
        execution_plan_id: Option<ExecutionPlanId>,
        stable_plan_hash: Option<String>,
        current_posture: ApprovalPosture,
        execution_status_context: Option<ReviewExecutionStatusContext>,
        intent_basis: UserIntentBasis,
        requester_summary: String,
        proposed_execution_summary: String,
        side_effect_assessment: String,
        degraded_or_fallback_posture: Option<DegradedSubtype>,
        approval_options_allowed_by_policy: Vec<ApprovalOption>,
        denial_consequences_if_declined: String,
        packaged_at_utc: TimestampUtc,
    ) -> FaLocalResult<Self> {
        let package = Self {
            review_package_id,
            originating_request_id,
            correlation_id,
            route_decision_id,
            execution_plan_id,
            stable_plan_hash,
            current_posture,
            execution_status_context,
            intent_basis,
            requester_summary,
            proposed_execution_summary,
            side_effect_assessment,
            degraded_or_fallback_posture,
            approval_options_allowed_by_policy,
            denial_consequences_if_declined,
            packaged_at_utc,
        };
        package.validate()?;
        Ok(package)
    }

    pub fn load_contract_value(value: &Value) -> FaLocalResult<Self> {
        deserialize_contract_value(SchemaName::ReviewPackage, value)
    }

    pub fn validate(&self) -> FaLocalResult<()> {
        if !matches!(
            self.current_posture,
            ApprovalPosture::ReviewRequired | ApprovalPosture::ExplicitOperatorApproval
        ) {
            return Err(contract_invalid(
                "review package must preserve review_required or explicit_operator_approval posture",
            ));
        }

        validate_optional_plan_link(self.execution_plan_id, self.stable_plan_hash.as_deref())?;

        if let Some(stable_plan_hash) = self.stable_plan_hash.as_deref() {
            if !is_valid_hash(stable_plan_hash) {
                return Err(contract_invalid(
                    "review package stable_plan_hash must be a 64-character lowercase hex digest",
                ));
            }
        }

        validate_required_summary(&self.requester_summary, "requester_summary")?;
        validate_required_summary(
            &self.proposed_execution_summary,
            "proposed_execution_summary",
        )?;
        validate_required_summary(&self.side_effect_assessment, "side_effect_assessment")?;
        validate_required_summary(
            &self.denial_consequences_if_declined,
            "denial_consequences_if_declined",
        )?;

        if self.approval_options_allowed_by_policy.is_empty() {
            return Err(contract_invalid(
                "review package must include at least one approval option",
            ));
        }

        let mut unique_options = BTreeSet::new();
        for option in &self.approval_options_allowed_by_policy {
            if !unique_options.insert(*option) {
                return Err(contract_invalid(
                    "review package approval options must be unique",
                ));
            }
        }

        if !self
            .approval_options_allowed_by_policy
            .contains(&ApprovalOption::ApproveExecute)
        {
            return Err(contract_invalid(
                "review package must include approve_execute as an allowed approval option",
            ));
        }

        if !self
            .approval_options_allowed_by_policy
            .contains(&ApprovalOption::DeclineRequest)
        {
            return Err(contract_invalid(
                "review package must include decline_request as an allowed approval option",
            ));
        }

        if mentions_degraded_or_fallback(&self.proposed_execution_summary)
            || mentions_degraded_or_fallback(&self.side_effect_assessment)
        {
            if self.degraded_or_fallback_posture.is_none() {
                return Err(contract_invalid(
                    "review package cannot narrate degraded or fallback posture without explicit degraded_or_fallback_posture",
                ));
            }
        }

        if let Some(execution_status_context) = &self.execution_status_context {
            execution_status_context.validate()?;
        }

        match self.current_posture {
            ApprovalPosture::ReviewRequired => {
                require_no_plan_link(
                    self.execution_plan_id,
                    self.stable_plan_hash.as_deref(),
                    "review_required review package must not include execution_plan_id or stable_plan_hash",
                )?;

                if let Some(execution_status_context) = &self.execution_status_context {
                    if execution_status_context.state != ExecutionState::ReviewRequired {
                        return Err(contract_invalid(
                            "review package execution_status_context must remain review_required for review_required posture",
                        ));
                    }
                }
            }
            ApprovalPosture::ExplicitOperatorApproval => {
                require_plan_link(
                    self.execution_plan_id,
                    self.stable_plan_hash.as_deref(),
                    "explicit approval review package must include execution_plan_id and stable_plan_hash",
                )?;

                if let Some(execution_status_context) = &self.execution_status_context {
                    if execution_status_context.state != ExecutionState::WaitingExplicitApproval {
                        return Err(contract_invalid(
                            "review package execution_status_context must remain waiting_explicit_approval for explicit_operator_approval posture",
                        ));
                    }
                }
            }
            _ => unreachable!("non-review postures rejected above"),
        }

        Ok(())
    }

    pub fn validated(self) -> FaLocalResult<ValidatedReviewPackage> {
        self.validate()?;
        Ok(ValidatedReviewPackage { package: self })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedReviewPackage {
    pub package: ReviewPackage,
}

impl ValidatedReviewPackage {
    pub fn new(package: ReviewPackage) -> FaLocalResult<Self> {
        package.validated()
    }
}

fn contract_invalid(message: impl Into<String>) -> FaLocalError {
    FaLocalError::ContractInvalid(message.into())
}

fn validate_required_summary(summary: &str, field_name: &str) -> FaLocalResult<()> {
    if summary.is_empty() || summary.len() > 160 {
        return Err(contract_invalid(format!(
            "review package {field_name} must be between 1 and 160 characters",
        )));
    }
    Ok(())
}

fn validate_optional_plan_link(
    execution_plan_id: Option<ExecutionPlanId>,
    stable_plan_hash: Option<&str>,
) -> FaLocalResult<()> {
    match (execution_plan_id, stable_plan_hash) {
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => Err(contract_invalid(
            "review package must carry execution_plan_id and stable_plan_hash together",
        )),
    }
}

fn require_plan_link(
    execution_plan_id: Option<ExecutionPlanId>,
    stable_plan_hash: Option<&str>,
    error_message: &'static str,
) -> FaLocalResult<()> {
    match (execution_plan_id, stable_plan_hash) {
        (Some(_), Some(_)) => Ok(()),
        _ => Err(contract_invalid(error_message)),
    }
}

fn require_no_plan_link(
    execution_plan_id: Option<ExecutionPlanId>,
    stable_plan_hash: Option<&str>,
    error_message: &'static str,
) -> FaLocalResult<()> {
    match (execution_plan_id, stable_plan_hash) {
        (None, None) => Ok(()),
        _ => Err(contract_invalid(error_message)),
    }
}

fn is_valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn mentions_degraded_or_fallback(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("fallback") || lower.contains("degraded")
}
