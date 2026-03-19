use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::guards::{DenialGuard, deny};
use crate::domain::shared::{
    ApprovalPosture, CapabilityId, DenialBasis, DenialReasonClass, DenialScope, EnvironmentMode,
    PolicyId, RequesterClass, SchemaName, SideEffectClass, TimestampUtc,
    deserialize_contract_value,
};
use crate::errors::FaLocalResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyScope {
    pub service_id: String,
    pub environment_modes: Vec<EnvironmentMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRule {
    pub capability_id: CapabilityId,
    pub allowed: bool,
    pub allowed_requester_classes: Vec<RequesterClass>,
    pub allowed_side_effect_classes: Vec<SideEffectClass>,
    pub required_approval_posture: ApprovalPosture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectRule {
    pub side_effect_class: SideEffectClass,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub requester_class: RequesterClass,
    pub max_posture: ApprovalPosture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyReadinessCondition {
    AllDependenciesReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureBehavior {
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProvenanceKind {
    LocalGovernedFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyProvenance {
    pub source_kind: PolicyProvenanceKind,
    pub issued_at: TimestampUtc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyArtifact {
    pub policy_id: PolicyId,
    pub policy_version: String,
    pub scope: PolicyScope,
    pub capability_rules: Vec<CapabilityRule>,
    pub side_effect_rules: Vec<SideEffectRule>,
    pub approval_rules: Vec<ApprovalRule>,
    pub environment_conditions: Vec<EnvironmentMode>,
    pub dependency_readiness_conditions: Vec<DependencyReadinessCondition>,
    pub failure_behavior: FailureBehavior,
    pub policy_provenance: PolicyProvenance,
}

#[derive(Debug, Default)]
pub struct PolicyArtifactLoader;

impl PolicyArtifactLoader {
    pub fn load_contract_value(value: &Value) -> FaLocalResult<PolicyArtifact> {
        deserialize_contract_value(SchemaName::PolicyArtifact, value)
    }

    pub fn load_required_value(value: Option<&Value>) -> Result<PolicyArtifact, DenialGuard> {
        let value = value.ok_or_else(|| {
            deny(
                DenialReasonClass::MissingPolicy,
                DenialScope::Artifact,
                DenialBasis::Policy,
                "missing policy artifact",
            )
        })?;

        Self::load_contract_value(value).map_err(|error| {
            deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Artifact,
                DenialBasis::Contract,
                format!("invalid policy artifact: {error}"),
            )
        })
    }
}

impl PolicyArtifact {
    pub fn capability_rule_for(&self, capability_id: CapabilityId) -> Option<&CapabilityRule> {
        self.capability_rules
            .iter()
            .find(|rule| rule.capability_id == capability_id)
    }
}
