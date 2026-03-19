use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::SERVICE_ID;
use crate::domain::capabilities::{CapabilityRegistry, EnabledState};
use crate::domain::guards::{DenialGuard, deny};
use crate::domain::shared::{
    CapabilityId, CorrelationId, DenialBasis, DenialReasonClass, DenialScope, EnvironmentMode,
    ExecutionPlanId, RequestId, RequesterId, RevocationState, SchemaName, SideEffectClass,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationPolicy {
    CancelRemainingSteps,
    FinishInFlightOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionPolicy {
    AllStepsMustSucceed,
    AllowDeclaredFallbackCompletion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlanStep {
    pub step_id: String,
    pub capability_id: CapabilityId,
    pub declared_side_effect_class: SideEffectClass,
    pub timeout_budget_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackReference {
    pub step_id: String,
    pub fallback_step_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub execution_plan_id: ExecutionPlanId,
    pub correlation_id: CorrelationId,
    pub originating_request_id: RequestId,
    pub steps: Vec<ExecutionPlanStep>,
    pub referenced_capabilities: Vec<CapabilityId>,
    pub declared_max_step_count: u32,
    pub declared_side_effect_classes: Vec<SideEffectClass>,
    pub fallback_references: Vec<FallbackReference>,
    pub cancellation_policy: CancellationPolicy,
    pub completion_policy: CompletionPolicy,
    pub max_duration_budget_ms: u64,
    pub stable_plan_hash: String,
    pub planned_at_utc: TimestampUtc,
}

impl ExecutionPlan {
    pub fn load_contract_value(value: &Value) -> FaLocalResult<Self> {
        deserialize_contract_value(SchemaName::ExecutionPlan, value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedExecutionPlan {
    pub plan: ExecutionPlan,
    pub stable_plan_hash: String,
}

#[derive(Debug, Default)]
pub struct ExecutionPlanValidator;

impl ExecutionPlanValidator {
    pub fn validate(
        plan: &ExecutionPlan,
        registry: &CapabilityRegistry,
    ) -> Result<ValidatedExecutionPlan, DenialGuard> {
        if plan.steps.is_empty() {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Operation,
                DenialBasis::Contract,
                "execution plan must declare at least one step",
            ));
        }

        if plan.steps.len() > plan.declared_max_step_count as usize {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Operation,
                DenialBasis::Contract,
                "execution plan exceeds declared max step count",
            ));
        }

        let declared_capabilities = plan
            .referenced_capabilities
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let declared_side_effects = plan
            .declared_side_effect_classes
            .iter()
            .map(|side_effect_class| side_effect_class_label(*side_effect_class))
            .collect::<BTreeSet<_>>();
        let mut step_index_by_id = HashMap::new();
        let mut total_timeout_budget_ms = 0_u64;

        for capability_id in &plan.referenced_capabilities {
            let capability = registry.capability_for(*capability_id).ok_or_else(|| {
                deny(
                    DenialReasonClass::CapabilityNotAdmitted,
                    DenialScope::Capability,
                    DenialBasis::Policy,
                    "execution plan references unregistered capability",
                )
            })?;

            if capability.enabled_state == EnabledState::Disabled {
                return Err(deny(
                    DenialReasonClass::DisabledByOperator,
                    DenialScope::Capability,
                    DenialBasis::Policy,
                    "execution plan references disabled capability",
                ));
            }

            if capability.revocation_state == RevocationState::Revoked {
                return Err(deny(
                    DenialReasonClass::CapabilityNotAdmitted,
                    DenialScope::Capability,
                    DenialBasis::Policy,
                    "execution plan references revoked capability",
                ));
            }

            if capability.owner_service != SERVICE_ID {
                return Err(deny(
                    DenialReasonClass::CapabilityNotAdmitted,
                    DenialScope::Capability,
                    DenialBasis::Policy,
                    "execution plan references capability outside fa-local ownership",
                ));
            }
        }

        for (index, step) in plan.steps.iter().enumerate() {
            if !step_index_by_id
                .insert(step.step_id.clone(), index)
                .is_none()
            {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan contains duplicate step ids",
                ));
            }

            if !declared_capabilities.contains(&step.capability_id) {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan step references undeclared capability",
                ));
            }

            if !declared_side_effects
                .contains(side_effect_class_label(step.declared_side_effect_class))
            {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan step uses undeclared side effect class",
                ));
            }

            let capability = registry.capability_for(step.capability_id).ok_or_else(|| {
                deny(
                    DenialReasonClass::CapabilityNotAdmitted,
                    DenialScope::Capability,
                    DenialBasis::Policy,
                    "execution plan references unregistered capability",
                )
            })?;

            if capability.side_effect_class != step.declared_side_effect_class {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan step side effect does not match capability",
                ));
            }

            if step.timeout_budget_ms > capability.timeout_budget_ms {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan step timeout exceeds capability timeout budget",
                ));
            }

            total_timeout_budget_ms = total_timeout_budget_ms
                .checked_add(step.timeout_budget_ms)
                .ok_or_else(|| {
                    deny(
                        DenialReasonClass::ContractInvalid,
                        DenialScope::Operation,
                        DenialBasis::Contract,
                        "execution plan timeout budget overflowed",
                    )
                })?;
        }

        if total_timeout_budget_ms > plan.max_duration_budget_ms {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Operation,
                DenialBasis::Contract,
                "execution plan timeout sum exceeds max duration budget",
            ));
        }

        if !plan.fallback_references.is_empty()
            && plan.completion_policy != CompletionPolicy::AllowDeclaredFallbackCompletion
        {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Operation,
                DenialBasis::Contract,
                "execution plan fallback references require fallback-aware completion policy",
            ));
        }

        for fallback_reference in &plan.fallback_references {
            let Some(step_index) = step_index_by_id.get(&fallback_reference.step_id) else {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan fallback references undeclared primary step",
                ));
            };
            let Some(fallback_index) = step_index_by_id.get(&fallback_reference.fallback_step_id)
            else {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan fallback references undeclared fallback step",
                ));
            };

            if fallback_reference.step_id == fallback_reference.fallback_step_id {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan fallback cannot target the same step",
                ));
            }

            if fallback_index <= step_index {
                return Err(deny(
                    DenialReasonClass::ContractInvalid,
                    DenialScope::Operation,
                    DenialBasis::Contract,
                    "execution plan fallback must target a later declared step",
                ));
            }
        }

        let stable_plan_hash = Self::compute_stable_plan_hash(plan);
        if plan.stable_plan_hash != stable_plan_hash {
            return Err(deny(
                DenialReasonClass::IntegrityFailed,
                DenialScope::Artifact,
                DenialBasis::Contract,
                "execution plan hash does not match canonical plan content",
            ));
        }

        Ok(ValidatedExecutionPlan {
            plan: plan.clone(),
            stable_plan_hash,
        })
    }

    pub fn compute_stable_plan_hash(plan: &ExecutionPlan) -> String {
        let mut referenced_capabilities = plan
            .referenced_capabilities
            .iter()
            .map(|capability_id| capability_id.to_string())
            .collect::<Vec<_>>();
        referenced_capabilities.sort();

        let mut declared_side_effect_classes = plan
            .declared_side_effect_classes
            .iter()
            .map(|side_effect_class| side_effect_class_label(*side_effect_class).to_owned())
            .collect::<Vec<_>>();
        declared_side_effect_classes.sort();

        let mut fallback_references = plan
            .fallback_references
            .iter()
            .map(|fallback_reference| CanonicalFallbackReference {
                step_id: fallback_reference.step_id.clone(),
                fallback_step_id: fallback_reference.fallback_step_id.clone(),
            })
            .collect::<Vec<_>>();
        fallback_references.sort_by(|left, right| {
            left.step_id
                .cmp(&right.step_id)
                .then(left.fallback_step_id.cmp(&right.fallback_step_id))
        });

        let canonical = CanonicalExecutionPlan {
            steps: plan
                .steps
                .iter()
                .map(|step| CanonicalExecutionPlanStep {
                    step_id: step.step_id.clone(),
                    capability_id: step.capability_id.to_string(),
                    declared_side_effect_class: side_effect_class_label(
                        step.declared_side_effect_class,
                    )
                    .to_owned(),
                    timeout_budget_ms: step.timeout_budget_ms,
                })
                .collect(),
            referenced_capabilities,
            declared_max_step_count: plan.declared_max_step_count,
            declared_side_effect_classes,
            fallback_references,
            cancellation_policy: plan.cancellation_policy,
            completion_policy: plan.completion_policy,
            max_duration_budget_ms: plan.max_duration_budget_ms,
        };

        let canonical_json = serde_json::to_vec(&canonical)
            .expect("canonical execution plan serialization must succeed");
        let digest = Sha256::digest(canonical_json);
        let mut hash = String::with_capacity(64);
        for byte in digest {
            write!(&mut hash, "{byte:02x}").expect("writing SHA-256 hex digest must succeed");
        }
        hash
    }
}

#[derive(Debug, Serialize)]
struct CanonicalExecutionPlan {
    steps: Vec<CanonicalExecutionPlanStep>,
    referenced_capabilities: Vec<String>,
    declared_max_step_count: u32,
    declared_side_effect_classes: Vec<String>,
    fallback_references: Vec<CanonicalFallbackReference>,
    cancellation_policy: CancellationPolicy,
    completion_policy: CompletionPolicy,
    max_duration_budget_ms: u64,
}

#[derive(Debug, Serialize)]
struct CanonicalExecutionPlanStep {
    step_id: String,
    capability_id: String,
    declared_side_effect_class: String,
    timeout_budget_ms: u64,
}

#[derive(Debug, Serialize)]
struct CanonicalFallbackReference {
    step_id: String,
    fallback_step_id: String,
}

fn side_effect_class_label(side_effect_class: SideEffectClass) -> &'static str {
    match side_effect_class {
        SideEffectClass::None => "none",
        SideEffectClass::LocalFileWrite => "local_file_write",
        SideEffectClass::LocalDbMutation => "local_db_mutation",
        SideEffectClass::LocalProcessSpawn => "local_process_spawn",
        SideEffectClass::ExternalNetworkDeniedByDefault => "external_network_denied_by_default",
        SideEffectClass::OtherGoverned => "other_governed",
    }
}

#[derive(Debug, Default)]
pub struct ExecutionCoordinator;
