use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::capabilities::ReviewClass;
use crate::domain::guards::DenialGuard;
use crate::domain::shared::{
    ApprovalPosture, CapabilityId, CorrelationId, DegradedSubtype, PolicyId, RequestId,
    RouteDecisionId, SchemaName, SideEffectClass, TimestampUtc, deserialize_contract_value,
};
use crate::errors::FaLocalResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyReference {
    pub policy_id: PolicyId,
    pub policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDecisionSummary {
    pub requested_capability_id: CapabilityId,
    pub capability_admitted: bool,
    pub capability_owner_service: Option<String>,
    pub requested_side_effect_class: SideEffectClass,
    pub capability_approval_posture: Option<ApprovalPosture>,
    pub policy_required_approval_posture: Option<ApprovalPosture>,
    pub requester_max_approval_posture: Option<ApprovalPosture>,
    pub side_effect_minimum_approval_posture: ApprovalPosture,
    pub review_class: Option<ReviewClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub route_decision_id: RouteDecisionId,
    pub correlation_id: CorrelationId,
    pub request_id: RequestId,
    pub resolved_approval_posture: ApprovalPosture,
    pub execution_allowed: bool,
    pub denial_guards: Vec<DenialGuard>,
    pub review_required: bool,
    pub explicit_approval_required: bool,
    pub policy_reference: Option<PolicyReference>,
    pub capability_decision_summary: CapabilityDecisionSummary,
    pub operator_visible_summary: String,
    pub degraded_subtype: Option<DegradedSubtype>,
    pub decided_at_utc: TimestampUtc,
}

#[derive(Debug, Default)]
pub struct RouteDecisionLoader;

impl RouteDecisionLoader {
    pub fn load_contract_value(value: &Value) -> FaLocalResult<RouteDecision> {
        deserialize_contract_value(SchemaName::RouteDecision, value)
    }
}
