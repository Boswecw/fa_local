use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::adapters::execution_delivery::{
    AdapterDeliveryRequest, AdapterDeliveryResult, ExternalRouteDeliveryAdapter,
};
use crate::domain::shared::{ApprovalPosture, CapabilityId, RequestId};

const REFUSAL_MARKER_FILE_NAME: &str = ".fa_local_refuse_delivery";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFileWriteAdapterConfig {
    pub supported_capability_id: CapabilityId,
    pub delivery_root: PathBuf,
}

impl LocalFileWriteAdapterConfig {
    pub fn new(supported_capability_id: CapabilityId, delivery_root: PathBuf) -> Self {
        Self {
            supported_capability_id,
            delivery_root,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalFileWriteDeliveryAdapter {
    config: LocalFileWriteAdapterConfig,
}

impl LocalFileWriteDeliveryAdapter {
    pub fn new(config: LocalFileWriteAdapterConfig) -> Self {
        Self { config }
    }

    pub fn delivery_root(&self) -> &Path {
        &self.config.delivery_root
    }

    pub fn refusal_marker_path(&self) -> PathBuf {
        self.delivery_root().join(REFUSAL_MARKER_FILE_NAME)
    }

    pub fn receipt_path_for(&self, request_id: RequestId, stable_plan_hash: &str) -> PathBuf {
        let short_hash = stable_plan_hash.chars().take(12).collect::<String>();
        self.delivery_root().join(format!(
            "fa_local_delivery_{request_id}_{short_hash}.receipt"
        ))
    }

    fn first_declared_step<'a>(&self, request: &'a AdapterDeliveryRequest) -> Option<&'a str> {
        request.declared_step_ids.first().map(String::as_str)
    }

    fn validate_request(&self, request: &AdapterDeliveryRequest) -> Result<(), &'static str> {
        if !matches!(
            request.resolved_approval_posture,
            ApprovalPosture::PolicyPreapproved | ApprovalPosture::ExecuteAllowed
        ) {
            return Err("local file write adapter requires admitted posture");
        }

        if request.requested_capability_id != self.config.supported_capability_id {
            return Err("local file write adapter capability mismatch");
        }

        if request.declared_capability_ids.len() != 1
            || request.declared_capability_ids[0] != self.config.supported_capability_id
        {
            return Err("local file write adapter requires one declared capability");
        }

        if request.declared_step_ids.is_empty() {
            return Err("local file write adapter requires declared steps");
        }

        if !request.declared_fallback_references.is_empty() {
            return Err("local file write adapter does not support fallbacks");
        }

        Ok(())
    }

    fn receipt_contents(&self, request: &AdapterDeliveryRequest) -> String {
        format!(
            "adapter_id={}\nroute_decision_id={}\ncorrelation_id={}\nrequest_id={}\nexecution_plan_id={}\nstable_plan_hash={}\nrequested_capability_id={}\ndeclared_step_ids={}\n",
            self.adapter_id(),
            request.route_decision_id,
            request.correlation_id,
            request.request_id,
            request.execution_plan_id,
            request.stable_plan_hash,
            request.requested_capability_id,
            request.declared_step_ids.join(","),
        )
    }
}

impl ExternalRouteDeliveryAdapter for LocalFileWriteDeliveryAdapter {
    fn adapter_id(&self) -> &'static str {
        "local-file-write-delivery"
    }

    fn deliver_route(&self, request: &AdapterDeliveryRequest) -> AdapterDeliveryResult {
        if let Err(summary) = self.validate_request(request) {
            return AdapterDeliveryResult::Unsupported {
                summary: summary.to_owned(),
            };
        }

        if !self.delivery_root().is_dir() {
            return AdapterDeliveryResult::DependencyUnavailable {
                summary: "local file write delivery root is unavailable".to_owned(),
            };
        }

        let first_step_id = self
            .first_declared_step(request)
            .expect("validated request always has at least one declared step")
            .to_owned();

        if self.refusal_marker_path().exists() {
            return AdapterDeliveryResult::FailedAtDeclaredStep {
                step_id: first_step_id,
                failure_summary: "local file write delivery refused by operator marker".to_owned(),
            };
        }

        let receipt_path = self.receipt_path_for(request.request_id, &request.stable_plan_hash);
        match fs::write(receipt_path, self.receipt_contents(request)) {
            Ok(()) => AdapterDeliveryResult::DeliveredAllSteps,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                AdapterDeliveryResult::DependencyUnavailable {
                    summary: "local file write delivery root is unavailable".to_owned(),
                }
            }
            Err(_) => AdapterDeliveryResult::FailedAtDeclaredStep {
                step_id: first_step_id,
                failure_summary: "local file write receipt write failed".to_owned(),
            },
        }
    }
}
