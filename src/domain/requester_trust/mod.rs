use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::guards::{DenialGuard, deny};
use crate::domain::shared::{
    DenialBasis, DenialReasonClass, DenialScope, EnvironmentMode, RequesterClass, RequesterId,
    SchemaName, TimestampUtc, deserialize_contract_value,
};
use crate::errors::FaLocalResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppContext {
    pub app_id: String,
    pub app_version: String,
    pub installation_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustBasis {
    SignedLocalSurface,
    InternalServiceToken,
    ReviewMediation,
    DevelopmentFixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustBasisProvenance {
    SignedManifest,
    OperatorConfiguration,
    RuntimeAttestation,
    TestFixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserIntentBasis {
    ExplicitUserAction,
    OperatorApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequesterTrustEnvelope {
    pub requester_id: RequesterId,
    pub requester_class: RequesterClass,
    pub app_context: AppContext,
    pub environment_mode: EnvironmentMode,
    pub trust_basis: TrustBasis,
    pub trust_basis_provenance: TrustBasisProvenance,
    pub user_intent_basis: Option<UserIntentBasis>,
    pub request_nonce_or_token: String,
    pub issued_at: TimestampUtc,
    pub expires_at: TimestampUtc,
}

#[derive(Debug, Clone)]
pub struct TrustEvaluationContext {
    pub expected_environment: EnvironmentMode,
    pub now: TimestampUtc,
}

#[derive(Debug, Default)]
pub struct RequesterTrustEngine;

impl RequesterTrustEngine {
    pub fn load_contract_value(value: &Value) -> FaLocalResult<RequesterTrustEnvelope> {
        deserialize_contract_value(SchemaName::RequesterTrust, value)
    }

    pub fn load_and_evaluate(
        value: &Value,
        context: &TrustEvaluationContext,
    ) -> Result<RequesterTrustEnvelope, DenialGuard> {
        if !value.is_object() {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Request,
                DenialBasis::Contract,
                "malformed requester envelope: root must be an object",
            ));
        }

        if value.get("trust_basis").is_none() {
            return Err(deny(
                DenialReasonClass::UntrustedRequester,
                DenialScope::Request,
                DenialBasis::Contract,
                "missing trust basis",
            ));
        }

        let envelope = Self::load_contract_value(value).map_err(|error| {
            deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Request,
                DenialBasis::Contract,
                format!("malformed requester envelope: {error}"),
            )
        })?;

        Self::evaluate(&envelope, context)?;
        Ok(envelope)
    }

    pub fn evaluate(
        envelope: &RequesterTrustEnvelope,
        context: &TrustEvaluationContext,
    ) -> Result<(), DenialGuard> {
        if envelope.requester_class == RequesterClass::UntrustedUnknown {
            return Err(deny(
                DenialReasonClass::UnknownRequester,
                DenialScope::Request,
                DenialBasis::RuntimeSafety,
                "unknown requester denied",
            ));
        }

        if envelope.environment_mode != context.expected_environment {
            return Err(deny(
                DenialReasonClass::ContractInvalid,
                DenialScope::Request,
                DenialBasis::Contract,
                "environment mismatch",
            ));
        }

        if envelope.issued_at >= envelope.expires_at {
            return Err(deny(
                DenialReasonClass::UntrustedRequester,
                DenialScope::Request,
                DenialBasis::RuntimeSafety,
                "request token or nonce has an invalid time window",
            ));
        }

        if !is_valid_nonce_or_token(&envelope.request_nonce_or_token) {
            return Err(deny(
                DenialReasonClass::UntrustedRequester,
                DenialScope::Request,
                DenialBasis::RuntimeSafety,
                "request token or nonce is invalid",
            ));
        }

        if context.now >= envelope.expires_at {
            return Err(deny(
                DenialReasonClass::UntrustedRequester,
                DenialScope::Request,
                DenialBasis::RuntimeSafety,
                "request token or nonce is expired",
            ));
        }

        Ok(())
    }
}

fn is_valid_nonce_or_token(value: &str) -> bool {
    let len = value.len();
    if !(16..=128).contains(&len) {
        return false;
    }

    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
