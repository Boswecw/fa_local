use crate::config::SERVICE_ID;
use crate::domain::capabilities::{CapabilityRecord, ReviewClass};
use crate::domain::execution::ExecutionRequest;
use crate::domain::guards::DenialGuard;
use crate::domain::policy::{ApprovalRule, PolicyArtifact, SideEffectRule};
use crate::domain::requester_trust::{RequesterTrustEnvelope, UserIntentBasis};
use crate::domain::routing::{CapabilityDecisionSummary, PolicyReference, RouteDecision};
use crate::domain::shared::{
    ApprovalPosture, DegradedSubtype, DenialBasis, DenialReasonClass, DenialScope, RouteDecisionId,
    SideEffectClass, TimestampUtc, now_utc,
};

#[derive(Debug, Clone)]
pub struct RouteResolutionInput {
    pub request: ExecutionRequest,
    pub requester_trust_outcome: Result<RequesterTrustEnvelope, DenialGuard>,
    pub policy_outcome: Result<PolicyArtifact, DenialGuard>,
    pub capability_admission_outcome: Result<CapabilityRecord, DenialGuard>,
}

#[derive(Debug, Clone)]
pub struct RouteResolutionContext {
    pub route_decision_id: RouteDecisionId,
    pub decided_at_utc: TimestampUtc,
    pub degraded_subtype: Option<DegradedSubtype>,
}

impl RouteResolutionContext {
    pub fn new(route_decision_id: RouteDecisionId, decided_at_utc: TimestampUtc) -> Self {
        Self {
            route_decision_id,
            decided_at_utc,
            degraded_subtype: None,
        }
    }

    pub fn with_degraded_subtype(mut self, degraded_subtype: DegradedSubtype) -> Self {
        self.degraded_subtype = Some(degraded_subtype);
        self
    }
}

impl Default for RouteResolutionContext {
    fn default() -> Self {
        Self {
            route_decision_id: RouteDecisionId::new(),
            decided_at_utc: now_utc(),
            degraded_subtype: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ApprovalPostureResolver;

impl ApprovalPostureResolver {
    pub fn resolve(input: RouteResolutionInput, context: RouteResolutionContext) -> RouteDecision {
        let RouteResolutionInput {
            request,
            requester_trust_outcome,
            policy_outcome,
            capability_admission_outcome,
        } = input;

        let policy_reference = policy_outcome.as_ref().ok().map(|policy| PolicyReference {
            policy_id: policy.policy_id,
            policy_version: policy.policy_version.clone(),
        });

        let mut denials = Vec::new();

        let requester = match requester_trust_outcome {
            Ok(requester) => Some(requester),
            Err(denial) => {
                denials.push(denial);
                None
            }
        };

        let policy = match policy_outcome {
            Ok(policy) => Some(policy),
            Err(denial) => {
                denials.push(denial);
                None
            }
        };

        let capability = match capability_admission_outcome {
            Ok(capability) => Some(capability),
            Err(denial) => {
                denials.push(denial);
                None
            }
        };

        if let (Some(requester), Some(policy), Some(capability)) =
            (requester.as_ref(), policy.as_ref(), capability.as_ref())
        {
            denials.extend(validate_consistency(
                &request,
                requester,
                policy,
                capability,
                context.decided_at_utc,
            ));
        }

        let capability_decision_summary = build_capability_decision_summary(
            &request,
            requester.as_ref(),
            policy.as_ref(),
            capability.as_ref(),
        );

        if !denials.is_empty() {
            return denied_decision(
                &request,
                policy_reference,
                capability_decision_summary,
                denials,
                &context,
            );
        }

        let requester = requester.expect("requester must be present when no denials were recorded");
        let policy = policy.expect("policy must be present when no denials were recorded");
        let capability =
            capability.expect("capability must be present when no denials were recorded");

        let policy_rule = policy
            .capability_rule_for(capability.capability_id)
            .expect("policy capability rule must be present after consistency validation");
        let approval_rule = approval_rule_for(&policy, &requester)
            .expect("policy approval rule must be present after consistency validation");

        let resolved_approval_posture = [
            approval_rule.max_posture,
            policy_rule.required_approval_posture,
            capability.approval_posture,
            review_class_minimum_posture(capability.review_class),
            side_effect_minimum_posture(request.requested_side_effect_class),
            user_intent_minimum_posture(requester.user_intent_basis),
        ]
        .into_iter()
        .min_by_key(|posture| posture_rank(*posture))
        .expect("approval posture factor set must not be empty");

        if resolved_approval_posture == ApprovalPosture::Denied {
            let denial = stamped_denial(
                DenialReasonClass::PolicyDenied,
                DenialScope::Operation,
                DenialBasis::ContractAndPolicy,
                denied_summary(&request),
                context.decided_at_utc,
            );

            return denied_decision(
                &request,
                policy_reference,
                capability_decision_summary,
                vec![denial],
                &context,
            );
        }

        RouteDecision {
            route_decision_id: context.route_decision_id,
            correlation_id: request.correlation_id,
            request_id: request.request_id,
            resolved_approval_posture,
            execution_allowed: execution_allowed_for(resolved_approval_posture),
            denial_guards: Vec::new(),
            review_required: resolved_approval_posture == ApprovalPosture::ReviewRequired,
            explicit_approval_required: resolved_approval_posture
                == ApprovalPosture::ExplicitOperatorApproval,
            policy_reference,
            capability_decision_summary,
            operator_visible_summary: operator_summary(
                resolved_approval_posture,
                request.requested_capability_id.to_string(),
            ),
            degraded_subtype: context.degraded_subtype,
            decided_at_utc: context.decided_at_utc,
        }
    }
}

fn denied_decision(
    request: &ExecutionRequest,
    policy_reference: Option<PolicyReference>,
    capability_decision_summary: CapabilityDecisionSummary,
    denial_guards: Vec<DenialGuard>,
    context: &RouteResolutionContext,
) -> RouteDecision {
    let first_denial_summary = first_denial_summary(&denial_guards);

    RouteDecision {
        route_decision_id: context.route_decision_id,
        correlation_id: request.correlation_id,
        request_id: request.request_id,
        resolved_approval_posture: ApprovalPosture::Denied,
        execution_allowed: false,
        denial_guards,
        review_required: false,
        explicit_approval_required: false,
        policy_reference,
        capability_decision_summary,
        operator_visible_summary: bounded_summary(format!(
            "request denied: {first_denial_summary}"
        )),
        degraded_subtype: context.degraded_subtype,
        decided_at_utc: context.decided_at_utc,
    }
}

fn first_denial_summary(denial_guards: &[DenialGuard]) -> String {
    denial_guards
        .first()
        .map(|denial| denial.summary.clone())
        .unwrap_or_else(|| "request denied".to_owned())
}

fn validate_consistency(
    request: &ExecutionRequest,
    requester: &RequesterTrustEnvelope,
    policy: &PolicyArtifact,
    capability: &CapabilityRecord,
    decided_at_utc: TimestampUtc,
) -> Vec<DenialGuard> {
    let mut denials = Vec::new();

    if request.requester_id != requester.requester_id {
        denials.push(stamped_denial(
            DenialReasonClass::ContractInvalid,
            DenialScope::Request,
            DenialBasis::Contract,
            "requester envelope does not match execution request",
            decided_at_utc,
        ));
    }

    if request.environment_mode != requester.environment_mode {
        denials.push(stamped_denial(
            DenialReasonClass::ContractInvalid,
            DenialScope::Request,
            DenialBasis::Contract,
            "requester environment does not match execution request",
            decided_at_utc,
        ));
    }

    if policy.scope.service_id != SERVICE_ID {
        denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Artifact,
            DenialBasis::Policy,
            "policy service scope does not match fa-local",
            decided_at_utc,
        ));
    }

    if !policy
        .scope
        .environment_modes
        .contains(&request.environment_mode)
        || !policy
            .environment_conditions
            .contains(&request.environment_mode)
    {
        denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Capability,
            DenialBasis::Policy,
            "policy does not admit this environment",
            decided_at_utc,
        ));
    }

    if capability.capability_id != request.requested_capability_id {
        denials.push(stamped_denial(
            DenialReasonClass::CapabilityNotAdmitted,
            DenialScope::Capability,
            DenialBasis::Policy,
            "capability decision does not match execution request",
            decided_at_utc,
        ));
    }

    if capability.owner_service != SERVICE_ID {
        denials.push(stamped_denial(
            DenialReasonClass::CapabilityNotAdmitted,
            DenialScope::Capability,
            DenialBasis::Policy,
            "capability owner does not match fa-local",
            decided_at_utc,
        ));
    }

    if capability.side_effect_class != request.requested_side_effect_class {
        denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Capability,
            DenialBasis::ContractAndPolicy,
            "policy/capability mismatch: requested side effect does not match capability",
            decided_at_utc,
        ));
    }

    if policy
        .capability_rule_for(capability.capability_id)
        .is_none()
    {
        denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Capability,
            DenialBasis::ContractAndPolicy,
            "policy/capability mismatch: capability missing from policy",
            decided_at_utc,
        ));
    }

    if approval_rule_for(policy, requester).is_none() {
        denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Request,
            DenialBasis::Policy,
            "policy lacks requester approval rule",
            decided_at_utc,
        ));
    }

    match side_effect_rule_for(policy, request.requested_side_effect_class) {
        Some(rule) if rule.allowed => {}
        _ => denials.push(stamped_denial(
            DenialReasonClass::PolicyDenied,
            DenialScope::Operation,
            DenialBasis::Policy,
            "policy denies requested side effect class",
            decided_at_utc,
        )),
    }

    denials
}

fn build_capability_decision_summary(
    request: &ExecutionRequest,
    requester: Option<&RequesterTrustEnvelope>,
    policy: Option<&PolicyArtifact>,
    capability: Option<&CapabilityRecord>,
) -> CapabilityDecisionSummary {
    let requester_max_approval_posture = requester.and_then(|requester| {
        policy.and_then(|policy| {
            approval_rule_for(policy, requester).map(|approval_rule| approval_rule.max_posture)
        })
    });

    let policy_required_approval_posture = policy.and_then(|policy| {
        policy
            .capability_rule_for(request.requested_capability_id)
            .map(|rule| rule.required_approval_posture)
    });

    CapabilityDecisionSummary {
        requested_capability_id: request.requested_capability_id,
        capability_admitted: capability.is_some(),
        capability_owner_service: capability.map(|capability| capability.owner_service.clone()),
        requested_side_effect_class: request.requested_side_effect_class,
        capability_approval_posture: capability.map(|capability| capability.approval_posture),
        policy_required_approval_posture,
        requester_max_approval_posture,
        side_effect_minimum_approval_posture: side_effect_minimum_posture(
            request.requested_side_effect_class,
        ),
        review_class: capability.map(|capability| capability.review_class),
    }
}

fn approval_rule_for<'a>(
    policy: &'a PolicyArtifact,
    requester: &RequesterTrustEnvelope,
) -> Option<&'a ApprovalRule> {
    policy
        .approval_rules
        .iter()
        .find(|rule| rule.requester_class == requester.requester_class)
}

fn side_effect_rule_for(
    policy: &PolicyArtifact,
    side_effect_class: SideEffectClass,
) -> Option<&SideEffectRule> {
    policy
        .side_effect_rules
        .iter()
        .find(|rule| rule.side_effect_class == side_effect_class)
}

fn side_effect_minimum_posture(side_effect_class: SideEffectClass) -> ApprovalPosture {
    match side_effect_class {
        SideEffectClass::None => ApprovalPosture::ExecuteAllowed,
        SideEffectClass::LocalFileWrite | SideEffectClass::LocalDbMutation => {
            ApprovalPosture::PolicyPreapproved
        }
        SideEffectClass::LocalProcessSpawn | SideEffectClass::OtherGoverned => {
            ApprovalPosture::ExplicitOperatorApproval
        }
        SideEffectClass::ExternalNetworkDeniedByDefault => ApprovalPosture::Denied,
    }
}

fn review_class_minimum_posture(review_class: ReviewClass) -> ApprovalPosture {
    match review_class {
        ReviewClass::None => ApprovalPosture::ExecuteAllowed,
        ReviewClass::Operator => ApprovalPosture::ExplicitOperatorApproval,
    }
}

fn user_intent_minimum_posture(user_intent_basis: Option<UserIntentBasis>) -> ApprovalPosture {
    match user_intent_basis {
        Some(UserIntentBasis::ExplicitUserAction | UserIntentBasis::OperatorApproval) => {
            ApprovalPosture::ExecuteAllowed
        }
        None => ApprovalPosture::ReviewRequired,
    }
}

fn posture_rank(posture: ApprovalPosture) -> u8 {
    match posture {
        ApprovalPosture::Denied => 0,
        ApprovalPosture::ReviewRequired => 1,
        ApprovalPosture::ExplicitOperatorApproval => 2,
        ApprovalPosture::PolicyPreapproved => 3,
        ApprovalPosture::ExecuteAllowed => 4,
    }
}

fn execution_allowed_for(posture: ApprovalPosture) -> bool {
    matches!(
        posture,
        ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed
    )
}

fn denied_summary(request: &ExecutionRequest) -> String {
    match request.requested_side_effect_class {
        SideEffectClass::ExternalNetworkDeniedByDefault => {
            "requested side effect is denied by bounded doctrine".to_owned()
        }
        _ => "approval posture resolution denied request".to_owned(),
    }
}

fn operator_summary(posture: ApprovalPosture, capability_id: String) -> String {
    let summary = match posture {
        ApprovalPosture::Denied => format!("request denied for capability {capability_id}"),
        ApprovalPosture::ReviewRequired => {
            format!("request requires review for capability {capability_id}")
        }
        ApprovalPosture::ExplicitOperatorApproval => {
            format!("request requires explicit operator approval for capability {capability_id}")
        }
        ApprovalPosture::PolicyPreapproved => {
            format!("request is policy preapproved for capability {capability_id}")
        }
        ApprovalPosture::ExecuteAllowed => {
            format!("request is execute allowed for capability {capability_id}")
        }
    };

    bounded_summary(summary)
}

fn stamped_denial(
    reason_class: DenialReasonClass,
    scope: DenialScope,
    basis: DenialBasis,
    summary: impl Into<String>,
    timestamp_utc: TimestampUtc,
) -> DenialGuard {
    DenialGuard {
        reason_class,
        scope,
        basis,
        remediable: false,
        review_available: false,
        summary: summary.into(),
        timestamp_utc,
    }
}

fn bounded_summary(summary: String) -> String {
    const MAX_SUMMARY_CHARS: usize = 160;

    let mut bounded = summary.chars().take(MAX_SUMMARY_CHARS).collect::<String>();
    if summary.chars().count() > MAX_SUMMARY_CHARS {
        bounded.truncate(MAX_SUMMARY_CHARS.saturating_sub(3));
        bounded.push_str("...");
    }
    bounded
}
