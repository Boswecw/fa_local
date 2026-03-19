use chrono::Utc;

use fa_local::{DenialBasis, DenialReasonClass, DenialScope, deny, ensure, fail_closed};

#[test]
fn ensure_returns_ok_when_condition_holds() {
    assert!(
        ensure(true, || {
            deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Request,
                DenialBasis::Contract,
                "should not be used",
            )
        })
        .is_ok()
    );
}

#[test]
fn ensure_returns_structured_denial_when_condition_fails() {
    let denial = ensure(false, || {
        deny(
            DenialReasonClass::UnknownRequester,
            DenialScope::Request,
            DenialBasis::RuntimeSafety,
            "unknown requester denied",
        )
        .review_available(false)
        .remediable(false)
    })
    .unwrap_err();

    assert_eq!(denial.reason_class, DenialReasonClass::UnknownRequester);
    assert_eq!(denial.scope, DenialScope::Request);
    assert_eq!(denial.basis, DenialBasis::RuntimeSafety);
    assert_eq!(denial.summary, "unknown requester denied");
    assert!(!denial.review_available);
    assert!(!denial.remediable);
}

#[test]
fn fail_closed_rejects_missing_value() {
    let result = fail_closed::<u8>(None, || {
        deny(
            DenialReasonClass::MissingPolicy,
            DenialScope::Artifact,
            DenialBasis::Policy,
            "policy artifact is required",
        )
    });

    let denial = result.unwrap_err();
    assert_eq!(denial.reason_class, DenialReasonClass::MissingPolicy);
    assert_eq!(denial.scope, DenialScope::Artifact);
}

#[test]
fn deny_stamps_current_utc_time() {
    let before = Utc::now();
    let denial = deny(
        DenialReasonClass::UnsupportedRoute,
        DenialScope::Route,
        DenialBasis::RuntimeSafety,
        "route not admitted",
    );
    let after = Utc::now();

    assert!(denial.timestamp_utc >= before);
    assert!(denial.timestamp_utc <= after);
}
