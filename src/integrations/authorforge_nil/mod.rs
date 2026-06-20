//! AuthorForge NIL — Writer Authority Mode policy + narrative-commit capability.
//!
//! FA Local producer obligation from the NIL v2 support-service delegation
//! amendment. NIL's Writer Authority Mode / approval tiering is *expressed as FA
//! Local inputs* and enforced by the existing admission gate — nothing in the
//! gate changes. A canon current-state commit is a governed execution request
//! with side-effect class `LocalDbMutation`; the project's Writer Authority Mode
//! and the candidate's authority class select the required approval posture, so
//! `AdmissionService::admit` returns:
//!
//! | NIL tier | FA Local posture        | meaning                              |
//! |----------|-------------------------|--------------------------------------|
//! | A_auto   | `PolicyPreapproved`     | auto-commit (audited, reversible)    |
//! | B_review | `ReviewRequired`        | non-blocking review (review package) |
//! | (writer) | `ExplicitOperatorApproval` | explicit writer approval          |
//! | C_block  | `Denied`                | blocked / not a committable capability |
//!
//! Non-negotiable: `manuscript_text` is never a committable capability (it is
//! absent from the registry), so the gate denies it in every mode.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::AdmissionInputs;
use crate::domain::shared::{ApprovalPosture, TimestampUtc};

/// Per-project Writer Authority Mode (mirrors NIL `WriterAuthorityMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterAuthorityMode {
    StrictWriterApproval,
    AssistedMetadataAuto,
    ExperimentalLowRiskAuto,
}

/// What kind of state a commit would change (mirrors NIL `AuthorityClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrativeAuthorityClass {
    DerivedMetadata,
    CandidateAuthority,
    AcceptedAuthority,
    ManuscriptText,
}

/// Stable capability id for a narrative commit of the given authority class.
/// `ManuscriptText` has no capability — it is never committable.
pub fn narrative_commit_capability_id(class: NarrativeAuthorityClass) -> Option<Uuid> {
    let id = match class {
        NarrativeAuthorityClass::DerivedMetadata => "a1100000-0000-4000-8000-000000000001",
        NarrativeAuthorityClass::CandidateAuthority => "a1100000-0000-4000-8000-000000000002",
        NarrativeAuthorityClass::AcceptedAuthority => "a1100000-0000-4000-8000-000000000003",
        NarrativeAuthorityClass::ManuscriptText => return None,
    };
    Some(Uuid::parse_str(id).expect("valid capability uuid"))
}

/// The Writer Authority Mode policy decision: the approval posture required to
/// commit a candidate of `class` under `mode`. This is the heart of the NIL ->
/// FA Local mapping; the admission gate enforces it.
pub fn narrative_commit_posture(
    mode: WriterAuthorityMode,
    class: NarrativeAuthorityClass,
) -> ApprovalPosture {
    use NarrativeAuthorityClass::*;
    use WriterAuthorityMode::*;
    match class {
        // manuscript_text never auto-commits and is not a committable capability.
        ManuscriptText => ApprovalPosture::Denied,
        // Rebuildable derived metadata may auto-commit once the writer has opted
        // out of strict approval; strict mode still reviews it.
        DerivedMetadata => match mode {
            StrictWriterApproval => ApprovalPosture::ReviewRequired,
            AssistedMetadataAuto | ExperimentalLowRiskAuto => ApprovalPosture::PolicyPreapproved,
        },
        // Proposed authority-bearing signals wait calmly for review in all modes.
        CandidateAuthority => ApprovalPosture::ReviewRequired,
        // Accepting authority state requires an explicit writer decision.
        AcceptedAuthority => ApprovalPosture::ExplicitOperatorApproval,
    }
}

fn iso(ts: TimestampUtc) -> String {
    ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Build the four contract envelopes for one narrative-commit admission: a
/// requester-trust envelope, the Writer Authority Mode policy artifact, the
/// narrative-commit capability registry, and the commit execution request.
/// Feeding these to `AdmissionService::admit` yields the NIL tier for
/// `(mode, class)`. For `ManuscriptText` the registry is empty, so the gate
/// denies the (unregistered) commit capability.
pub fn build_admission_inputs(
    mode: WriterAuthorityMode,
    class: NarrativeAuthorityClass,
    now: TimestampUtc,
) -> AdmissionInputs {
    let requester_id = "b2200000-0000-4000-8000-000000000001";
    let issued = iso(now - chrono::Duration::hours(1));
    let expires = iso(now + chrono::Duration::hours(1));
    let posture = narrative_commit_posture(mode, class);
    let cap = narrative_commit_capability_id(class);

    let requester_trust = json!({
        "requester_id": requester_id,
        "requester_class": "trusted_app_surface",
        "app_context": {
            "app_id": "forge-author",
            "app_version": "2.0.0",
            "installation_id": "b2200000-0000-4000-8000-0000000000aa"
        },
        "environment_mode": "prod",
        "trust_basis": "signed_local_surface",
        "trust_basis_provenance": "signed_manifest",
        "user_intent_basis": "explicit_user_action",
        "request_nonce_or_token": "nil-commit-token",
        "issued_at": issued,
        "expires_at": expires
    });

    // The Writer Authority Mode policy artifact: one rule per narrative-commit
    // capability, requiring the posture this mode assigns to that authority class.
    let capability_rules: Vec<Value> = [
        NarrativeAuthorityClass::DerivedMetadata,
        NarrativeAuthorityClass::CandidateAuthority,
        NarrativeAuthorityClass::AcceptedAuthority,
    ]
    .into_iter()
    .map(|c| {
        json!({
            "capability_id": narrative_commit_capability_id(c).unwrap(),
            "allowed": true,
            "allowed_requester_classes": ["trusted_app_surface"],
            "allowed_side_effect_classes": ["local_db_mutation"],
            "required_approval_posture": narrative_commit_posture(mode, c)
        })
    })
    .collect();

    let policy = json!({
        "policy_id": "c3300000-0000-4000-8000-000000000001",
        "policy_version": "1.0.0",
        "scope": { "service_id": "fa-local", "environment_modes": ["prod"] },
        "capability_rules": capability_rules,
        "side_effect_rules": [{ "side_effect_class": "local_db_mutation", "allowed": true }],
        "approval_rules": [{ "requester_class": "trusted_app_surface", "max_posture": "execute_allowed" }],
        "environment_conditions": ["prod"],
        "dependency_readiness_conditions": ["all_dependencies_ready"],
        "failure_behavior": "deny",
        "policy_provenance": { "source_kind": "local_governed_file", "issued_at": issued }
    });

    // The narrative-commit capability registry. manuscript_text is intentionally
    // absent — it is never a committable capability.
    let capabilities: Vec<Value> = cap
        .map(|cap_id| {
            vec![json!({
                "capability_id": cap_id,
                "owner_service": "fa-local",
                "capability_type": "local_db_mutation",
                "side_effect_class": "local_db_mutation",
                "approval_posture": posture,
                "allowed_requester_classes": ["trusted_app_surface"],
                "timeout_budget_ms": 1000,
                "retry_budget": 0,
                "max_duration_budget_ms": 5000,
                "enabled_state": "enabled",
                "review_class": "none",
                "provenance": { "source_kind": "registry_file", "issued_at": issued },
                "revocation_state": "active",
                "version_range": ">=1.0.0"
            })]
        })
        .unwrap_or_default();

    let capability_registry = json!({
        "registry_version": "1.0.0",
        "capabilities": capabilities
    });

    // The execution request: a canon current-state commit (LocalDbMutation).
    let requested_capability = cap
        .map(|c| c.to_string())
        // manuscript_text: reference an id the (empty) registry does not contain.
        .unwrap_or_else(|| "a1100000-0000-4000-8000-0000000000ff".to_string());
    let execution_request = json!({
        "request_id": Uuid::new_v4(),
        "correlation_id": Uuid::new_v4(),
        "requester_id": requester_id,
        "environment_mode": "prod",
        "requested_capability_id": requested_capability,
        "requested_side_effect_class": "local_db_mutation",
        "intent": "execute_capability",
        "intent_summary": "commit canon candidate to nil_canon_current",
        "requested_at": iso(now)
    });

    AdmissionInputs {
        requester_trust,
        policy,
        capability_registry,
        execution_request,
    }
}
