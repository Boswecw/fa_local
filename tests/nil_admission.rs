//! NIL narrative-commit admission tests.
//!
//! Drives the real admission gate (`AdmissionService::admit`) over the Writer
//! Authority Mode policy + narrative-commit capability built by
//! `integrations::authorforge_nil`, proving FA Local enforces NIL's approval
//! tiers for a canon current-state commit.

use chrono::{DateTime, Utc};
use fa_local::AdmissionService;
use fa_local::integrations::authorforge_nil::{
    NarrativeAuthorityClass, WriterAuthorityMode, build_admission_inputs, narrative_commit_posture,
};

fn now() -> DateTime<Utc> {
    "2030-01-01T00:00:00Z".parse().expect("valid timestamp")
}

fn admit(mode: WriterAuthorityMode, class: NarrativeAuthorityClass) -> fa_local::AdmissionOutcome {
    AdmissionService
        .admit(
            &build_admission_inputs(mode, class, now()),
            fa_local::AdmissionContext::new(fa_local::EnvironmentMode::Prod, now()),
        )
        .expect("admit ok")
}

/// Every (mode, authority-class) resolves to exactly the posture the Writer
/// Authority Mode policy assigns — the gate enforces the NIL tier mapping.
#[test]
fn gate_enforces_the_nil_tier_for_every_mode_and_class() {
    use NarrativeAuthorityClass::*;
    use WriterAuthorityMode::*;
    for mode in [
        StrictWriterApproval,
        AssistedMetadataAuto,
        ExperimentalLowRiskAuto,
    ] {
        for class in [
            DerivedMetadata,
            CandidateAuthority,
            AcceptedAuthority,
            ManuscriptText,
        ] {
            let outcome = admit(mode, class);
            assert_eq!(
                outcome.route_decision.resolved_approval_posture,
                narrative_commit_posture(mode, class),
                "mode={mode:?} class={class:?}"
            );
        }
    }
}

#[test]
fn derived_metadata_auto_commits_only_outside_strict_mode() {
    use NarrativeAuthorityClass::DerivedMetadata;
    use WriterAuthorityMode::*;

    // assisted / experimental -> Tier A (PolicyPreapproved, executable)
    let assisted = admit(AssistedMetadataAuto, DerivedMetadata);
    assert!(assisted.route_decision.execution_allowed);
    assert!(assisted.review_package.is_none());

    // strict -> Tier B review (not executable, review package emitted)
    let strict = admit(StrictWriterApproval, DerivedMetadata);
    assert!(!strict.route_decision.execution_allowed);
    assert!(strict.route_decision.review_required);
    assert!(strict.review_package.is_some());
}

#[test]
fn candidate_authority_is_review_in_all_modes() {
    use WriterAuthorityMode::*;
    for mode in [
        StrictWriterApproval,
        AssistedMetadataAuto,
        ExperimentalLowRiskAuto,
    ] {
        let outcome = admit(mode, NarrativeAuthorityClass::CandidateAuthority);
        assert!(outcome.route_decision.review_required, "mode={mode:?}");
        assert!(!outcome.route_decision.execution_allowed);
        assert!(outcome.review_package.is_some());
    }
}

#[test]
fn accepted_authority_requires_explicit_writer_approval() {
    let outcome = admit(
        WriterAuthorityMode::AssistedMetadataAuto,
        NarrativeAuthorityClass::AcceptedAuthority,
    );
    assert!(outcome.route_decision.explicit_approval_required);
    assert!(!outcome.route_decision.execution_allowed);
}

#[test]
fn manuscript_text_is_never_committable_in_any_mode() {
    use WriterAuthorityMode::*;
    for mode in [
        StrictWriterApproval,
        AssistedMetadataAuto,
        ExperimentalLowRiskAuto,
    ] {
        let outcome = admit(mode, NarrativeAuthorityClass::ManuscriptText);
        assert!(!outcome.route_decision.execution_allowed, "mode={mode:?}");
        assert!(!outcome.route_decision.denial_guards.is_empty());
        assert!(outcome.review_package.is_none());
    }
}
