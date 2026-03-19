use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::shared::{
    CorrelationId, EnvironmentMode, RequestId, RequesterId, SchemaName, SideEffectClass,
    TimestampUtc, deserialize_contract_value,
};
use crate::errors::FaLocalResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestIntent {
    ExecuteCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub requester_id: RequesterId,
    pub environment_mode: EnvironmentMode,
    pub requested_capability_id: crate::domain::shared::CapabilityId,
    pub requested_side_effect_class: SideEffectClass,
    pub intent: RequestIntent,
    pub intent_summary: String,
    pub requested_at: TimestampUtc,
}

impl ExecutionRequest {
    pub fn load_contract_value(value: &Value) -> FaLocalResult<Self> {
        deserialize_contract_value(SchemaName::ExecutionRequest, value)
    }
}

#[derive(Debug, Default)]
pub struct ExecutionCoordinator;
